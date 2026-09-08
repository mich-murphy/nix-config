use crate::{
    ids::{CriterionId, Digest, Sha},
    task::Criterion,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof {
    pub entries: BTreeMap<CriterionId, ProofEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofEntry {
    pub criterion: CriterionId,
    pub status: ProofStatus,
    pub kind: ProofKind,
    pub artifact: PathBuf,
    pub digest: Digest,
    pub snapshot: Snapshot,
    pub measurement: Option<Measurement>,
    pub implementation: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub base: Sha,
    pub head: Sha,
    pub requirements: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Measurement {
    pub baseline: Digest,
    pub observed: String,
    pub satisfied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofStatus {
    Passed,
    Failed,
    Skipped,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofKind {
    Command,
    Inspection,
    Human,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptanceError {
    Duplicate,
    Missing,
    Failed,
    HumanAutomation,
    Measurement,
    ArtifactChanged,
    BaselineChanged,
    SnapshotChanged,
}

pub fn validate_batch(
    entries: &[ProofEntry],
    criteria: &[Criterion],
) -> Result<(), AcceptanceError> {
    let mut seen = BTreeMap::new();
    for entry in entries {
        if seen.insert(&entry.criterion, ()).is_some() {
            return Err(AcceptanceError::Duplicate);
        }
        let criterion = criteria
            .iter()
            .find(|criterion| criterion.id == entry.criterion)
            .ok_or(AcceptanceError::Missing)?;
        validate_kind(criterion, entry)?;
    }
    Ok(())
}

pub fn complete(
    criteria: &[Criterion],
    proof: &Proof,
    snapshot: &Snapshot,
    artifacts: &BTreeMap<PathBuf, Digest>,
    baselines: &BTreeMap<CriterionId, Digest>,
) -> Result<(), AcceptanceError> {
    for criterion in criteria {
        let entry = proof
            .entries
            .get(&criterion.id)
            .ok_or(AcceptanceError::Missing)?;
        validate_entry(criterion, entry, snapshot, artifacts, baselines)?;
    }
    Ok(())
}

fn validate_entry(
    criterion: &Criterion,
    entry: &ProofEntry,
    snapshot: &Snapshot,
    artifacts: &BTreeMap<PathBuf, Digest>,
    baselines: &BTreeMap<CriterionId, Digest>,
) -> Result<(), AcceptanceError> {
    if entry.status != ProofStatus::Passed {
        return Err(AcceptanceError::Failed);
    }
    validate_kind(criterion, entry)?;
    if &entry.snapshot != snapshot {
        return Err(AcceptanceError::SnapshotChanged);
    }
    if artifacts.get(&entry.artifact) != Some(&entry.digest) {
        return Err(AcceptanceError::ArtifactChanged);
    }
    if let Some(measurement) = &entry.measurement
        && baselines.get(&criterion.id) != Some(&measurement.baseline)
    {
        return Err(AcceptanceError::BaselineChanged);
    }
    Ok(())
}

fn validate_kind(criterion: &Criterion, entry: &ProofEntry) -> Result<(), AcceptanceError> {
    if criterion.human_only && entry.kind != ProofKind::Human {
        return Err(AcceptanceError::HumanAutomation);
    }
    let measured = entry
        .measurement
        .as_ref()
        .is_some_and(|measurement| measurement.satisfied);
    if entry.kind != ProofKind::Human && !measured {
        return Err(AcceptanceError::Measurement);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn ids() -> Result<(CriterionId, Digest, Sha), crate::ids::InvalidId> {
        Ok((
            CriterionId::from_str("AC1")?,
            Digest::from_str(&"a".repeat(64))?,
            Sha::from_str(&"b".repeat(40))?,
        ))
    }

    #[test]
    fn human_criterion_rejects_automation() -> Result<(), crate::ids::InvalidId> {
        let (id, digest, sha) = ids()?;
        let criterion = Criterion {
            id: id.clone(),
            text: "approve".into(),
            human_only: true,
        };
        let entry = ProofEntry {
            criterion: id,
            status: ProofStatus::Passed,
            kind: ProofKind::Inspection,
            artifact: "/proof".into(),
            digest: digest.clone(),
            snapshot: Snapshot {
                base: sha.clone(),
                head: sha,
                requirements: digest.clone(),
            },
            measurement: Some(Measurement {
                baseline: digest,
                observed: "yes".into(),
                satisfied: true,
            }),
            implementation: Vec::new(),
            description: String::new(),
        };
        assert_eq!(
            validate_batch(&[entry], &[criterion]),
            Err(AcceptanceError::HumanAutomation)
        );
        Ok(())
    }

    #[test]
    fn acceptance_requires_measurement() -> Result<(), crate::ids::InvalidId> {
        let (id, digest, sha) = ids()?;
        let criterion = Criterion {
            id: id.clone(),
            text: "works".into(),
            human_only: false,
        };
        let entry = ProofEntry {
            criterion: id,
            status: ProofStatus::Passed,
            kind: ProofKind::Command,
            artifact: "/proof".into(),
            digest: digest.clone(),
            snapshot: Snapshot {
                base: sha.clone(),
                head: sha,
                requirements: digest,
            },
            measurement: None,
            implementation: Vec::new(),
            description: String::new(),
        };
        assert_eq!(
            validate_batch(&[entry], &[criterion]),
            Err(AcceptanceError::Measurement)
        );
        Ok(())
    }
}
