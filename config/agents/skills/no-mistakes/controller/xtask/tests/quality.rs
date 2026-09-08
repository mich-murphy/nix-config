// Exercise observable CLI outcomes using real Rust input and the pinned analyzer.
use std::process::{Command, Output};

fn scan(source: Option<&str>) -> Output {
    let directory = tempfile::Builder::new()
        .prefix("batman-quality-test-")
        .tempdir()
        .unwrap();
    if let Some(source) = source {
        std::fs::write(directory.path().join("lib.rs"), source).unwrap();
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .args(["quality", "--complexity-only", "--source"])
        .arg(directory.path());
    if let Some(analyzer) = std::env::var_os("BATMAN_QUALITY_ANALYZER") {
        command.arg("--analyzer").arg(analyzer);
    }
    command.output().unwrap()
}

fn branches(count: u32) -> String {
    (0..count)
        .map(|n| format!("if x == {n} {{ y += 1; }}\n"))
        .collect()
}

fn rejected(source: Option<&str>, reason: &str) {
    let result = scan(source);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains(reason),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn comments_and_strings_do_not_count_as_branches() {
    let result = scan(Some(
        "pub fn value() -> &'static str { /* if match while && || */ \"if match while\" }",
    ));
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn threshold_is_inclusive() {
    let code = format!(
        "pub fn value(x:u32)->u32 {{ let mut y=0; {} y }}",
        branches(19)
    );
    let result = scan(Some(&code));
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn excessive_function_complexity_fails() {
    let code = format!(
        "pub fn value(x:u32)->u32 {{ let mut y=0; {} y }}",
        branches(20)
    );
    rejected(Some(&code), "Cyclomatic complexity gate failed");
}

#[test]
fn excessive_closure_complexity_cannot_hide_in_parent() {
    let code = format!(
        "pub fn value(x:u32)->u32 {{ let f=|x| {{ let mut y=0; {} y }}; f(x) }}",
        branches(20)
    );
    rejected(Some(&code), "Cyclomatic complexity gate failed");
}

#[test]
fn malformed_rust_fails() {
    rejected(Some("pub fn broken( {"), "cannot parse");
}

#[test]
fn empty_scope_fails() {
    rejected(None, "No Rust source");
}
