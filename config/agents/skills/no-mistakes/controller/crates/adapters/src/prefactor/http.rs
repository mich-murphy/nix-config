//! The two Prefactor calls the `prefactor` CLI cannot make: registering an
//! instance whose schema version declares quality schemas, and recording a
//! quality payload. Both go through `curl`, run like every other external
//! command through the `Process` port. The bearer token travels in a
//! header file (`-H @file`), never on the command line.

use crate::{Process, ProcessRequest};
use domain::ports::PortError;
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

use super::support::PayloadFile;

pub(super) struct Http<'a, P> {
    process: &'a P,
    base: &'a str,
    token: &'a str,
}

impl<'a, P: Process> Http<'a, P> {
    pub(super) fn new(process: &'a P, base: &'a str, token: &'a str) -> Self {
        Self {
            process,
            base,
            token,
        }
    }

    /// `POST {base}{path}` with a JSON body; returns the parsed response.
    /// A non-2xx status is an error carrying the response body, which is
    /// where Prefactor puts its validation messages.
    pub(super) fn post(&self, path: &str, body: &Value) -> Result<Value, PortError> {
        let headers = PayloadFile::text(&format!(
            "Authorization: Bearer {}\nContent-Type: application/json\n",
            self.token
        ))?;
        let payload = PayloadFile::new(body)?;
        let request = ProcessRequest {
            program: "curl".into(),
            args: vec![
                "-sS".into(),
                "--fail-with-body".into(),
                "-X".into(),
                "POST".into(),
                format!("{}{}", self.base.trim_end_matches('/'), path),
                "-H".into(),
                headers.arg(),
                "--data".into(),
                payload.arg(),
            ],
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
            timeout_seconds: 60,
        };
        let output = self.process.run(&request)?;
        if output.code != Some(0) {
            return Err(PortError(format!(
                "curl failed with {:?}: {} {}",
                output.code,
                output.stdout.trim(),
                output.stderr.trim()
            )));
        }
        serde_json::from_str(&output.stdout).map_err(|error| PortError(error.to_string()))
    }
}
