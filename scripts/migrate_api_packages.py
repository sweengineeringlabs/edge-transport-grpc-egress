#!/usr/bin/env python3
"""Plan or apply the reviewed contract-package rename and src/api flattening.

The default mode is read-only. The repository is modified only with --apply,
which refuses dirty worktrees unless --allow-dirty is explicit. The package table below is explicit;
the script refuses missing packages, destination collisions, and unexpected
src/api module declarations instead of guessing.

    python scripts/migrate_api_packages.py
    python scripts/migrate_api_packages.py --apply
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


PACKAGE_MAP = {
    "auth-bearer": ("grpc-egress-auth-bearer", "grpc-egress-auth-bearer-api"),
    "breaker": ("grpc-egress-breaker", "grpc-egress-breaker-api"),
    "resilient": ("grpc-egress-resilient", "grpc-egress-resilient-api"),
    "retry": ("grpc-egress-retry", "grpc-egress-retry-api"),
    "transport": ("grpc-egress-transport", "grpc-egress-transport-api"),
}
TEXT_SUFFIXES = {
    ".c", ".h", ".html", ".json", ".md", ".ps1", ".py", ".rs", ".sh",
    ".toml", ".txt", ".xml", ".yml", ".yaml",
}
SCRIPT_PATH = "scripts/migrate_api_packages.py"
EXCLUDED_PATHS = {
    "scripts/tests/test_migrate_api_packages.py",
    "scripts/rename_crate.py",
    "scripts/tests/test_rename_crate.py",
    "scripts/draft_std_spi_split.py",
    "scripts/tests/test_draft_std_spi_split.py",
}


class MigrationError(RuntimeError):
    """The source tree is incomplete, ambiguous, or unsafe to migrate."""


@dataclass(frozen=True)
class PlannedFile:
    source: Path
    destination: Path | None
    content: bytes | None


def git(root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, encoding="utf-8"
    )
    if check and result.returncode:
        raise MigrationError(result.stderr.strip() or "git command failed")
    return result


def tracked_files(root: Path) -> list[str]:
    return sorted(
        path for path in git(root, "ls-files").stdout.splitlines()
        if path not in EXCLUDED_PATHS | {SCRIPT_PATH}
    )


def root_modules(api_mod: str, package: str) -> str:
    declarations = re.findall(r"^pub\(crate\)\s+mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;\s*$", api_mod, re.M)
    if not declarations:
        raise MigrationError(f"{package}/src/api/mod.rs has no recognized module declarations")
    recognized = re.sub(r"(?m)^\s*//!.*$", "", api_mod)
    recognized = re.sub(r"(?m)^\s*pub\(crate\)\s+mod\s+[A-Za-z_][A-Za-z0-9_]*\s*;\s*$", "", recognized)
    if recognized.strip():
        raise MigrationError(f"{package}/src/api/mod.rs contains declarations the tool cannot migrate")
    return "\n".join(f"mod {name};" for name in declarations)


def rewrite_text(text: str, package_dir: str, old_name: str, new_name: str, in_api_package: bool) -> str:
    old_ident, new_ident = old_name.replace("-", "_"), new_name.replace("-", "_")

    def token(value: str, replacement: str) -> str:
        return re.sub(rf"(?<![A-Za-z0-9_-]){re.escape(value)}(?![A-Za-z0-9_-])", replacement, text)

    text = token(old_name, new_name)
    # Recompute the identifier replacement from the already package-renamed text.
    text = re.sub(rf"(?<![A-Za-z0-9_]){re.escape(old_ident)}(?![A-Za-z0-9_])", new_ident, text)
    # Rename directory path segments only. A plain word such as "breaker" in
    # prose or an unrelated dependency name must never become "breaker-api".
    text = re.sub(
        rf"(?<=/){re.escape(package_dir)}(?=[/\\\"'`])",
        package_dir + "-api",
        text,
    )
    if in_api_package:
        text = text.replace("crate::api::", "crate::")
    text = re.sub(rf"({re.escape(new_ident)})::api::", r"\1::", text)
    text = re.sub(rf"({re.escape(new_name)})::api::", r"\1::", text)
    if in_api_package:
        text = re.sub(r"(?m)^\s*pub use api::", "pub use ", text)
    return text


def plan_migration(root: Path) -> list[PlannedFile]:
    files = tracked_files(root)
    if not files:
        raise MigrationError("no tracked files found; run from a Git worktree")

    active, completed = [], []
    for directory, (old_name, new_name) in PACKAGE_MAP.items():
        old_dir = f"scm/main/{directory}"
        new_dir = f"{old_dir}-api"
        old_manifest, new_manifest = root / old_dir / "Cargo.toml", root / new_dir / "Cargo.toml"
        if old_manifest.is_file():
            active.append((directory, old_name, new_name, old_dir, new_dir))
        elif new_manifest.is_file():
            completed.append(directory)
        else:
            raise MigrationError(f"neither source nor target API package exists for {directory}")
    if not active:
        return []
    if completed:
        raise MigrationError("partial migration detected; packages already migrated: " + ", ".join(completed))

    module_declarations = {}
    for directory, _, _, old_dir, _ in active:
        api_mod = root / old_dir / "src/api/mod.rs"
        lib = root / old_dir / "src/lib.rs"
        if not api_mod.is_file() or not lib.is_file():
            raise MigrationError(f"expected {old_dir}/src/api/mod.rs and src/lib.rs")
        declarations = root_modules(api_mod.read_text(encoding="utf-8"), directory)
        lib_text = lib.read_text(encoding="utf-8")
        if len(re.findall(r"(?m)^mod api;\s*$", lib_text)) != 1:
            raise MigrationError(f"{old_dir}/src/lib.rs must contain exactly one 'mod api;' line")
        module_declarations[directory] = declarations

    planned: list[PlannedFile] = []
    source_paths = {root / path for path in files}
    destination_paths: set[Path] = set()
    package_lookup = {directory: (old_name, new_name, old_dir, new_dir) for directory, old_name, new_name, old_dir, new_dir in active}

    for relative in files:
        source = root / relative
        destination = source
        package = next((entry for directory, entry in package_lookup.items()
                        if relative == entry[2] or relative.startswith(entry[2] + "/")), None)
        if package:
            old_name, new_name, old_dir, new_dir = package
            suffix = relative[len(old_dir):]
            if suffix == "/src/api/mod.rs":
                planned.append(PlannedFile(source, None, None))
                continue
            if suffix.startswith("/src/api/"):
                suffix = "/src/" + suffix[len("/src/api/"):]
            destination = root / (new_dir + suffix)

        try:
            raw = source.read_bytes()
        except FileNotFoundError:
            raise MigrationError(f"tracked file is missing: {relative}") from None
        if source.suffix.lower() in TEXT_SUFFIXES:
            try:
                content = raw.decode("utf-8")
            except UnicodeDecodeError:
                content = None
            if content is not None:
                for directory, (old_name, new_name, old_dir, _) in package_lookup.items():
                    content = rewrite_text(content, old_dir.removeprefix("scm/main/"), old_name, new_name, bool(package))
                    if relative == f"{old_dir}/src/lib.rs":
                        content = re.sub(r"(?m)^mod api;\s*$", module_declarations[directory], content)
                raw = content.encode("utf-8")
        if destination in destination_paths:
            raise MigrationError(f"multiple sources map to {destination.relative_to(root).as_posix()}")
        destination_paths.add(destination)
        if destination != source and destination.exists() and destination not in source_paths:
            raise MigrationError(f"destination already exists and is not part of this migration: {destination}")
        planned.append(PlannedFile(source, destination, raw))

    return sorted(planned, key=lambda item: item.source.as_posix())


def apply_plan(
    root: Path,
    plan: list[PlannedFile],
    refresh_lock: bool = True,
    allow_dirty: bool = False,
) -> None:
    status = git(root, "status", "--porcelain").stdout
    if status and not allow_dirty:
        raise MigrationError("refusing --apply on a dirty worktree; commit or otherwise resolve changes first")
    if not plan:
        print("already migrated: no changes")
        return

    originals = {item.source: item.source.read_bytes() for item in plan}
    lockfile = root / "scm/Cargo.lock"
    lock_existed = lockfile.exists()
    if refresh_lock and lockfile.exists():
        originals[lockfile] = lockfile.read_bytes()
    targets = {item.destination for item in plan if item.destination is not None}
    try:
        for item in plan:
            if item.destination is None:
                continue
            item.destination.parent.mkdir(parents=True, exist_ok=True)
            item.destination.write_bytes(item.content or b"")
        for item in plan:
            if item.destination is None or item.destination != item.source:
                item.source.unlink(missing_ok=True)
        for name in PACKAGE_MAP:
            old = root / f"scm/main/{name}"
            if old.exists():
                for child in sorted(old.rglob("*"), key=lambda item: len(item.parts), reverse=True):
                    if child.is_dir():
                        try:
                            child.rmdir()
                        except OSError:
                            pass
                try:
                    old.rmdir()
                except OSError:
                    raise MigrationError(f"untracked or ignored files remain in source package {old}") from None
        if refresh_lock and (root / "scm/Cargo.toml").is_file():
            result = subprocess.run(
                ["cargo", "metadata", "--format-version", "1", "--manifest-path", "scm/Cargo.toml", "--offline"],
                cwd=root, capture_output=True, text=True,
            )
            if result.returncode:
                raise MigrationError("cargo metadata could not refresh Cargo.lock: " + result.stderr.strip())
        for directory in PACKAGE_MAP:
            api_root = root / f"scm/main/{directory}-api/src/api"
            if api_root.exists():
                raise MigrationError(f"src/api remains after migration: {api_root}")
    except Exception:
        for target in targets:
            if target.exists() and target not in originals:
                target.unlink()
        if refresh_lock and not lock_existed:
            lockfile.unlink(missing_ok=True)
        for source, content in originals.items():
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_bytes(content)
        raise


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--apply", action="store_true", help="apply the reviewed package/source migration")
    parser.add_argument(
        "--allow-dirty", action="store_true",
        help="preserve current tracked edits and unrelated files while applying; default refuses dirty worktrees",
    )
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        plan = plan_migration(root)
        for item in plan:
            source = item.source.relative_to(root).as_posix()
            if item.destination is None:
                print(("remove " if args.apply else "would remove ") + source)
            elif item.destination != item.source:
                print(("move " if args.apply else "would move ") + source + " -> " + item.destination.relative_to(root).as_posix())
            elif item.content != item.source.read_bytes():
                print(("rewrite " if args.apply else "would rewrite ") + source)
        if args.apply:
            apply_plan(root, plan, allow_dirty=args.allow_dirty)
            print("API package migration applied")
        else:
            print("read-only plan; pass --apply to make changes")
        return 0
    except MigrationError as error:
        print(f"refused: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
