use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
struct Score {
    sum: f64,
}
#[derive(Deserialize)]
struct Metrics {
    cyclomatic: Score,
}
#[derive(Deserialize)]
struct Space {
    name: String,
    kind: String,
    start_line: u64,
    metrics: Metrics,
    spaces: Vec<Space>,
}

fn files(root: &Path, extension: &str) -> Result<BTreeSet<PathBuf>> {
    let mut result = BTreeSet::new();
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            result.extend(files(&path, extension)?);
        } else if path.extension().is_some_and(|value| value == extension) {
            result.insert(path.canonicalize()?);
        }
    }
    Ok(result)
}

fn measure(
    space: &Space,
    path: &Path,
    limit: u32,
    scores: &mut Vec<f64>,
    failures: &mut Vec<String>,
) {
    if space.kind == "function" {
        let own = space.metrics.cyclomatic.sum
            - space
                .spaces
                .iter()
                .map(|child| child.metrics.cyclomatic.sum)
                .sum::<f64>();
        scores.push(own);
        if own > f64::from(limit) {
            failures.push(format!(
                "{}:{} {}: complexity {own} exceeds {limit}",
                path.display(),
                space.start_line,
                space.name
            ));
        }
    }
    for child in &space.spaces {
        measure(child, path, limit, scores, failures);
    }
}

fn parse_check(source: &Path, analyzer: &Path) -> Result<()> {
    let errors = super::output(
        Command::new(analyzer)
            .arg("-p")
            .arg(source)
            .args(["-C", "ERROR"]),
    )?;
    let count = errors
        .lines()
        .find_map(|line| line.strip_prefix("Found nodes:"))
        .map(|value| value.trim().replace(',', ""))
        .context("Analyzer omitted parse-error count")?;
    ensure!(
        count.parse::<u64>()? == 0,
        "Analyzer cannot parse all Rust source"
    );
    Ok(())
}

pub fn check(source: &Path, analyzer: &Path, limit: u32) -> Result<()> {
    let source = source.canonicalize()?;
    let expected = files(&source, "rs")?;
    ensure!(!expected.is_empty(), "No Rust source files found");
    parse_check(&source, analyzer)?;
    let directory = tempfile::tempdir()?;
    super::output(
        Command::new(analyzer)
            .arg("-p")
            .arg(&source)
            .args(["-m", "-O", "json", "-o"])
            .arg(directory.path()),
    )?;
    let mut seen = BTreeSet::new();
    let mut scores = Vec::new();
    let mut failures = Vec::new();
    for file in files(directory.path(), "json")? {
        let space: Space = serde_json::from_slice(&fs::read(file)?)?;
        let path = Path::new(&space.name).canonicalize()?;
        ensure!(
            expected.contains(&path) && seen.insert(path.clone()),
            "Unexpected or duplicate analyzer result"
        );
        measure(&space, &path, limit, &mut scores, &mut failures);
    }
    ensure!(
        seen == expected && !scores.is_empty(),
        "Incomplete analyzer coverage"
    );
    ensure!(
        failures.is_empty(),
        "Cyclomatic complexity gate failed:\n{}",
        failures.join("\n")
    );
    let maximum = scores.iter().copied().fold(0.0_f64, f64::max);
    println!(
        "Complexity passed for {}: {} files, {} functions/closures, maximum {maximum}",
        source.display(),
        seen.len(),
        scores.len()
    );
    Ok(())
}
