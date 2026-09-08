use domain::ports::PortError;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    process::{Command, Stdio},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRequest {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub stdin: Option<String>,
    pub env: BTreeMap<String, String>,
    pub remove_env: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait Process {
    fn run(&self, request: &ProcessRequest) -> Result<ProcessOutput, PortError>;
}

#[derive(Debug, Clone, Copy)]
pub struct SystemProcess;

impl Process for SystemProcess {
    fn run(&self, request: &ProcessRequest) -> Result<ProcessOutput, PortError> {
        let mut command = Command::new(&request.program);
        command.args(&request.args).current_dir(&request.cwd);
        for (name, value) in &request.env {
            command.env(name, value);
        }
        for name in &request.remove_env {
            command.env_remove(name);
        }
        if request.stdin.is_some() {
            command.stdin(Stdio::piped());
        }
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| PortError(error.to_string()))?;
        if let Some(input) = &request.stdin {
            write_stdin(&mut child, input)?;
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
}

fn write_stdin(child: &mut std::process::Child, input: &str) -> Result<(), PortError> {
    use std::io::Write;
    let mut pipe = child
        .stdin
        .take()
        .ok_or_else(|| PortError("child stdin unavailable".into()))?;
    pipe.write_all(input.as_bytes())
        .map_err(|error| PortError(error.to_string()))
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
