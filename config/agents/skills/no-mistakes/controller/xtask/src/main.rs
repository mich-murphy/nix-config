mod analyzer;
mod architecture;
mod complexity;

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
struct Policy {
    rust_version: String,
    max_cyclomatic_complexity: u32,
    max_file_lines: usize,
    complexity_dispatches: Vec<complexity::Dispatch>,
    analyzer: analyzer::Settings,
}

/// The controller directory under test. `quality.sh` names it in
/// `NO_MISTAKES_CONTROLLER`, because a cached xtask binary built from
/// another checkout of this workspace would otherwise carry that
/// checkout's `CARGO_MANIFEST_DIR` and silently gate the wrong tree.
fn controller() -> PathBuf {
    std::env::var_os("NO_MISTAKES_CONTROLLER").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("workspace parent")
                .to_owned()
        },
        PathBuf::from,
    )
}

fn cache() -> Result<PathBuf> {
    let home = match std::env::var_os("XDG_CACHE_HOME") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(std::env::var_os("HOME").context("HOME is required")?).join(".cache"),
    };
    Ok(home.join("no-mistakes"))
}

fn toolchain(cache: &Path, version: &str) -> Result<PathBuf> {
    let path = cache.join("toolchain");
    ensure!(
        path.join("bin/rustc").is_file(),
        "Pinned Rust {version} toolchain missing at {}; provisioning is a separate step",
        path.display()
    );
    Ok(path)
}

fn tool_path(toolchain: &Path, name: &str) -> Result<PathBuf> {
    let executable = toolchain.join("bin").join(name);
    ensure!(
        executable.is_file(),
        "Pinned Rust tool missing: {}",
        executable.display()
    );
    Ok(executable)
}

fn tool(toolchain: &Path, name: &str) -> Result<Command> {
    Ok(Command::new(tool_path(toolchain, name)?))
}

fn cargo_path(toolchain: &Path) -> Result<std::ffi::OsString> {
    let current = std::env::var_os("PATH").unwrap_or_default();
    Ok(std::env::join_paths(
        std::iter::once(toolchain.join("bin")).chain(std::env::split_paths(&current)),
    )?)
}

fn guard_invocation() -> Result<()> {
    let root = controller().canonicalize()?;
    let executable = std::env::current_exe()?;
    ensure!(
        !executable.ancestors().any(|path| path == root),
        "Source controller/env.sh before invoking Cargo, or run controller/quality.sh"
    );
    Ok(())
}

fn output(command: &mut Command) -> Result<String> {
    let result = command
        .output()
        .with_context(|| format!("starting {command:?}"))?;
    ensure!(
        result.status.success(),
        "{command:?} failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(String::from_utf8(result.stdout)?)
}

fn cargo(args: &[&str], analyzer: &Path, toolchain: &Path, target: &Path) -> Result<()> {
    println!("Running cargo {}", args.join(" "));
    let command = match args.first() {
        Some(&"fmt") => "cargo-fmt",
        Some(&"clippy") => "cargo-clippy",
        _ => "cargo",
    };
    let status = tool(toolchain, command)?
        .args(args)
        .current_dir(controller())
        .env("PATH", cargo_path(toolchain)?)
        .env("RUSTFMT", tool_path(toolchain, "rustfmt")?)
        .env("CLIPPY_DRIVER", tool_path(toolchain, "clippy-driver")?)
        .env("CARGO_TARGET_DIR", target)
        .env("CLIPPY_CONF_DIR", controller())
        .env("BATMAN_QUALITY_ANALYZER", analyzer)
        .status()?;
    ensure!(status.success(), "cargo {} failed", args.join(" "));
    Ok(())
}

#[derive(Default)]
struct Options {
    analyzer: Option<PathBuf>,
    source: Option<PathBuf>,
    diagnostic: bool,
}

fn options() -> Result<Options> {
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "quality" => {}
            "--analyzer" => {
                options.analyzer = Some(args.next().context("--analyzer needs a path")?.into())
            }
            "--source" => {
                options.source = Some(args.next().context("--source needs a path")?.into())
            }
            "--complexity-only" => options.diagnostic = true,
            _ => bail!("Unknown argument: {arg}"),
        }
    }
    ensure!(
        options.source.is_none() || options.diagnostic,
        "--source requires --complexity-only"
    );
    Ok(options)
}

fn run() -> Result<()> {
    guard_invocation()?;
    let options = options()?;
    let policy = policy()?;
    let cache = cache()?;
    let toolchain = toolchain(&cache, &policy.rust_version)?;
    let target = cache.join("target");
    let analyzer = analyzer::resolve(&policy.analyzer, options.analyzer.clone(), &cache)?;
    execute_mode(&policy, &options, &analyzer, &toolchain, &target)
}

fn execute_mode(
    policy: &Policy,
    options: &Options,
    analyzer: &Path,
    toolchain: &Path,
    target: &Path,
) -> Result<()> {
    if options.diagnostic {
        analyze(policy, options, analyzer)?;
        println!("Complexity diagnostic passed; this is not a full quality pass.");
        Ok(())
    } else {
        lint(policy, analyzer, toolchain, target)?;
        analyze(policy, options, analyzer)?;
        test_and_build(analyzer, toolchain, target)
    }
}

fn policy() -> Result<Policy> {
    let policy: Policy =
        serde_json::from_slice(&std::fs::read(controller().join("quality-gates.json"))?)?;
    ensure!(
        policy.max_cyclomatic_complexity > 0,
        "Complexity limit must be positive"
    );
    Ok(policy)
}

fn lint(policy: &Policy, analyzer: &Path, toolchain: &Path, target: &Path) -> Result<()> {
    let rust = output(tool(toolchain, "rustc")?.arg("--version"))?;
    ensure!(
        rust.split_whitespace().nth(1) == Some(&policy.rust_version),
        "Use Rust {}; found {}",
        policy.rust_version,
        rust.trim()
    );
    cargo(&["fmt", "--all", "--check"], analyzer, toolchain, target)?;
    cargo(
        &[
            "clippy",
            "--workspace",
            "--locked",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        analyzer,
        toolchain,
        target,
    )
}

fn analyze(policy: &Policy, options: &Options, analyzer: &Path) -> Result<()> {
    let sources = options.source.as_ref().map_or_else(
        || vec![controller().join("crates"), controller().join("xtask/src")],
        |source| vec![source.clone()],
    );
    for source in &sources {
        complexity::check(
            source,
            analyzer,
            policy.max_cyclomatic_complexity,
            &policy.complexity_dispatches,
        )?;
    }
    architecture::check(&controller().join("crates"), policy.max_file_lines)
}

fn test_and_build(analyzer: &Path, toolchain: &Path, target: &Path) -> Result<()> {
    cargo(
        &[
            "test",
            "--workspace",
            "--locked",
            "--all-targets",
            "--all-features",
        ],
        analyzer,
        toolchain,
        target,
    )?;
    cargo(
        &["test", "--workspace", "--locked", "--doc", "--all-features"],
        analyzer,
        toolchain,
        target,
    )?;
    cargo(
        &["build", "--release", "--locked"],
        analyzer,
        toolchain,
        target,
    )?;
    println!(
        "Rust quality gates passed. Behavior coverage and design judgment still require the documented review."
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Rust quality gate failed: {error:#}");
        std::process::exit(1);
    }
}
