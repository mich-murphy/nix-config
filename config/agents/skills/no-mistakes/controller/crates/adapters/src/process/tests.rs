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
fn large_output_does_not_deadlock() -> Result<(), PortError> {
    let output = SystemProcess.run(&request(
        "sh",
        &["-c", "head -c 300000 /dev/zero | tr '\\0' a"],
        5,
    ))?;
    assert_eq!(output.code, Some(0));
    assert_eq!(output.stdout.len(), 300_000);
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

/// A process started but dropped before it ever passes the gate (the
/// implementer never released it with `finish`) is not left running:
/// `Drop` terminates the still-gated group, so a later `recover`
/// finds nothing to settle and reports `Stopped` rather than treating
/// it as still executing.
#[test]
fn pre_gate_crash_settles_failed() -> Result<(), PortError> {
    let identity = {
        let process = SystemProcess.start(&request("sleep", &["5"], 5))?;
        process.identity()
    };
    assert_eq!(SystemProcess.recover(identity, false)?, Recovery::Stopped);
    Ok(())
}

/// A process that spawns a background descendant (`sleep 30 & wait`)
/// and is terminated by group still loses that descendant: `recover`
/// with `terminate: true` reports `Terminated`, a following recovery
/// (once the leader is reaped) reports `Stopped`, and no process from
/// the group survives. The gate is released directly (bypassing
/// `finish`, which would also wait for the child) so the descendant
/// is genuinely alive, past its gate, when the explicit `recover`
/// calls run.
#[test]
fn orphan_process_is_stoppable() -> Result<(), PortError> {
    let spec = request("sh", &["-c", "sleep 30 & wait"], 4);
    let mut child = gated_command(&spec)
        .spawn()
        .map_err(|error| PortError(error.to_string()))?;
    let process_identity = identity(child.id())?;
    release(&mut child, spec.stdin.as_deref())?;
    thread::sleep(Duration::from_millis(200));
    assert_eq!(
        SystemProcess.recover(process_identity, true)?,
        Recovery::Terminated
    );
    // `recover` only signals the group; reap the leader ourselves
    // (as `finish` or `Drop` normally would) before checking that
    // nothing in the group is still alive.
    let _ = child.wait();
    assert_eq!(
        SystemProcess.recover(process_identity, false)?,
        Recovery::Stopped
    );
    assert!(!group_alive(process_identity.group));
    Ok(())
}

/// Whether any process in `group` still answers to a signal, the same
/// check `terminate_group` itself sends `SIGTERM` through, used here
/// with signal 0 to confirm nothing from an orphaned group survives.
fn group_alive(group: u32) -> bool {
    Command::new("kill")
        .args(["-0", &format!("-{group}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
