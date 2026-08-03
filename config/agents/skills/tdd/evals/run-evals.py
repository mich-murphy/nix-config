#!/usr/bin/env python3
"""Run TDD's cross-harness evaluation package."""

from pathlib import Path
import sys

EVAL_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(EVAL_DIR.parents[2]))

from eval_runtime import main  # noqa: E402

if __name__ == "__main__":
    main(EVAL_DIR)
