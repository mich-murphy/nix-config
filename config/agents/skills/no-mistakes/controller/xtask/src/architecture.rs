use anyhow::{Result, ensure};
use std::{fs, path::Path};

pub fn check(root: &Path, max_lines: usize) -> Result<()> {
    let mut files = 0_usize;
    visit(root, max_lines, &mut files)?;
    ensure!(files > 0, "No Rust source files found");
    println!(
        "Architecture passed for {}: {files} files, at most {max_lines} lines each",
        root.display()
    );
    Ok(())
}

fn visit(path: &Path, max_lines: usize, files: &mut usize) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            visit(&path, max_lines, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            inspect(&path, max_lines)?;
            *files += 1;
        }
    }
    Ok(())
}

fn inspect(path: &Path, max_lines: usize) -> Result<()> {
    let source = fs::read_to_string(path)?;
    let lines = source.lines().count();
    ensure!(
        lines <= max_lines,
        "Rust file exceeds {max_lines} lines: {} has {lines}",
        path.display()
    );
    let normalized = path.to_string_lossy().replace('\\', "/");
    let adapter = normalized.contains("/crates/adapters/");
    let executes = source.contains("std::process::Command")
        || source.contains("process::Command")
        || source.contains("Command::new(");
    ensure!(
        adapter || !executes,
        "process execution outside adapters: {}",
        path.display()
    );
    let domain = normalized.contains("/crates/domain/");
    ensure!(
        !domain || !source.contains("serde_json"),
        "serde_json is forbidden in domain: {}",
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_file_fails() -> Result<()> {
        let directory = tempfile::tempdir()?;
        fs::write(directory.path().join("lib.rs"), "line\n".repeat(401))?;
        assert!(check(directory.path(), 400).is_err());
        Ok(())
    }

    #[test]
    fn process_outside_adapters_fails() -> Result<()> {
        let directory = tempfile::tempdir()?;
        fs::write(
            directory.path().join("lib.rs"),
            "fn run() { std::process::Command::new(\"git\"); }",
        )?;
        assert!(check(directory.path(), 400).is_err());
        Ok(())
    }
}
