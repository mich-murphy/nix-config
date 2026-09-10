use crate::{Process, ProcessRequest, success};
use domain::{
    ports::{
        Capabilities, Harness, Isolation, LaunchRequest, LaunchResult, PortError, ProcessIdentity,
        StructuredOutput, Tokens,
    },
    risk::{Effort, HarnessKind},
};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

pub struct Claude<P> {
    process: P,
}

impl<P> Claude<P> {
    pub fn new(process: P) -> Self {
        Self { process }
    }
}

impl<P: Process> Harness for Claude<P> {
    fn kind(&self) -> HarnessKind {
        HarnessKind::Claude
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            structured_output: StructuredOutput::Native,
            // The reviewer runs under `dontAsk` with a read-only tool list
            // and a git allowlist, so it is tool-restricted, not an OS
            // sandbox.
            isolation: Isolation::ToolRestriction,
        }
    }

    fn run(
        &self,
        launch: &LaunchRequest,
        started: &mut dyn FnMut(ProcessIdentity) -> Result<(), PortError>,
    ) -> Result<LaunchResult, PortError> {
        let process = self.process.start(&request(launch)?)?;
        started(process.identity())?;
        let output = success(process.finish()?)?;
        parse(&output)
    }
}

fn request(launch: &LaunchRequest) -> Result<ProcessRequest, PortError> {
    let model = launch
        .assignment
        .model
        .as_ref()
        .split_once('/')
        .map_or(launch.assignment.model.as_ref(), |(_, id)| id);
    let mut args = vec![
        "-p".into(),
        "--output-format".into(),
        "json".into(),
        "--model".into(),
        model.into(),
        "--effort".into(),
        effort(launch.assignment.effort).into(),
        "--setting-sources".into(),
        "user".into(),
    ];
    if let Some(session) = &launch.session {
        args.extend(["--resume".into(), session.clone()]);
    }
    args.extend(role_args(launch.reviewer));
    if let Some(schema) = &launch.output_schema {
        args.extend(["--json-schema".into(), inline_schema(schema)?]);
    }
    Ok(ProcessRequest {
        program: "claude".into(),
        args,
        cwd: launch.cwd.clone(),
        stdin: Some(launch.prompt.clone()),
        env: BTreeMap::new(),
        remove_env: vec!["PREFACTOR_API_TOKEN".into()],
        timeout_seconds: 3600,
    })
}

fn role_args(reviewer: bool) -> Vec<String> {
    if reviewer {
        vec![
            "--permission-mode".into(),
            "dontAsk".into(),
            "--permission-prompts".into(),
            "none".into(),
            "--tools".into(),
            "Read,Grep,Glob,Bash".into(),
            "--allowedTools".into(),
            "Bash(git diff:*),Bash(git log:*),Bash(git show:*)".into(),
        ]
    } else {
        vec![
            "--permission-mode".into(),
            "bypassPermissions".into(),
            "--permission-prompts".into(),
            "none".into(),
        ]
    }
}

// Claude takes the review output schema inline (`--json-schema <contents>`),
// not as a path, so the controller reads the file itself before launch. Its
// validator rejects the draft 2020-12 `$schema` declaration that schemars
// writes ("no schema with key or ref"), so that key is dropped; the rest of
// the schema, `$defs` included, is accepted as written.
fn inline_schema(path: &Path) -> Result<String, PortError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| PortError(format!("failed to read output schema: {error}")))?;
    let mut schema: Value = serde_json::from_str(&text)
        .map_err(|error| PortError(format!("output schema is not JSON: {error}")))?;
    if let Some(object) = schema.as_object_mut() {
        object.remove("$schema");
    }
    serde_json::to_string(&schema).map_err(|error| PortError(error.to_string()))
}

fn parse(stream: &str) -> Result<LaunchResult, PortError> {
    let value: Value =
        serde_json::from_str(stream.trim()).map_err(|error| PortError(error.to_string()))?;
    if value.get("is_error").and_then(Value::as_bool) == Some(true) {
        let message = value
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or("no result");
        return Err(PortError(format!("Claude run failed: {message}")));
    }
    let session = value
        .get("session_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| PortError("Claude result omitted session".into()))?;
    Ok(LaunchResult {
        session,
        output: final_output(&value)?,
        tokens: parse_tokens(value.get("usage")),
    })
}

fn final_output(value: &Value) -> Result<String, PortError> {
    match value.get("structured_output") {
        Some(structured) if !structured.is_null() => {
            serde_json::to_string(structured).map_err(|error| PortError(error.to_string()))
        }
        _ => value
            .get("result")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| PortError("Claude result omitted final message".into())),
    }
}

fn parse_tokens(value: Option<&Value>) -> Option<Tokens> {
    let usage = value?;
    let input = usage.get("input_tokens")?.as_u64()?;
    let output = usage.get("output_tokens")?.as_u64()?;
    let cached = field_u64(usage, "cache_read_input_tokens");
    let created = field_u64(usage, "cache_creation_input_tokens").unwrap_or(0);
    Some(Tokens {
        input: input + cached.unwrap_or(0) + created,
        cached,
        output,
    })
}

fn field_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
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

    const SUCCESS_STREAM: &str = "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"{\\\"word\\\":\\\"pong\\\"}\",\"structured_output\":{\"word\":\"pong\"},\"session_id\":\"00000000-0000-4000-8000-000000000000\",\"num_turns\":2,\"usage\":{\"input_tokens\":10,\"cache_creation_input_tokens\":7125,\"cache_read_input_tokens\":0,\"output_tokens\":191}}";

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
                stdout: SUCCESS_STREAM.into(),
                stderr: String::new(),
            })))
        }
    }

    fn launch(
        reviewer: bool,
        output_schema: Option<PathBuf>,
    ) -> Result<LaunchRequest, domain::ids::InvalidId> {
        Ok(LaunchRequest {
            id: LaunchId(1),
            assignment: Assignment {
                model: ModelId::from_str("anthropic/claude-sonnet-5")?,
                effort: Effort::High,
            },
            prompt: "context".into(),
            cwd: PathBuf::from("/repo"),
            session: Some("old".into()),
            reviewer,
            output_schema,
        })
    }

    fn arg_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|index| args.get(index + 1))
            .map(String::as_str)
    }

    #[test]
    fn reviewer_argv_is_tool_restricted_with_inline_schema_minus_declaration()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let schema_path = dir.path().join("schema.json");
        std::fs::write(
            &schema_path,
            "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"type\":\"object\"}",
        )?;
        let request = request(&launch(true, Some(schema_path))?)?;
        assert_eq!(
            arg_after(&request.args, "--permission-mode"),
            Some("dontAsk")
        );
        assert_eq!(
            arg_after(&request.args, "--tools"),
            Some("Read,Grep,Glob,Bash")
        );
        assert_eq!(
            arg_after(&request.args, "--allowedTools"),
            Some("Bash(git diff:*),Bash(git log:*),Bash(git show:*)")
        );
        assert_eq!(
            arg_after(&request.args, "--json-schema"),
            Some("{\"type\":\"object\"}")
        );
        Ok(())
    }

    #[test]
    fn implementer_argv_bypasses_permissions_without_tool_flags()
    -> Result<(), domain::ids::InvalidId> {
        let request =
            request(&launch(false, None)?).map_err(|_| domain::ids::InvalidId("request"))?;
        assert_eq!(
            arg_after(&request.args, "--permission-mode"),
            Some("bypassPermissions")
        );
        assert!(!request.args.contains(&"--tools".into()));
        assert!(!request.args.contains(&"--json-schema".into()));
        Ok(())
    }

    #[test]
    fn prompt_travels_on_stdin_not_argv() -> Result<(), domain::ids::InvalidId> {
        let request =
            request(&launch(true, None)?).map_err(|_| domain::ids::InvalidId("request"))?;
        assert_eq!(request.stdin.as_deref(), Some("context"));
        assert!(!request.args.contains(&"context".into()));
        Ok(())
    }

    #[test]
    fn resume_carries_the_prior_session() -> Result<(), domain::ids::InvalidId> {
        let request =
            request(&launch(true, None)?).map_err(|_| domain::ids::InvalidId("request"))?;
        assert_eq!(arg_after(&request.args, "--resume"), Some("old"));
        Ok(())
    }

    #[test]
    fn model_prefix_is_stripped() -> Result<(), domain::ids::InvalidId> {
        let request =
            request(&launch(true, None)?).map_err(|_| domain::ids::InvalidId("request"))?;
        assert_eq!(arg_after(&request.args, "--model"), Some("claude-sonnet-5"));
        Ok(())
    }

    #[test]
    fn claude_run_yields_session_output_and_tokens() -> Result<(), domain::ids::InvalidId> {
        let harness = Claude::new(Fake(RefCell::new(Vec::new())));
        let result = harness
            .run(&launch(true, None)?, &mut |_| Ok(()))
            .map_err(|_| domain::ids::InvalidId("run"))?;
        assert_eq!(result.session, "00000000-0000-4000-8000-000000000000");
        assert_eq!(result.output, "{\"word\":\"pong\"}");
        let tokens = result.tokens.ok_or(domain::ids::InvalidId("tokens"))?;
        assert_eq!(tokens.input, 7135);
        assert_eq!(tokens.cached, Some(0));
        assert_eq!(tokens.output, 191);
        Ok(())
    }

    #[test]
    fn error_result_is_rejected_even_when_subtype_says_success() {
        let stream = "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":true,\"result\":\"Not logged in\",\"session_id\":\"s1\",\"usage\":{}}";
        assert!(parse(stream).is_err());
    }

    #[test]
    fn parse_falls_back_to_result_when_structured_output_absent() -> Result<(), PortError> {
        let stream = "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"done\",\"session_id\":\"s1\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}";
        let result = parse(stream)?;
        assert_eq!(result.output, "done");
        Ok(())
    }
}
