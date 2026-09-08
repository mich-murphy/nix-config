use domain::ports::{PortError, ProcessIdentity};
use std::{
    collections::BTreeMap,
    io::Write,
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRequest {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub stdin: Option<String>,
    pub env: BTreeMap<String, String>,
    pub remove_env: Vec<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    Running,
    Stopped,
    Terminated,
}

pub trait RunningProcess {
    fn identity(&self) -> ProcessIdentity;
    fn finish(self: Box<Self>) -> Result<ProcessOutput, PortError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessOutput {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait Process {
    fn run(&self, request: &ProcessRequest) -> Result<ProcessOutput, PortError>;

    fn start(&self, _request: &ProcessRequest) -> Result<Box<dyn RunningProcess>, PortError> {
        Err(PortError("owned process start is unsupported".into()))
    }

    fn recover(&self, _identity: ProcessIdentity, _terminate: bool) -> Result<Recovery, PortError> {
        Err(PortError("owned process recovery is unsupported".into()))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SystemProcess;

impl Process for SystemProcess {
    fn run(&self, request: &ProcessRequest) -> Result<ProcessOutput, PortError> {
        self.start(request)?.finish()
    }

    fn start(&self, request: &ProcessRequest) -> Result<Box<dyn RunningProcess>, PortError> {
        let mut command = gated_command(request);
        let child = command
            .spawn()
            .map_err(|error| PortError(error.to_string()))?;
        let identity = identity(child.id())?;
        Ok(Box::new(OwnedChild {
            child: Some(child),
            identity,
            input: request.stdin.clone(),
            timeout: Duration::from_secs(request.timeout_seconds),
        }))
    }

    fn recover(&self, identity: ProcessIdentity, terminate: bool) -> Result<Recovery, PortError> {
        if !identity_matches(identity)? {
            return Ok(Recovery::Stopped);
        }
        if !terminate {
            return Ok(Recovery::Running);
        }
        terminate_group(identity.group)?;
        Ok(Recovery::Terminated)
    }
}

struct OwnedChild {
    child: Option<Child>,
    identity: ProcessIdentity,
    input: Option<String>,
    timeout: Duration,
}

impl RunningProcess for OwnedChild {
    fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    fn finish(mut self: Box<Self>) -> Result<ProcessOutput, PortError> {
        let mut child = self
            .child
            .take()
            .ok_or_else(|| PortError("owned process already consumed".into()))?;
        if let Err(error) = release(&mut child, self.input.as_deref()) {
            let _ = terminate_group(self.identity.group);
            let _ = child.wait();
            return Err(error);
        }
        wait(child, self.identity, self.timeout)
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = terminate_group(self.identity.group);
            let _ = child.wait();
        }
    }
}

fn gated_command(request: &ProcessRequest) -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg("IFS= read -r gate; exec \"$@\"")
        .arg("process-gate")
        .arg(&request.program)
        .args(&request.args)
        .current_dir(&request.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    for (name, value) in &request.env {
        command.env(name, value);
    }
    for name in &request.remove_env {
        command.env_remove(name);
    }
    command
}

fn release(child: &mut Child, input: Option<&str>) -> Result<(), PortError> {
    let mut pipe = child
        .stdin
        .take()
        .ok_or_else(|| PortError("child gate unavailable".into()))?;
    pipe.write_all(b"run\n")
        .and_then(|()| input.map_or(Ok(()), |value| pipe.write_all(value.as_bytes())))
        .map_err(|error| PortError(error.to_string()))
}

fn wait(
    mut child: Child,
    identity: ProcessIdentity,
    timeout: Duration,
) -> Result<ProcessOutput, PortError> {
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|error| PortError(error.to_string()))?
            .is_some()
        {
            break;
        }
        if started.elapsed() >= timeout {
            terminate_group(identity.group)?;
            let _ = child.wait();
            return Err(PortError(format!(
                "process timed out after {}s",
                timeout.as_secs()
            )));
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child
        .wait_with_output()
        .map_err(|error| PortError(error.to_string()))?;
    Ok(ProcessOutput {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn identity(pid: u32) -> Result<ProcessIdentity, PortError> {
    let (group, start_ticks) = process_stat(pid)?;
    Ok(ProcessIdentity {
        pid,
        start_ticks,
        group,
    })
}

fn identity_matches(identity: ProcessIdentity) -> Result<bool, PortError> {
    match process_stat(identity.pid) {
        Ok((group, start)) => Ok(group == identity.group && start == identity.start_ticks),
        Err(error) if error.0.contains("No such file") => Ok(false),
        Err(error) => Err(error),
    }
}

fn process_stat(pid: u32) -> Result<(u32, u64), PortError> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|error| PortError(error.to_string()))?;
    let rest = text
        .rsplit_once(") ")
        .map(|(_, rest)| rest)
        .ok_or_else(|| PortError("invalid process stat".into()))?;
    let fields = rest.split_whitespace().collect::<Vec<_>>();
    let group = fields
        .get(2)
        .ok_or_else(|| PortError("process group missing".into()))?
        .parse::<u32>()
        .map_err(|error| PortError(error.to_string()))?;
    let start = fields
        .get(19)
        .ok_or_else(|| PortError("process start identity missing".into()))?
        .parse::<u64>()
        .map_err(|error| PortError(error.to_string()))?;
    Ok((group, start))
}

fn terminate_group(group: u32) -> Result<(), PortError> {
    let status = Command::new("kill")
        .args(["-TERM", &format!("-{group}")])
        .status()
        .map_err(|error| PortError(error.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(PortError("failed to terminate owned process group".into()))
    }
}

pub fn success(output: ProcessOutput) -> Result<String, PortError> {
    if output.code == Some(0) {
        Ok(output.stdout)
    } else {
        Err(PortError(format!(
            "process failed with {:?}: {}",
            output.code, output.stderr
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(program: &str, args: &[&str], timeout_seconds: u64) -> ProcessRequest {
        ProcessRequest {
            program: program.into(),
            args: args.iter().map(ToString::to_string).collect(),
            cwd: PathBuf::from("/"),
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
            timeout_seconds,
        }
    }

    #[test]
    fn timeout_stops_process_group() -> Result<(), PortError> {
        let error = match SystemProcess.run(&request("sleep", &["5"], 0)) {
            Ok(_) => return Err(PortError("zero timeout did not stop the process".into())),
            Err(error) => error,
        };
        assert!(error.0.contains("timed out"));
        Ok(())
    }

    #[test]
    fn recovery_checks_start_identity() -> Result<(), PortError> {
        let process = SystemProcess.start(&request("sleep", &["5"], 5))?;
        let mut wrong = process.identity();
        wrong.start_ticks = wrong.start_ticks.saturating_add(1);
        assert_eq!(SystemProcess.recover(wrong, true)?, Recovery::Stopped);
        assert_eq!(
            SystemProcess.recover(process.identity(), false)?,
            Recovery::Running
        );
        assert_eq!(
            SystemProcess.recover(process.identity(), true)?,
            Recovery::Terminated
        );
        Ok(())
    }
}
