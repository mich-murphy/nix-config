use crate::{
    Instant,
    delivery::DeliveryKind,
    ids::{AuthorityId, CriterionId, Digest, LaunchId, SlotId, TaskId, UseId},
};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

pub type PathGlob = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authority {
    pub id: AuthorityId,
    pub source: String,
    pub artifact: PathBuf,
    pub digest: Digest,
    pub requirements: Digest,
    pub recorded: Instant,
    pub grant: Grant,
    pub used: AuthorityUse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grant {
    Pair {
        paths: Option<Vec<PathGlob>>,
    },
    Delivery {
        kind: DeliveryKind,
        criteria: Vec<CriterionId>,
        paths: Option<Vec<PathGlob>>,
    },
    Narrowing {
        criteria: Vec<CriterionId>,
        operational_proof: String,
        cleanup_plan: String,
    },
    SlotReuse {
        historical: TaskId,
        slot: SlotId,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityUse {
    pub whole: Option<UseId>,
    pub implementation: Option<LaunchId>,
    pub review: Option<LaunchId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityError {
    Duplicate,
    Spent,
    PairOpen,
    Requirements,
    NotIdle,
    ReceiptPath,
    ReceiptChanged,
    Scope,
}

pub fn register(all: &[Authority], new: &Authority, idle: bool) -> Result<(), AuthorityError> {
    if !idle {
        return Err(AuthorityError::NotIdle);
    }
    if !new.artifact.is_absolute() {
        return Err(AuthorityError::ReceiptPath);
    }
    validate_scope(paths(&new.grant))?;
    if all.iter().any(|entry| entry.digest == new.digest) {
        return Err(AuthorityError::Duplicate);
    }
    if matches!(new.grant, Grant::Pair { .. }) && all.iter().any(unused_pair) {
        return Err(AuthorityError::PairOpen);
    }
    Ok(())
}

pub fn validate_receipt(authority: &Authority, observed: &Digest) -> Result<(), AuthorityError> {
    if !authority.artifact.is_absolute() {
        return Err(AuthorityError::ReceiptPath);
    }
    if &authority.digest != observed {
        return Err(AuthorityError::ReceiptChanged);
    }
    Ok(())
}

pub fn validate_use(authority: &Authority, requirements: &Digest) -> Result<(), AuthorityError> {
    if authority.used.whole.is_some() {
        return Err(AuthorityError::Spent);
    }
    if &authority.requirements != requirements {
        return Err(AuthorityError::Requirements);
    }
    Ok(())
}

pub fn unused_pair(authority: &Authority) -> bool {
    matches!(authority.grant, Grant::Pair { .. })
        && authority.used.implementation.is_none()
        && authority.used.review.is_none()
        && authority.used.whole.is_none()
}

pub fn repurpose(all: &[Authority], digest: &Digest) -> Result<AuthorityId, AuthorityError> {
    all.iter()
        .find(|entry| &entry.digest == digest && unused_pair(entry))
        .map(|entry| entry.id)
        .ok_or(AuthorityError::Spent)
}

pub fn pair_available(authority: &Authority, implementation: bool) -> bool {
    let grants_pair = matches!(authority.grant, Grant::Pair { .. } | Grant::Delivery { .. });
    let spent_pair_grant =
        matches!(authority.grant, Grant::Pair { .. }) && authority.used.whole.is_some();
    if !grants_pair || spent_pair_grant {
        return false;
    }
    if implementation {
        authority.used.implementation.is_none() && authority.used.review.is_none()
    } else {
        authority.used.review.is_none()
    }
}

pub fn spend_pair(
    authority: &mut Authority,
    implementation: bool,
    launch: LaunchId,
) -> Result<(), AuthorityError> {
    if !pair_available(authority, implementation) {
        return Err(AuthorityError::Spent);
    }
    if implementation {
        authority.used.implementation = Some(launch);
    } else {
        authority.used.review = Some(launch);
    }
    Ok(())
}

pub fn paths(grant: &Grant) -> Option<&[PathGlob]> {
    match grant {
        Grant::Pair { paths } | Grant::Delivery { paths, .. } => paths.as_deref(),
        Grant::Narrowing { .. } | Grant::SlotReuse { .. } => None,
    }
}

pub fn scoped(authority: &Authority) -> bool {
    paths(&authority.grant).is_some()
}

pub fn validate_scope(paths: Option<&[PathGlob]>) -> Result<(), AuthorityError> {
    let Some(paths) = paths else {
        return Ok(());
    };
    let invalid = paths.is_empty() || paths.iter().any(|path| !markdown_path(path));
    if invalid {
        Err(AuthorityError::Scope)
    } else {
        Ok(())
    }
}

fn markdown_path(path: &str) -> bool {
    path.ends_with(".md")
        && !path.contains(['\\', '\n', '\r'])
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn authority() -> Result<Authority, crate::ids::InvalidId> {
        Ok(Authority {
            id: AuthorityId(1),
            source: "user".into(),
            artifact: PathBuf::from("/receipt"),
            digest: Digest::from_str(&"a".repeat(64))?,
            requirements: Digest::from_str(&"b".repeat(64))?,
            recorded: 1,
            grant: Grant::Pair { paths: None },
            used: AuthorityUse::default(),
        })
    }

    #[test]
    fn digest_registers_once() -> Result<(), crate::ids::InvalidId> {
        let receipt = authority()?;
        assert_eq!(
            register(std::slice::from_ref(&receipt), &receipt, true),
            Err(AuthorityError::Duplicate)
        );
        Ok(())
    }

    #[test]
    fn changed_receipt_blocks_grant() -> Result<(), crate::ids::InvalidId> {
        let receipt = authority()?;
        let changed = Digest::from_str(&"c".repeat(64))?;
        assert_eq!(
            validate_receipt(&receipt, &changed),
            Err(AuthorityError::ReceiptChanged)
        );
        Ok(())
    }

    #[test]
    fn receipt_must_be_absolute() -> Result<(), crate::ids::InvalidId> {
        let mut receipt = authority()?;
        receipt.artifact = PathBuf::from("receipt");
        assert_eq!(
            register(&[], &receipt, true),
            Err(AuthorityError::ReceiptPath)
        );
        Ok(())
    }

    #[test]
    fn grant_requires_current_criteria() -> Result<(), crate::ids::InvalidId> {
        let receipt = authority()?;
        let changed = Digest::from_str(&"c".repeat(64))?;
        assert_eq!(
            validate_use(&receipt, &changed),
            Err(AuthorityError::Requirements)
        );
        Ok(())
    }

    #[test]
    fn partial_pair_stays_put() -> Result<(), crate::ids::InvalidId> {
        let mut receipt = authority()?;
        spend_pair(&mut receipt, true, LaunchId(1)).map_err(|_| crate::ids::InvalidId("spend"))?;
        assert_eq!(
            repurpose(&[receipt], &Digest::from_str(&"a".repeat(64))?),
            Err(AuthorityError::Spent)
        );
        Ok(())
    }

    #[test]
    fn scope_requires_markdown_paths() {
        assert_eq!(validate_scope(Some(&[])), Err(AuthorityError::Scope));
        assert_eq!(
            validate_scope(Some(&["src/lib.rs".into()])),
            Err(AuthorityError::Scope)
        );
        assert!(validate_scope(Some(&["docs/guide.md".into()])).is_ok());
    }

    #[test]
    fn grant_requires_idle() -> Result<(), crate::ids::InvalidId> {
        assert_eq!(
            register(&[], &authority()?, false),
            Err(AuthorityError::NotIdle)
        );
        Ok(())
    }

    #[test]
    fn one_unused_pair() -> Result<(), crate::ids::InvalidId> {
        let first = authority()?;
        let mut second = authority()?;
        second.id = AuthorityId(2);
        second.digest = Digest::from_str(&"c".repeat(64))?;
        assert_eq!(
            register(&[first], &second, true),
            Err(AuthorityError::PairOpen)
        );
        Ok(())
    }

    #[test]
    fn unused_pair_repurposes() -> Result<(), crate::ids::InvalidId> {
        let receipt = authority()?;
        assert_eq!(
            repurpose(std::slice::from_ref(&receipt), &receipt.digest),
            Ok(receipt.id)
        );
        Ok(())
    }
}
