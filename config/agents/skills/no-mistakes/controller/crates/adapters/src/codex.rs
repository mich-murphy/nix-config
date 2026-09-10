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

pub struct Codex<P> {
    process: P,
}

impl<P> Codex<P> {
    pub fn new(process: P) -> Self {
        Self { process }
    }
}

impl<P: Process> Harness for Codex<P> {
    fn kind(&self) -> HarnessKind {
        HarnessKind::Codex
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            structured_output: StructuredOutput::Native,
            isolation: Isolation::Sandbox,
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
        parse(&output)
    }
}

fn request(launch: &LaunchRequest) -> ProcessRequest {
    let model = launch
        .assignment
        .model
        .as_ref()
        .split_once('/')
        .map_or(launch.assignment.model.as_ref(), |(_, id)| id);
    // The codex CLI grammar is `codex exec resume <SESSION_ID> [OPTIONS]`:
    // the session id is a positional argument to `resume`, immediately
    // after `exec`, not a trailing flag.
    let mut args = vec!["exec".into()];
    if let Some(session) = &launch.session {
        args.extend(["resume".into(), session.clone()]);
    }
    args.extend(["--json".into(), "--model".into(), model.into()]);
    args.extend([
        "-c".into(),
        format!(
            "model_reasoning_effort=\"{}\"",
            effort(launch.assignment.effort)
        ),
    ]);
    args.extend([
        "--sandbox".into(),
        if launch.reviewer {
            "read-only"
        } else {
            "workspace-write"
        }
        .into(),
    ]);
    if let Some(schema) = &launch.output_schema {
        args.extend(["--output-schema".into(), schema.display().to_string()]);
    }
    ProcessRequest {
        program: "codex".into(),
        args,
        cwd: launch.cwd.clone(),
        stdin: Some(launch.prompt.clone()),
        env: BTreeMap::new(),
        remove_env: vec![
            "OPENAI_API_KEY".into(),
            "CODEX_API_KEY".into(),
            "PREFACTOR_API_TOKEN".into(),
        ],
        timeout_seconds: 3600,
    }
}

fn parse(stream: &str) -> Result<LaunchResult, PortError> {
    let mut session = None;
    let mut output = None;
    let mut tokens = None;
    for line in stream.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value =
            serde_json::from_str(line).map_err(|error| PortError(error.to_string()))?;
        match value.get("type").and_then(Value::as_str) {
            Some("thread.started") => {
                session = value
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            }
            Some("item.completed") => output = agent_message(&value).or(output),
            Some("turn.completed") => tokens = parse_tokens(value.get("usage")),
            _ => {}
        }
    }
    Ok(LaunchResult {
        session: session.ok_or_else(|| PortError("Codex stream omitted session".into()))?,
        output: output.ok_or_else(|| PortError("Codex stream omitted final message".into()))?,
        tokens,
    })
}

fn agent_message(value: &Value) -> Option<String> {
    (value.pointer("/item/type").and_then(Value::as_str) == Some("agent_message"))
        .then(|| value.pointer("/item/text").and_then(Value::as_str))
        .flatten()
        .map(ToOwned::to_owned)
}

fn parse_tokens(value: Option<&Value>) -> Option<Tokens> {
    let value = value?;
    Some(Tokens {
        input: value.get("input_tokens")?.as_u64()?,
        cached: value.get("cached_input_tokens").and_then(Value::as_u64),
        output: value.get("output_tokens")?.as_u64()?,
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
                stdout: "{\"type\":\"thread.started\",\"thread_id\":\"s1\"}\n{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"done\"}}\n{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":4,\"cached_input_tokens\":2,\"output_tokens\":1}}".into(),
                stderr: String::new(),
            })))
        }
    }

    fn launch() -> Result<LaunchRequest, domain::ids::InvalidId> {
        Ok(LaunchRequest {
            id: LaunchId(1),
            assignment: Assignment {
                model: ModelId::from_str("openai/gpt-5")?,
                effort: Effort::High,
            },
            prompt: "context".into(),
            cwd: PathBuf::from("/repo"),
            session: Some("old".into()),
            reviewer: true,
            output_schema: Some(PathBuf::from("/run/review-schema.json")),
        })
    }

    #[test]
    fn codex_argv_matches_launch() -> Result<(), domain::ids::InvalidId> {
        let harness = Codex::new(Fake(RefCell::new(Vec::new())));
        let result = harness
            .run(&launch()?, &mut |_| Ok(()))
            .map_err(|_| domain::ids::InvalidId("run"))?;
        assert_eq!(result.tokens.map(|value| value.cached), Some(Some(2)));
        assert_eq!(
            harness.process.0.borrow()[0].stdin.as_deref(),
            Some("context")
        );
        assert!(
            harness.process.0.borrow()[0]
                .args
                .contains(&"read-only".into())
        );
        Ok(())
    }

    #[test]
    fn codex_resume_follows_exec() -> Result<(), domain::ids::InvalidId> {
        let request = request(&launch()?);
        assert_eq!(request.args[0], "exec");
        assert_eq!(request.args[1], "resume");
        assert_eq!(request.args[2], "old");
        Ok(())
    }

    #[test]
    fn codex_reviewer_passes_output_schema() -> Result<(), domain::ids::InvalidId> {
        let request = request(&launch()?);
        let index = request
            .args
            .iter()
            .position(|arg| arg == "--output-schema")
            .ok_or(domain::ids::InvalidId("missing --output-schema"))?;
        assert_eq!(request.args[index + 1], "/run/review-schema.json");
        Ok(())
    }

    #[test]
    fn codex_stream_yields_usage() -> Result<(), PortError> {
        let result = parse(
            "{\"type\":\"thread.started\",\"thread_id\":\"s1\"}\n{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"done\"}}\n{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":4,\"cached_input_tokens\":2,\"output_tokens\":1}}",
        )?;
        assert_eq!(result.session, "s1");
        assert_eq!(result.tokens.map(|tokens| tokens.input), Some(4));
        Ok(())
    }

    #[test]
    fn harness_injects_context() -> Result<(), domain::ids::InvalidId> {
        let request = request(&launch()?);
        assert_eq!(request.stdin.as_deref(), Some("context"));
        Ok(())
    }
}
