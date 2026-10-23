#!/usr/bin/env python3
"""Rename one of our own crates, by an explicit table, never by pattern (#17 task 19).

    python scripts/rename_crate.py OLD NEW [--dry-run] [--no-lock]

OLD and NEW must be a pair of TABLE. A crate that belongs to another repository (for example
`edge-transport-retry`, `edge-transport-breaker-policy`, `edge-security-*`, `swe-edge-*`) is
refused, because it is not in the table.

What it changes, in tracked files of the repository only:
  * `name = "OLD"` in the Cargo.toml of the crate, and the dependency key or `package = "OLD"` in
    every Cargo.toml that names it (including `[workspace.dependencies]`);
  * the library identifier of OLD (`-` as `_`) in .rs files, and both spellings in docs, scripts and
    metadata;
  * Cargo.lock, by asking cargo to refresh it (offline when possible; skip with --no-lock).
It fails, after the run, when OLD still appears in a tracked file other than a CHANGELOG, which is
the one place that records a rename.
"""
import argparse
import os
import re
import subprocess
import sys

# Our five crates, old package name to new package name. The new name is the implementation
# crate; standard implementations use `-std`, while breaker and resilient are SPI-only and use
# `-spi`. The domain crate of a theme takes the name without the suffix and is created, not renamed.
# Nothing else may be renamed by this script.
TABLE = {
    "edge-transport-grpc-egress-transport": "grpc-egress-transport-std",
    "edge-transport-grpc-egress-retry": "grpc-egress-retry-std",
    "edge-transport-grpc-egress-breaker": "grpc-egress-breaker-spi",
    "edge-transport-grpc-egress-resilient": "grpc-egress-resilient-spi",
    "edge-transport-grpc-egress-auth-bearer": "grpc-egress-auth-bearer-std",
}

# The script and its tests name the crates on purpose (the table, the fixtures); they are never rewritten.
SELF_FILES = ("scripts/rename_crate.py", "scripts/tests/test_rename_crate.py")

TEXT_SUFFIXES = (".rs", ".toml", ".md", ".py", ".sh", ".ps1", ".yml", ".yaml", ".json", ".txt")


class CrateRenamer:
    """Renames one crate in the tracked files of a repository."""

    def __init__(self, root, old, new, dry_run=False, refresh_lock=True):
        self.root = os.path.abspath(root)
        self.old = old
        self.new = new
        self.dry_run = dry_run
        self.refresh_lock = refresh_lock
        self.old_ident = old.replace("-", "_")
        self.new_ident = new.replace("-", "_")
        # An exact token: `edge-transport-grpc-egress-retry` must not match inside a longer name.
        self.name_pattern = re.compile(r"(?<![A-Za-z0-9_-])%s(?![A-Za-z0-9_-])" % re.escape(old))
        self.ident_pattern = re.compile(r"(?<![A-Za-z0-9_-])%s(?![A-Za-z0-9_-])" % re.escape(self.old_ident))

    def check_pair(self):
        """Refuse any pair that is not in the table."""
        if TABLE.get(self.old) != self.new:
            known = ", ".join("%s -> %s" % kv for kv in TABLE.items())
            raise SystemExit("refused: '%s' -> '%s' is not in the rename table (%s)" % (self.old, self.new, known))

    def list_tracked_files(self):
        out = subprocess.run(["git", "-C", self.root, "ls-files"], check=True, capture_output=True, text=True)
        return [p for p in out.stdout.splitlines()
                if p not in SELF_FILES and (p.endswith(TEXT_SUFFIXES) or os.path.basename(p) == "Cargo.lock")]

    def plan_changes(self):
        """Return {path: (original text, new text, [changed line numbers])} for every file that changes."""
        changes = {}
        for rel in self.list_tracked_files():
            name = os.path.basename(rel)
            if name == "Cargo.lock" or name.upper().startswith("CHANGELOG"):
                continue  # the lock is refreshed by cargo; a CHANGELOG is the record of the old name
            path = os.path.join(self.root, rel)
            if not os.path.isfile(path):
                continue
            raw = open(path, "rb").read().decode("utf-8")
            new_text, lines = self.rewrite(raw)
            if new_text != raw:
                changes[rel] = (raw, new_text, lines)
        return changes

    def rewrite(self, text):
        out, changed = [], []
        for number, line in enumerate(text.split("\n"), start=1):
            rewritten = self.name_pattern.sub(self.new, line)
            rewritten = self.ident_pattern.sub(self.new_ident, rewritten)
            if rewritten != line:
                changed.append(number)
            out.append(rewritten)
        return "\n".join(out), changed

    def apply(self, changes):
        for rel, (_, new_text, _) in changes.items():
            open(os.path.join(self.root, rel), "wb").write(new_text.encode("utf-8"))

    def find_leftovers(self):
        """Lines still naming the old crate, outside a CHANGELOG."""
        left = []
        for rel in self.list_tracked_files():
            if os.path.basename(rel).upper().startswith("CHANGELOG") or os.path.basename(rel) == "Cargo.lock":
                continue
            path = os.path.join(self.root, rel)
            if not os.path.isfile(path):
                continue
            for number, line in enumerate(open(path, "rb").read().decode("utf-8").split("\n"), start=1):
                if self.name_pattern.search(line) or self.ident_pattern.search(line):
                    left.append("%s:%d: %s" % (rel, number, line.strip()))
        return left

    def refresh_cargo_lock(self):
        for manifest in ("scm/Cargo.toml", "Cargo.toml"):
            if os.path.isfile(os.path.join(self.root, manifest)):
                base = ["cargo", "metadata", "--format-version", "1", "--manifest-path", manifest]
                # Offline first; metadata also wants crates of other platforms, so fall back to online.
                if subprocess.run(base + ["--offline"], cwd=self.root, capture_output=True).returncode != 0:
                    subprocess.run(base, cwd=self.root, check=True, capture_output=True)
                return

    def run(self):
        """Do the rename; return the process exit code."""
        self.check_pair()
        changes = self.plan_changes()
        if not changes:
            print("nothing to change: '%s' does not appear in any tracked file" % self.old)
            return 0
        for rel, (_, _, lines) in sorted(changes.items()):
            print("%s%s: lines %s" % ("would change " if self.dry_run else "changed ", rel, ", ".join(map(str, lines))))
        if self.dry_run:
            return 0
        self.apply(changes)
        if self.refresh_lock:
            self.refresh_cargo_lock()
        leftovers = self.find_leftovers()
        if leftovers:
            print("FAILED: the old name is still in tracked files:", file=sys.stderr)
            for entry in leftovers:
                print("  " + entry, file=sys.stderr)
            return 1
        print("renamed '%s' to '%s' in %d files" % (self.old, self.new, len(changes)))
        return 0


class RenameCommand:
    """Command line of the script."""

    @staticmethod
    def main(argv):
        parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
        parser.add_argument("old")
        parser.add_argument("new")
        parser.add_argument("--dry-run", action="store_true", help="print the files and lines, change nothing")
        parser.add_argument("--no-lock", action="store_true", help="do not refresh Cargo.lock")
        parser.add_argument("--root", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
        args = parser.parse_args(argv)
        return CrateRenamer(args.root, args.old, args.new, args.dry_run, not args.no_lock).run()


if __name__ == "__main__":
    sys.exit(RenameCommand.main(sys.argv[1:]))
