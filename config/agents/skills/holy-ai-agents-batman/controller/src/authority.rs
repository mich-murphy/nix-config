//! User-granted authority and the rules shared by every receipt type.
use crate::ids::{Digest, SlotId, TaskId};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub type AuthorityId = u64;
pub type PathGlob = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Authority {
    pub id: AuthorityId,
    pub source: String,
    pub artifact: PathBuf,
    pub digest: Digest,
    pub requirements: Digest,
    pub recorded: u64,
    pub grant: Grant,
    pub used: Option<UseId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Grant {
    Pair {
        paths: Option<Vec<PathGlob>>,
    },
    Delivery {
        kind: DeliveryKind,
        criteria: Vec<String>,
        paths: Option<Vec<PathGlob>>,
    },
    Narrowing {
        criteria: Vec<String>,
        operational_proof: String,
        cleanup_plan: String,
    },
    SlotReuse {
        historical: TaskId,
        slot: SlotId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeliveryKind {
    Code,
    Verification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UseId {
    pub implementation: Option<u64>,
    pub review: Option<u64>,
    #[serde(default)]
    pub receipt_only: bool,
}

pub fn register(authorities: &[Authority], authority: &Authority, idle: bool) -> Result<()> {
    ensure!(idle, "grant requires an idle task");
    ensure!(
        authority.artifact.is_absolute(),
        "authority receipt must be absolute"
    );
    validate_paths(paths(authority))?;
    ensure!(
        !authorities
            .iter()
            .any(|entry| entry.digest == authority.digest),
        "authority digest already registered"
    );
    if unused_pair(authority) {
        ensure!(
            !authorities.iter().any(unused_pair),
            "consume the unused pair first"
        );
    }
    Ok(())
}

pub fn validate_receipt(authority: &Authority) -> Result<()> {
    ensure!(
        authority.artifact.is_absolute(),
        "authority receipt must be absolute"
    );
    let bytes = fs::read(&authority.artifact).context("read authority receipt")?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    ensure!(
        actual == authority.digest.as_ref(),
        "authority receipt changed"
    );
    Ok(())
}

pub fn current_requirements(authority: &Authority, requirements: &str) -> Result<()> {
    ensure!(
        authority.requirements.as_ref() == requirements,
        "authority does not cover current criteria"
    );
    Ok(())
}

pub fn validate_requirements(authority: &Authority, requirements: &str) -> Result<()> {
    validate_receipt(authority)?;
    current_requirements(authority, requirements)
}

pub fn validate_paths(paths: Option<&[PathGlob]>) -> Result<()> {
    let Some(paths) = paths else {
        return Ok(());
    };
    ensure!(!paths.is_empty(), "path-scoped pair needs paths");
    ensure!(
        paths.iter().all(|path| {
            path.ends_with(".md")
                && !path.contains(['\\', '\n', '\r'])
                && Path::new(path)
                    .components()
                    .all(|component| matches!(component, Component::Normal(_)))
        }),
        "pair paths must be relative Markdown paths"
    );
    Ok(())
}

pub fn receipt_only(authority: &Authority) -> bool {
    authority
        .used
        .as_ref()
        .is_some_and(|used| used.receipt_only)
}

pub fn unused_pair(authority: &Authority) -> bool {
    matches!(authority.grant, Grant::Pair { .. }) && authority.used.is_none()
}

pub fn pair_available(authority: &Authority, implementation: bool) -> bool {
    if !matches!(authority.grant, Grant::Pair { .. } | Grant::Delivery { .. })
        || receipt_only(authority)
    {
        return false;
    }
    match &authority.used {
        None => true,
        Some(used) if implementation => used.implementation.is_none() && used.review.is_none(),
        Some(used) => used.review.is_none(),
    }
}

pub fn record_pair(authority: &mut Authority, implementation: bool, launch: u64) -> Result<()> {
    ensure!(
        pair_available(authority, implementation),
        "pair grant is spent"
    );
    let used = authority.used.get_or_insert_with(UseId::default);
    if implementation {
        used.implementation = Some(launch);
    } else {
        used.review = Some(launch);
    }
    Ok(())
}

pub fn repurposable(authority: &Authority) -> bool {
    unused_pair(authority)
}

pub fn scoped(authority: &Authority) -> bool {
    paths(authority).is_some()
}

pub fn paths(authority: &Authority) -> Option<&[PathGlob]> {
    match &authority.grant {
        Grant::Pair { paths } | Grant::Delivery { paths, .. } => paths.as_deref(),
        Grant::Narrowing { .. } | Grant::SlotReuse { .. } => None,
    }
}

pub fn current_pair(authorities: &[Authority]) -> Option<&Authority> {
    authorities.iter().rev().find(|authority| {
        matches!(authority.grant, Grant::Pair { .. } | Grant::Delivery { .. })
            && pair_available(authority, false)
    })
}

pub fn current_pair_mut(authorities: &mut [Authority]) -> Option<&mut Authority> {
    authorities.iter_mut().rev().find(|authority| {
        matches!(authority.grant, Grant::Pair { .. } | Grant::Delivery { .. })
            && pair_available(authority, false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn digest(seed: char) -> Result<Digest> {
        Digest::from_str(&seed.to_string().repeat(64)).map_err(anyhow::Error::msg)
    }

    fn authority(paths: Option<Vec<String>>) -> Result<Authority> {
        Ok(Authority {
            id: 1,
            source: "user".into(),
            artifact: PathBuf::from("/tmp/receipt"),
            digest: digest('a')?,
            requirements: digest('b')?,
            recorded: 1,
            grant: Grant::Pair { paths },
            used: None,
        })
    }

    #[test]
    fn digest_registers_once() -> Result<()> {
        let existing = authority(None)?;
        assert!(register(std::slice::from_ref(&existing), &existing, true).is_err());
        Ok(())
    }

    #[test]
    fn changed_receipt_blocks_grant() -> Result<()> {
        let mut grant = authority(None)?;
        grant.artifact = std::env::current_dir()?.join(file!());
        assert!(validate_requirements(&grant, grant.requirements.as_ref()).is_err());
        Ok(())
    }

    #[test]
    fn receipt_must_be_absolute() -> Result<()> {
        let mut grant = authority(None)?;
        grant.artifact = PathBuf::from("receipt");
        assert!(register(&[], &grant, true).is_err());
        grant.artifact = PathBuf::from("/missing/receipt");
        assert!(validate_receipt(&grant).is_err());
        Ok(())
    }

    #[test]
    fn grant_requires_current_criteria() -> Result<()> {
        let grant = authority(None)?;
        assert!(current_requirements(&grant, digest('c')?.as_ref()).is_err());
        Ok(())
    }

    #[test]
    fn partial_pair_stays_put() -> Result<()> {
        let mut pair = authority(None)?;
        record_pair(&mut pair, true, 2)?;
        assert!(!repurposable(&pair));
        Ok(())
    }

    #[test]
    fn scope_requires_markdown_paths() {
        assert!(validate_paths(Some(&[])).is_err());
        assert!(validate_paths(Some(&["src/lib.rs".into()])).is_err());
        assert!(validate_paths(Some(&["docs/../guide.md".into()])).is_err());
        assert!(validate_paths(Some(&["docs/guide.md".into()])).is_ok());
    }

    #[test]
    fn grant_requires_idle() -> Result<()> {
        assert!(register(&[], &authority(None)?, false).is_err());
        Ok(())
    }

    #[test]
    fn one_unused_pair() -> Result<()> {
        assert!(register(&[authority(None)?], &authority(None)?, true).is_err());
        Ok(())
    }

    #[test]
    #[ignore = "step 3d wires delivery opening"]
    fn unused_pair_repurposes() -> Result<()> {
        assert!(repurposable(&authority(None)?));
        Ok(())
    }
}
