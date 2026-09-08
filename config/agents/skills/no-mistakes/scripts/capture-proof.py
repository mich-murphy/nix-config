#!/usr/bin/env python3
"""Capture one authorized local proof command without interpreting its outcome."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def git(cwd, *args):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True).stdout


def snapshot(cwd, paths, output, label):
    files = {}
    for name in paths:
        path = cwd / name
        if path.is_symlink() or not path.resolve().is_relative_to(cwd):
            raise ValueError("proof path is redirected or outside the checkout")
        files[name] = hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None
    for suffix, args in [("status", ["status", "--porcelain=v1"]),
                         ("diff", ["diff", "--binary", "HEAD", "--", *paths])]:
        (output / f"{label}.{suffix}").write_bytes(git(cwd, *args))
    return {"head": git(cwd, "rev-parse", "HEAD").decode().strip(), "files": files}


def capture(args):
    cwd = args.cwd.resolve(strict=True)
    output = args.output.absolute()
    if output.exists() or output.parent.resolve() != output.parent:
        raise ValueError("use a new output directory with an existing unredirected parent")
    if any(Path(p).is_absolute() or ".." in Path(p).parts for p in args.path):
        raise ValueError("proof paths must be checkout relative")
    git(cwd, "check-ignore", "-q", str(output))
    if not args.command or args.command[0] != "--" or len(args.command) < 2:
        raise ValueError("supply -- followed by the actual command arguments")
    os.mkdir(output, 0o700)
    before = snapshot(cwd, args.path, output, "before")
    command = args.command[1:]
    # Write intent and baseline before launch so interruption retains evidence.
    receipt = {"argv": command, "cwd": str(cwd), "before": before, "started_ns": time.time_ns()}
    (output / "intent.json").write_text(json.dumps(receipt, indent=2) + "\n")
    started = time.monotonic_ns()
    with (output / "stdout.log").open("wb") as stdout, (output / "stderr.log").open("wb") as stderr:
        result = subprocess.run(command, cwd=cwd, stdout=stdout, stderr=stderr, check=False)
    receipt.update(exit_code=result.returncode, elapsed_ns=time.monotonic_ns() - started,
                   after=snapshot(cwd, args.path, output, "after"))
    receipt["log_sha256"] = {name: hashlib.sha256((output / name).read_bytes()).hexdigest()
                             for name in ["stdout.log", "stderr.log", "before.diff", "after.diff", "before.status", "after.status"]}
    (output / "receipt.tmp").write_text(json.dumps(receipt, indent=2) + "\n")
    (output / "receipt.tmp").replace(output / "receipt.json")
    print(json.dumps({"receipt": str(output / "receipt.json"), "exit_code": result.returncode}))
    return result.returncode if result.returncode >= 0 else 128 - result.returncode


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cwd", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--path", action="append", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    return capture(args)


if __name__ == "__main__":
    raise SystemExit(main())
