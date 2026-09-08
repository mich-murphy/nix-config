use crate::{Process, ProcessRequest, success};
use domain::{
    ports::{
        Capabilities, Harness, Isolation, LaunchRequest, LaunchResult, PortError, StructuredOutput,
        Tokens,
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

    fn run(&self, launch: &LaunchRequest) -> Result<LaunchResult, PortError> {
        let output = success(self.process.run(&request(launch))?)?;
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
    let mut args = vec![
        "exec".into(),
        "--json".into(),
        "--model".into(),
        model.into(),
    ];
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
    if let Some(session) = &launch.session {
        args.extend(["resume".into(), session.clone()]);
    }
    ProcessRequest {
        program: "codex".into(),
        args,
        cwd: launch.cwd.clone(),
        stdin: Some(launch.prompt.clone()),
        env: BTreeMap::new(),
        remove_env: vec!["OPENAI_API_KEY".into(), "CODEX_API_KEY".into()],
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
    impl Process for Fake {
        fn run(&self, request: &ProcessRequest) -> Result<crate::ProcessOutput, PortError> {
            self.0.borrow_mut().push(request.clone());
            Ok(crate::ProcessOutput {
                code: Some(0),
                stdout: "{\"type\":\"thread.started\",\"thread_id\":\"s1\"}\n{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"done\"}}\n{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":4,\"cached_input_tokens\":2,\"output_tokens\":1}}".into(),
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn codex_argv_matches_launch() -> Result<(), domain::ids::InvalidId> {
        let harness = Codex::new(Fake(RefCell::new(Vec::new())));
        let result = harness
            .run(&LaunchRequest {
                id: LaunchId(1),
                assignment: Assignment {
                    model: ModelId::from_str("openai/gpt-5")?,
                    effort: Effort::High,
                },
                prompt: "context".into(),
                cwd: PathBuf::from("/repo"),
                session: Some("old".into()),
                reviewer: true,
            })
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
}
