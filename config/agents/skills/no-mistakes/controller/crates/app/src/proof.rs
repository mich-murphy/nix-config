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
        for entry in &entries {
            let actual = digest_file(&entry.artifact).map_err(|message| {
                self.error("record-proof", Some(&task), Rejection::Evidence(message))
            })?;
            if actual != entry.digest {
                return Err(self.error(
                    "record-proof",
                    Some(&task),
                    Rejection::Evidence("proof artifact digest changed".into()),
                ));
            }
        }
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
        if input.argv.is_empty()
            || input.timeout_seconds == 0
            || input.criteria.is_empty()
            || input.implementation.is_empty()
        {
            return Err(self.error(
                "run-check",
                Some(&task),
                Rejection::Invalid(
                    "check command, criteria, timeout and implementation are required".into(),
                ),
            ));
        }
        let id = next_operation(&state);
        let operation = Operation {
            id,
            task: task.clone(),
            delivery: delivery.id,
            action: GitHubAction::Check {
                argv: input.argv.clone(),
                cwd: input.cwd.clone(),
            },
            status: OperationStatus::Running,
        };
        if check {
            return self.commit(
                "run-check",
                Some(&task),
                vec![Event::OperationStarted { operation }],
                true,
            );
        }
        let mut records = self.write(
            "run-check",
            Some(&task),
            vec![Event::OperationStarted { operation }],
            false,
        )?;
        let request = ProcessRequest {
            program: input.argv[0].clone(),
            args: input.argv[1..].to_vec(),
            cwd: input.cwd,
            stdin: None,
            env: BTreeMap::new(),
            remove_env: Vec::new(),
        };
        let output = self.services.process.run(&request);
        let (status, observation) = match output {
            Ok(output) if output.code == Some(0) => (
                OperationStatus::Confirmed,
                Observation::Check {
                    exit: 0,
                    artifact: PathBuf::new(),
                },
            ),
            Ok(output) => (
                OperationStatus::Failed,
                Observation::Check {
                    exit: output.code.unwrap_or(-1),
                    artifact: PathBuf::new(),
                },
            ),
            Err(error) => (
                OperationStatus::Failed,
                Observation::Failure { reason: error.0 },
            ),
        };
        records.extend(self.write(
            "run-check",
            Some(&task),
            vec![Event::OperationSettled {
                operation: id,
                status,
                observation,
            }],
            false,
        )?);
        Ok(Output {
            events: records,
            result: ResultData::Applied,
        })
    }
}
