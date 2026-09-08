use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
pub struct Settings {
    version: String,
    url: String,
    archive_sha256: String,
    executable_sha256: String,
}

fn verify(bytes: &[u8], expected: &str) -> Result<()> {
    ensure!(
        format!("{:x}", Sha256::digest(bytes)) == expected,
        "Analyzer checksum mismatch"
    );
    Ok(())
}

fn install(settings: &Settings, path: &Path) -> Result<()> {
    let parent = path.parent().context("Analyzer cache has no parent")?;
    fs::create_dir_all(parent)?;
    let directory = tempfile::tempdir_in(parent)?;
    let archive = directory.path().join("analyzer.tar.gz");
    println!("Installing pinned rust-code-analysis {}", settings.version);
    let status = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--max-time",
            "30",
            "--output",
        ])
        .arg(&archive)
        .arg(&settings.url)
        .status()?;
    ensure!(status.success(), "Analyzer download failed");
    verify(&fs::read(&archive)?, &settings.archive_sha256)?;
    // Stream only the named member; never extract archive paths into the filesystem.
    let bytes = Command::new("tar")
        .args(["-xOzf"])
        .arg(&archive)
        .arg("rust-code-analysis-cli")
        .output()?;
    ensure!(
        bytes.status.success(),
        "Cannot read analyzer executable from archive"
    );
    verify(&bytes.stdout, &settings.executable_sha256)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&bytes.stdout)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o755))?;
    }
    temporary.persist(path)?;
    Ok(())
}

pub fn resolve(settings: &Settings, supplied: Option<PathBuf>, cache: &Path) -> Result<PathBuf> {
    ensure!(
        cfg!(all(target_os = "linux", target_arch = "x86_64")),
        "The full controller gate requires Linux x86_64 for process-recovery proof"
    );
    let custom = supplied.is_some();
    let path = match supplied {
        Some(path) => path,
        None => cache
            .join("analyzer")
            .join(&settings.version)
            .join("rust-code-analysis-cli"),
    };
    if !path.exists() && !custom {
        install(settings, &path)?;
    }
    verify(
        &fs::read(&path).context("Pinned analyzer missing")?,
        &settings.executable_sha256,
    )?;
    Ok(path.canonicalize()?)
}
