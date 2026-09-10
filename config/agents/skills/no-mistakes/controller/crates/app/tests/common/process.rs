use super::Fake;
use adapters::process::{Recovery, RunningProcess};
use adapters::{Process, ProcessOutput, ProcessRequest};
use domain::ports::{PortError, ProcessIdentity};

impl Process for Fake {
    fn run(&self, _request: &ProcessRequest) -> Result<ProcessOutput, PortError> {
        Ok(ProcessOutput::default())
    }

    fn start(&self, _request: &ProcessRequest) -> Result<Box<dyn RunningProcess>, PortError> {
        Ok(Box::new(FakeProcess))
    }

    fn recover(&self, _identity: ProcessIdentity, terminate: bool) -> Result<Recovery, PortError> {
        Ok(if terminate {
            Recovery::Terminated
        } else {
            Recovery::Stopped
        })
    }

    fn spawn(&self, request: &ProcessRequest, _log: &std::path::Path) -> Result<u32, PortError> {
        self.spawned.borrow_mut().push(request.clone());
        Ok(4242)
    }
}

struct FakeProcess;

impl RunningProcess for FakeProcess {
    fn identity(&self) -> ProcessIdentity {
        ProcessIdentity {
            pid: 1,
            start_ticks: 2,
            group: 1,
        }
    }

    fn finish(self: Box<Self>) -> Result<ProcessOutput, PortError> {
        Ok(ProcessOutput::default())
    }
}
