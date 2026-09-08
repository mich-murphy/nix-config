use crate::task::CheckInput;
use crate::task_support::{current_snapshot, digest_file, next_operation, task_ref};
use crate::{AgentError, App, Output, Rejection, ResultData};
use adapters::ProcessRequest;
use domain::{
    acceptance::{self, ProofEntry, Snapshot},
    command::Lesson,
    delivery::Outcome,
    event::{Event, GitHubAction, Observation, Operation, OperationStatus},
    ids::{DeliveryId, Digest, FindingId, TaskId},
    review::Disposition,
    task::Phase,
};
use std::{collections::BTreeMap, path::PathBuf};

impl App<'_> {
    pub(super) fn record_proof(
        &mut self,
        task: TaskId,
        delivery: DeliveryId,
        entries: Vec<ProofEntry>,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("record-proof")?;
        let value = task_ref(&state, &task, "record-proof", self)?;
        let current = value
            .deliveries
            .iter()
            .find(|item| item.id == delivery)
            .ok_or_else(|| {
                self.error(
                    "record-proof",
                    Some(&task),
                    Rejection::Invalid("unknown delivery".into()),
                )
            })?;
        let criteria = current
            .criteria
            .iter()
            .map(|id| {
                value
                    .spec
                    .criteria
                    .iter()
                    .find(|criterion| criterion.id == *id)
                    .cloned()
                    .unwrap_or(domain::task::Criterion {
                        id: id.clone(),
                        text: "authorized narrowed criterion".into(),
                        human_only: false,
                    })
            })
            .collect::<Vec<_>>();
        acceptance::validate_batch(&entries, &criteria).map_err(|error| {
            self.error(
                "record-proof",
                Some(&task),
                Rejection::Evidence(format!("invalid proof: {error:?}")),
            )
        })?;
        if entries
            .iter()
            .any(|entry| entry.snapshot.requirements != value.spec.requirements)
            || !matches!(current.outcome, Outcome::Open)
        {
            return Err(self.error(
                "record-proof",
                Some(&task),
                Rejection::Evidence("proof is stale or delivery is closed".into()),
            ));
        }
        validate_artifacts(self, &task, &entries)?;
        self.commit(
            "record-proof",
            Some(&task),
            vec![Event::ProofRecorded {
                task: task.clone(),
                delivery,
                entries,
            }],
            check,
        )
    }

    pub(super) fn disposition(
        &mut self,
        task: TaskId,
        finding: FindingId,
        disposition: Disposition,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("disposition")?;
        let value = task_ref(&state, &task, "disposition", self)?;
        let known = value
            .deliveries
            .last()
            .and_then(|delivery| delivery.review.as_ref())
            .is_some_and(|review| review.findings.iter().any(|item| item.id == finding));
        if !known || disposition.evidence.trim().is_empty() {
            return Err(self.error(
                "disposition",
                Some(&task),
                Rejection::Invalid("finding and concrete evidence are required".into()),
            ));
        }
        self.commit(
            "disposition",
            Some(&task),
            vec![Event::Dispositioned {
                task: task.clone(),
                finding,
                disposition,
            }],
            check,
        )
    }

    pub(super) fn human_review(
        &mut self,
        task: TaskId,
        snapshot: Snapshot,
        receipt: Digest,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("human-review")?;
        let value = task_ref(&state, &task, "human-review", self)?;
        if current_snapshot(value) != Some(&snapshot) {
            return Err(self.error(
                "human-review",
                Some(&task),
                Rejection::Evidence("human review must pin the current snapshot".into()),
            ));
        }
        self.commit(
            "human-review",
            Some(&task),
            vec![Event::HumanReviewed {
                task: task.clone(),
                snapshot,
                receipt,
            }],
            check,
        )
    }

    pub(super) fn lesson(
        &mut self,
        task: TaskId,
        lesson: Lesson,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("lesson-record")?;
        let value = task_ref(&state, &task, "lesson-record", self)?;
        if lesson.accepted && !matches!(value.phase, Phase::Verified { .. }) {
            return Err(self.error(
                "lesson-record",
                Some(&task),
                Rejection::Conflict("accepted lessons require verified delivery".into()),
            ));
        }
        if state
            .lessons
            .iter()
            .any(|(_, existing)| existing.text == lesson.text)
        {
            return Err(self.error(
                "lesson-record",
                Some(&task),
                Rejection::Conflict("lesson already recorded".into()),
            ));
        }
        self.commit(
            "lesson-record",
            Some(&task),
            vec![Event::LessonRecorded {
                task: task.clone(),
                lesson,
            }],
            check,
        )
    }

    pub(super) fn run_check(
        &mut self,
        task: TaskId,
        input: CheckInput,
        check: bool,
    ) -> Result<Output, AgentError> {
        let state = self.state("run-check")?;
        let value = task_ref(&state, &task, "run-check", self)?;
        let delivery = value.deliveries.last().ok_or_else(|| {
            self.error(
                "run-check",
                Some(&task),
                Rejection::Conflict("task has no delivery".into()),
            )
        })?;
        validate_check(self, &task, &input)?;
        let id = next_operation(&state);
        let request = check_request(&input);
        if check {
            let operation = check_operation(&task, delivery.id, id, &input, None);
            return self.commit(
                "run-check",
                Some(&task),
                vec![Event::OperationStarted { operation }],
                true,
            );
        }
        let process =
            self.services.process.start(&request).map_err(|error| {
                self.error("run-check", Some(&task), Rejection::External(error.0))
            })?;
        let operation = check_operation(&task, delivery.id, id, &input, Some(process.identity()));
        let mut records = self.write(
            "run-check",
            Some(&task),
            vec![Event::OperationStarted { operation }],
            false,
        )?;
        let artifact = self.store.root().join(format!("check-{}.json", id.0));
        let result = execute_check(process, artifact);
        records.extend(self.write(
            "run-check",
            Some(&task),
            vec![Event::OperationSettled {
                operation: id,
                status: result.0,
                observation: result.1,
            }],
            false,
        )?);
        Ok(Output {
            events: records,
            result: ResultData::Applied,
        })
    }
}

fn validate_check(app: &App<'_>, task: &TaskId, input: &CheckInput) -> Result<(), AgentError> {
    let complete = !input.argv.is_empty()
        && input.timeout_seconds > 0
        && !input.criteria.is_empty()
        && !input.implementation.is_empty();
    if complete {
        Ok(())
    } else {
        Err(app.error(
            "run-check",
            Some(task),
            Rejection::Invalid(
                "check command, criteria, timeout and implementation are required".into(),
            ),
        ))
    }
}

fn check_request(input: &CheckInput) -> ProcessRequest {
    ProcessRequest {
        program: input.argv[0].clone(),
        args: input.argv[1..].to_vec(),
        cwd: input.cwd.clone(),
        stdin: None,
        env: BTreeMap::new(),
        remove_env: Vec::new(),
        timeout_seconds: input.timeout_seconds,
    }
}

fn check_operation(
    task: &TaskId,
    delivery: DeliveryId,
    id: domain::ids::OperationId,
    input: &CheckInput,
    process: Option<domain::ports::ProcessIdentity>,
) -> Operation {
    Operation {
        id,
        task: task.clone(),
        delivery,
        action: GitHubAction::Check {
            argv: input.argv.clone(),
            cwd: input.cwd.clone(),
            timeout_seconds: input.timeout_seconds,
        },
        status: OperationStatus::Running,
        process,
        timeout_seconds: input.timeout_seconds,
    }
}

fn execute_check(
    process: Box<dyn adapters::process::RunningProcess>,
    artifact: PathBuf,
) -> (OperationStatus, Observation) {
    match process.finish() {
        Ok(output) => check_output(output, artifact),
        Err(error) => {
            let _result = std::fs::write(&artifact, &error.0);
            (
                OperationStatus::Failed,
                Observation::Failure { reason: error.0 },
            )
        }
    }
}

fn check_output(
    output: adapters::ProcessOutput,
    artifact: PathBuf,
) -> (OperationStatus, Observation) {
    #[derive(serde::Serialize)]
    struct Receipt<'a> {
        exit: i32,
        stdout: &'a str,
        stderr: &'a str,
    }
    let exit = output.code.unwrap_or(-1);
    let receipt = Receipt {
        exit,
        stdout: &output.stdout,
        stderr: &output.stderr,
    };
    let status = if exit == 0 {
        OperationStatus::Confirmed
    } else {
        OperationStatus::Failed
    };
    if serde_json::to_vec(&receipt)
        .ok()
        .and_then(|bytes| std::fs::write(&artifact, bytes).ok())
        .is_none()
    {
        return (
            OperationStatus::Failed,
            Observation::Failure {
                reason: "failed to write check evidence".into(),
            },
        );
    }
    (status, Observation::Check { exit, artifact })
}

fn validate_artifacts(
    app: &App<'_>,
    task: &TaskId,
    entries: &[ProofEntry],
) -> Result<(), AgentError> {
    for entry in entries {
        let actual = digest_file(&entry.artifact).map_err(|message| {
            app.error("record-proof", Some(task), Rejection::Evidence(message))
        })?;
        if actual != entry.digest {
            return Err(app.error(
                "record-proof",
                Some(task),
                Rejection::Evidence("proof artifact digest changed".into()),
            ));
        }
    }
    Ok(())
}
