use crate::{Process, ProcessRequest, success};
use domain::{
    ports::{
        Capabilities, Harness, Isolation, LaunchRequest, LaunchResult, PortError, ProcessIdentity,
        StructuredOutput, Tokens,
    },
    risk::{Effort, HarnessKind},
};
use serde_json::Value;
use std::collections::BTreeMap;

pub struct Pi<P> {
    process: P,
}

impl<P> Pi<P> {
    pub fn new(process: P) -> Self {
        Self { process }
    }
}

impl<P: Process> Harness for Pi<P> {
    fn kind(&self) -> HarnessKind {
        HarnessKind::Pi
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            structured_output: StructuredOutput::SelfValidated,
            // The reviewer keeps `bash` in its tool list (owner's
            // decision: it needs `git diff`), so this is not a
            // tool-restricted sandbox, whatever the argv passes.
            isolation: Isolation::None,
        }
    }

    fn run(
        &self,
        launch: &LaunchRequest,
        started: &mut dyn FnMut(ProcessIdentity) -> Result<(), PortError>,
    ) -> Result<LaunchResult, PortError> {
        let process = self.process.start(&request(launch))?;
        started(process.identity())?;
        let output = success(process.finish()?)?;
        parse(&output, launch.session.as_deref())
    }
}

fn request(launch: &LaunchRequest) -> ProcessRequest {
    let mut args = vec![
        "-p".into(),
        "--approve".into(),
        "--mode".into(),
        "json".into(),
        "--model".into(),
        launch.assignment.model.to_string(),
        "--thinking".into(),
        effort(launch.assignment.effort).into(),
    ];
    if let Some(session) = &launch.session {
        args.extend(["--session".into(), session.clone()]);
    }
    if launch.reviewer {
        args.extend(["--tools".into(), "read,bash,grep,find,ls".into()]);
    }
    args.push(launch.prompt.clone());
    ProcessRequest {
        program: "pi".into(),
        args,
        cwd: launch.cwd.clone(),
        stdin: None,
        env: BTreeMap::new(),
        remove_env: vec!["PREFACTOR_API_TOKEN".into()],
        timeout_seconds: 3600,
    }
}

fn parse(stream: &str, existing_session: Option<&str>) -> Result<LaunchResult, PortError> {
    let mut session = existing_session.map(ToOwned::to_owned);
    let mut output = None;
    let mut tokens = None;
    for line in stream.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value =
            serde_json::from_str(line).map_err(|error| PortError(error.to_string()))?;
        if session.is_none() {
            session = value
                .get("sessionId")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
        match value.get("type").and_then(Value::as_str) {
            Some("message_update") => tokens = parse_tokens(&value).or(tokens),
            Some("agent_end") => output = final_message(&value),
            _ => {}
        }
    }
    Ok(LaunchResult {
        session: session.ok_or_else(|| PortError("pi stream omitted session".into()))?,
        output: output.ok_or_else(|| PortError("pi stream omitted final message".into()))?,
        tokens,
    })
}

fn final_message(value: &Value) -> Option<String> {
    value
        .get("messages")?
        .as_array()?
        .iter()
        .rev()
        .find(|message| message.get("role").and_then(Value::as_str) == Some("assistant"))
        .and_then(|message| message.get("content"))
        .and_then(|content| match content {
            Value::String(text) => Some(text.clone()),
            Value::Array(parts) => parts.iter().find_map(|part| {
                part.get("text")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            }),
            _ => None,
        })
}

fn parse_tokens(value: &Value) -> Option<Tokens> {
    let usage = value
        .pointer("/message/usage")
        .or_else(|| value.get("usage"))?;
    Some(Tokens {
        input: usage.get("input")?.as_u64()?,
        cached: usage.get("cacheRead").and_then(Value::as_u64),
        output: usage.get("output")?.as_u64()?,
    })
}

const fn effort(value: Effort) -> &'static str {
    match value {
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{
        ids::{LaunchId, ModelId},
        risk::Assignment,
    };
    use std::{cell::RefCell, path::PathBuf, str::FromStr};

    struct Fake(RefCell<Vec<ProcessRequest>>);
    struct FakeRunning(crate::ProcessOutput);
    impl crate::process::RunningProcess for FakeRunning {
        fn identity(&self) -> ProcessIdentity {
            ProcessIdentity {
                pid: 1,
                start_ticks: 2,
                group: 1,
            }
        }
        fn finish(self: Box<Self>) -> Result<crate::ProcessOutput, PortError> {
            Ok(self.0)
        }
    }
    impl Process for Fake {
        fn run(&self, request: &ProcessRequest) -> Result<crate::ProcessOutput, PortError> {
            self.start(request)?.finish()
        }
        fn start(
            &self,
            request: &ProcessRequest,
        ) -> Result<Box<dyn crate::process::RunningProcess>, PortError> {
            self.0.borrow_mut().push(request.clone());
            Ok(Box::new(FakeRunning(crate::ProcessOutput {
                code: Some(0),
                stdout: "{\"type\":\"message_update\",\"sessionId\":\"p1\",\"usage\":{\"input\":3,\"cacheRead\":1,\"output\":2}}\n{\"type\":\"agent_end\",\"messages\":[{\"role\":\"assistant\",\"content\":\"done\"}]}".into(),
                stderr: String::new(),
            })))
        }
    }

    #[test]
    fn pi_argv_matches_launch() -> Result<(), domain::ids::InvalidId> {
        let harness = Pi::new(Fake(RefCell::new(Vec::new())));
        let result = harness
            .run(
                &LaunchRequest {
                    id: LaunchId(1),
                    assignment: Assignment {
                        model: ModelId::from_str("openai/gpt-5")?,
                        effort: Effort::Medium,
                    },
                    prompt: "context".into(),
                    cwd: PathBuf::from("/repo"),
                    session: None,
                    reviewer: true,
                    output_schema: None,
                },
                &mut |_| Ok(()),
            )
            .map_err(|_| domain::ids::InvalidId("run"))?;
        assert_eq!(result.output, "done");
        assert_eq!(result.tokens.map(|value| value.cached), Some(Some(1)));
        assert!(
            harness.process.0.borrow()[0]
                .args
                .contains(&"--tools".into())
        );
        Ok(())
    }

    #[test]
    fn pi_stream_yields_final_and_usage() -> Result<(), PortError> {
        let stream = "{\"type\":\"message_update\",\"sessionId\":\"p1\",\"usage\":{\"input\":3,\"cacheRead\":1,\"output\":2}}\n{\"type\":\"agent_end\",\"messages\":[{\"role\":\"assistant\",\"content\":\"done\"}]}";
        let result = parse(stream, None)?;
        assert_eq!(result.output, "done");
        assert_eq!(result.tokens.map(|tokens| tokens.output), Some(2));
        Ok(())
    }
}
