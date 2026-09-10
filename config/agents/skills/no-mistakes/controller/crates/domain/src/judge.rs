//! The LLM-as-judge verdict on one task and the run-level quality payload
//! folded from every verdict. The judge scores what the ledger, the
//! coordinator transcript and the rejection log show about a finished (or
//! abandoned) task; nothing here decides delivery, so a judgment never
//! blocks a command.

use crate::{
    ids::{LaunchId, TaskId},
    state::State,
    task::Task,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The harness's own output for a judge launch: the only fields it can
/// genuinely report. `launch` and `session` come from the controller's own
/// launch record (`Judgment`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JudgeReport {
    /// Did the delivery satisfy the brief with proof, and was the review
    /// sound.
    pub outcome: Dimension,
    /// Repairs, stalls, retries and budget relative to the tier.
    pub efficiency: Dimension,
    /// Freedom from human or coordinator administration: 100 means nobody
    /// had to clear, approve, re-supply or work around anything.
    pub friction: Dimension,
    /// Whether the controller's own rules held: sync, verification, merge.
    pub process: Dimension,
    /// 0 to 100, higher is better.
    pub overall: u8,
    pub verdict: JudgeVerdict,
    /// Every moment where a person or the coordinator had to do
    /// administration to keep the run moving, whatever its kind.
    pub friction_events: Vec<FrictionEvent>,
    pub summary: String,
    /// Concrete changes to the skill or controller that would have avoided
    /// the friction or improved the outcome.
    pub improvements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dimension {
    /// 0 to 100, higher is better.
    pub score: u8,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum JudgeVerdict {
    Pass,
    Degraded,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrictionEvent {
    /// A short free-form category chosen by the judge, for example
    /// `slot-administration`, `pr-reconciliation`, `evidence-demand`,
    /// `approval`, `human-hold`, `recovery`, or anything new it observes.
    pub kind: String,
    pub description: String,
    pub source: FrictionSource,
    pub severity: FrictionSeverity,
    /// Whether the skill or controller could have avoided it.
    pub avoidable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FrictionSource {
    Ledger,
    Transcript,
    Rejections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FrictionSeverity {
    Minor,
    Moderate,
    Severe,
}

/// The stored judgment: the report plus the launch it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Judgment {
    pub launch: LaunchId,
    pub session: String,
    pub report: JudgeReport,
}

/// One rejected controller command, kept beside the ledger (never in it)
/// as friction evidence for the judge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RejectionNote {
    pub at: crate::Instant,
    pub command: String,
    pub task: Option<TaskId>,
    /// A `--check` probe that was refused: still friction, but cheaper.
    pub check: bool,
    pub class: String,
    pub message: String,
}

/// The run's quality payload as recorded on its trace instance: one entry
/// per judged task and a rollup, re-sent whenever a judgment lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QualityPayload {
    pub tasks: BTreeMap<TaskId, TaskQuality>,
    pub rollup: Rollup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskQuality {
    pub verdict: JudgeVerdict,
    pub overall: u8,
    pub outcome: u8,
    pub efficiency: u8,
    pub friction: u8,
    pub process: u8,
    pub friction_events: u32,
    pub avoidable_friction_events: u32,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rollup {
    /// Every task the run discovered.
    pub tasks: u32,
    pub judged: u32,
    pub pass: u32,
    pub degraded: u32,
    pub fail: u32,
    pub mean_overall: u8,
    pub mean_friction: u8,
    pub friction_events: u32,
    pub avoidable_friction_events: u32,
    /// The worst task verdict, or `pass` when nothing has been judged.
    pub verdict: JudgeVerdict,
}

#[must_use]
pub fn quality(state: &State) -> QualityPayload {
    let judged: Vec<(&TaskId, &Judgment)> = state
        .tasks
        .iter()
        .filter_map(|(id, task)| task.judgment.as_ref().map(|judgment| (id, judgment)))
        .collect();
    let tasks = judged
        .iter()
        .map(|(id, judgment)| ((*id).clone(), task_quality(&judgment.report)))
        .collect();
    QualityPayload {
        tasks,
        rollup: rollup(state.tasks.values(), &judged),
    }
}

fn task_quality(report: &JudgeReport) -> TaskQuality {
    TaskQuality {
        verdict: report.verdict,
        overall: report.overall,
        outcome: report.outcome.score,
        efficiency: report.efficiency.score,
        friction: report.friction.score,
        process: report.process.score,
        friction_events: count(report.friction_events.len()),
        avoidable_friction_events: count(
            report
                .friction_events
                .iter()
                .filter(|event| event.avoidable)
                .count(),
        ),
        summary: report.summary.clone(),
    }
}

fn rollup<'a>(tasks: impl Iterator<Item = &'a Task>, judged: &[(&TaskId, &Judgment)]) -> Rollup {
    let reports: Vec<&JudgeReport> = judged.iter().map(|(_, item)| &item.report).collect();
    let verdicts = |wanted: JudgeVerdict| {
        count(
            reports
                .iter()
                .filter(|report| report.verdict == wanted)
                .count(),
        )
    };
    Rollup {
        tasks: count(tasks.count()),
        judged: count(reports.len()),
        pass: verdicts(JudgeVerdict::Pass),
        degraded: verdicts(JudgeVerdict::Degraded),
        fail: verdicts(JudgeVerdict::Fail),
        mean_overall: mean(reports.iter().map(|report| report.overall)),
        mean_friction: mean(reports.iter().map(|report| report.friction.score)),
        friction_events: count(
            reports
                .iter()
                .map(|report| report.friction_events.len())
                .sum(),
        ),
        avoidable_friction_events: count(
            reports
                .iter()
                .flat_map(|report| &report.friction_events)
                .filter(|event| event.avoidable)
                .count(),
        ),
        verdict: worst(&reports),
    }
}

fn worst(reports: &[&JudgeReport]) -> JudgeVerdict {
    if reports
        .iter()
        .any(|report| report.verdict == JudgeVerdict::Fail)
    {
        JudgeVerdict::Fail
    } else if reports
        .iter()
        .any(|report| report.verdict == JudgeVerdict::Degraded)
    {
        JudgeVerdict::Degraded
    } else {
        JudgeVerdict::Pass
    }
}

fn mean(scores: impl Iterator<Item = u8>) -> u8 {
    let (total, n) = scores.fold((0u64, 0u64), |(total, n), score| {
        (total + u64::from(score), n + 1)
    });
    total
        .checked_div(n)
        .map_or(0, |value| u8::try_from(value).unwrap_or(u8::MAX))
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(verdict: JudgeVerdict, overall: u8, friction: u8, avoidable: bool) -> JudgeReport {
        let dimension = |score| Dimension {
            score,
            rationale: String::new(),
        };
        JudgeReport {
            outcome: dimension(overall),
            efficiency: dimension(overall),
            friction: dimension(friction),
            process: dimension(overall),
            overall,
            verdict,
            friction_events: vec![FrictionEvent {
                kind: "approval".into(),
                description: String::new(),
                source: FrictionSource::Transcript,
                severity: FrictionSeverity::Minor,
                avoidable,
            }],
            summary: "s".into(),
            improvements: Vec::new(),
        }
    }

    fn reports(items: &[&JudgeReport]) -> Vec<&'static JudgeReport> {
        items
            .iter()
            .map(|report| Box::leak(Box::new((*report).clone())) as &'static JudgeReport)
            .collect()
    }

    #[test]
    fn rollup_takes_the_worst_verdict_and_integer_means() {
        let pass = report(JudgeVerdict::Pass, 90, 100, false);
        let degraded = report(JudgeVerdict::Degraded, 60, 41, true);
        let judged = reports(&[&pass, &degraded]);
        assert_eq!(worst(&judged), JudgeVerdict::Degraded);
        assert_eq!(mean(judged.iter().map(|report| report.overall)), 75);
        assert_eq!(mean(judged.iter().map(|report| report.friction.score)), 70);
        assert_eq!(mean(std::iter::empty()), 0);
    }

    #[test]
    fn task_quality_counts_avoidable_events() {
        let quality = task_quality(&report(JudgeVerdict::Fail, 10, 5, true));
        assert_eq!(quality.friction_events, 1);
        assert_eq!(quality.avoidable_friction_events, 1);
        assert_eq!(quality.verdict, JudgeVerdict::Fail);
    }
}
