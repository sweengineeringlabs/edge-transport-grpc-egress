#!/usr/bin/env python3
"""Hand-made mutation check: break one thing at a time and see whether a test notices.

For every mutant in a JSON list the runner replaces one piece of source text, runs the test command,
restores the file byte for byte, and classifies the result:

  CAUGHT    at least one named test failed. The names are printed.
  SURVIVED  the whole suite passed with the breakage in place: a test is missing.
  INVALID   the mutant did not compile (or the run was cut short). It proves nothing; fix the mutant.

A mutant listed with "expected": "survives" and a "reason" is allowed to survive; any other survivor,
and every invalid mutant, makes the exit code non-zero. A compile error is never counted as a catch:
only a failing test is.

Usage (from anywhere):
  python scripts/mutation/run_mutants.py --mutants scripts/mutation/mutants.json \
      [--cmd "cargo test --workspace --no-fail-fast"] [--target-dir DIR] [--only M1 M7] [--min-free-gb 5]

Each mutant rebuilds the crate under test, so a run is slow and fills the build directory: pass
--target-dir to use a directory of its own and delete it afterwards (a shared one filled a disk
once, and every later mutant then "failed to compile").
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional

DEFAULT_COMMAND = "cargo test --workspace --no-fail-fast"
DEFAULT_MIN_FREE_GB = 5.0
OUT_OF_SPACE_MARKERS = ("not enough space", "no space left", "os error 112", "os error 28")


@dataclass
class Mutant:
    """One breakage: `find` must occur in `file` and is replaced by `replace`."""

    name: str
    file: str
    find: str
    replace: str
    expected: str = "caught"
    reason: str = ""


@dataclass
class Outcome:
    """What running one mutant showed."""

    mutant: Mutant
    status: str
    failing_tests: List[str]
    seconds: float
    note: str = ""

    def is_acceptable(self) -> bool:
        if self.status == "CAUGHT":
            return self.mutant.expected == "caught"
        if self.status == "SURVIVED":
            return self.mutant.expected == "survives"
        return False


class MutantRunner:
    """Runs a list of mutants against one repository."""

    def __init__(self, root: Path, command: str, target_dir: Optional[Path], min_free_gb: float):
        self.root = root
        self.command = command
        self.min_free_gb = min_free_gb
        self.env = dict(os.environ)
        if target_dir is not None:
            self.env["CARGO_TARGET_DIR"] = str(target_dir)
        self.disk_path = target_dir if target_dir is not None else root

    @staticmethod
    def load_mutants(path: Path) -> List[Mutant]:
        entries = json.loads(path.read_text(encoding="utf-8"))
        return [Mutant(**entry) for entry in entries]

    def check_free_space(self) -> None:
        free_gb = shutil.disk_usage(self.disk_path if self.disk_path.exists() else self.root).free / 1e9
        if free_gb < self.min_free_gb:
            raise SystemExit(
                f"only {free_gb:.1f} GB free; a run needs at least {self.min_free_gb} GB "
                "(each mutant leaves build output). Free space or pass --min-free-gb."
            )

    def run_suite(self) -> subprocess.CompletedProcess:
        return subprocess.run(
            self.command,
            cwd=self.root,
            shell=True,
            capture_output=True,
            text=True,
            env=self.env,
        )

    @staticmethod
    def failing_tests(output: str) -> List[str]:
        names = []
        for line in output.splitlines():
            match = re.match(r"^test (\S+) \.\.\. FAILED$", line.strip())
            if match:
                names.append(match.group(1))
        return names

    @staticmethod
    def adapt_line_endings(text: str, original: bytes) -> str:
        return text.replace("\n", "\r\n") if b"\r\n" in original else text

    def run_mutant(self, mutant: Mutant) -> Outcome:
        path = self.root / mutant.file
        original = path.read_bytes()
        text = original.decode("utf-8")
        find = self.adapt_line_endings(mutant.find, original)
        replace = self.adapt_line_endings(mutant.replace, original)
        started = time.time()
        if text.count(find) != 1:
            return Outcome(mutant, "INVALID", [], 0.0, f"`find` occurs {text.count(find)} times in {mutant.file}, not once")
        try:
            path.write_bytes(text.replace(find, replace, 1).encode("utf-8"))
            result = self.run_suite()
        finally:
            path.write_bytes(original)
            os.utime(path, None)
        if path.read_bytes() != original:
            raise SystemExit(f"{mutant.file} was not restored byte for byte: stop and check it with git diff")
        output = result.stdout + result.stderr
        elapsed = time.time() - started
        if any(marker in output.lower() for marker in OUT_OF_SPACE_MARKERS):
            return Outcome(mutant, "INVALID", [], elapsed, "the disk filled up during the run")
        failing = self.failing_tests(output)
        if failing:
            return Outcome(mutant, "CAUGHT", failing, elapsed)
        if "could not compile" in output or re.search(r"^error(\[|:)", output, re.MULTILINE):
            return Outcome(mutant, "INVALID", [], elapsed, "the mutant did not compile")
        if result.returncode != 0:
            return Outcome(mutant, "INVALID", [], elapsed, f"the command failed (exit {result.returncode}) without a failing test")
        return Outcome(mutant, "SURVIVED", [], elapsed)

    def run_all(self, mutants: List[Mutant]) -> List[Outcome]:
        self.check_free_space()
        print("baseline: the unmutated suite must pass ...", flush=True)
        baseline = self.run_suite()
        if baseline.returncode != 0:
            print((baseline.stdout + baseline.stderr)[-3000:])
            raise SystemExit("the unmutated suite fails: nothing a mutant does can be told from that")
        outcomes = []
        for mutant in mutants:
            self.check_free_space()
            outcome = self.run_mutant(mutant)
            outcomes.append(outcome)
            tests = ", ".join(outcome.failing_tests[:3]) + (" ..." if len(outcome.failing_tests) > 3 else "")
            detail = tests or outcome.note or mutant.reason
            mark = "ok " if outcome.is_acceptable() else "BAD"
            print(f"[{mark}] {mutant.name}: {outcome.status} ({len(outcome.failing_tests)} failing) {detail} [{outcome.seconds:.0f}s]", flush=True)
        return outcomes


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--mutants", required=True, type=Path, help="JSON list of mutants")
    parser.add_argument("--root", type=Path, default=None, help="repository root (default: two levels above this script)")
    parser.add_argument("--cmd", default=DEFAULT_COMMAND, help=f"test command (default: {DEFAULT_COMMAND})")
    parser.add_argument("--target-dir", type=Path, default=None, help="CARGO_TARGET_DIR for the run")
    parser.add_argument("--only", nargs="*", default=None, help="run only the mutants whose name starts with one of these")
    parser.add_argument("--min-free-gb", type=float, default=DEFAULT_MIN_FREE_GB)
    args = parser.parse_args()

    root = args.root or Path(__file__).resolve().parents[2]
    mutants = MutantRunner.load_mutants(args.mutants)
    if args.only:
        mutants = [m for m in mutants if any(m.name.startswith(prefix) for prefix in args.only)]
    runner = MutantRunner(root, args.cmd, args.target_dir, args.min_free_gb)
    outcomes = runner.run_all(mutants)

    caught = sum(1 for o in outcomes if o.status == "CAUGHT")
    expected = sum(1 for o in outcomes if o.status == "SURVIVED" and o.mutant.expected == "survives")
    bad = [o for o in outcomes if not o.is_acceptable()]
    print(f"\n{caught} caught, {expected} expected to survive, {len(bad)} to fix, of {len(outcomes)}")
    for outcome in bad:
        print(f"  fix: {outcome.mutant.name}: {outcome.status} {outcome.note}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
