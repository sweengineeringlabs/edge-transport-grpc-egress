#!/usr/bin/env python3
"""Plan and safely apply an explicitly reviewed std/SPI package migration.

Without ``--mapping``, this prints a deterministic, read-only package inventory.
With a reviewed mapping, it prints the exact moves and file writes. The
repository is changed only when ``--apply`` is supplied.

    python scripts/draft_std_spi_split.py [--root REPO_ROOT]
    python scripts/draft_std_spi_split.py --mapping split-map.json
    python scripts/draft_std_spi_split.py --mapping split-map.json --apply

Mapping format: ``reviewed`` must be true; ``source_commit`` must be the clean
Git revision used to prepare it; ``package_paths`` maps crate names to
workspace-relative directories; ``ownership`` maps every inventoried artifact
path to ``{"package": name, "sha256": source_hash}``; and ``writes`` contains
explicit ``{"path": ..., "expected_sha256": hash_or_null, "content": text_or_null}``
updates. A null expected hash means the file must not yet exist; null content
means a reviewed deletion. Package files are moved while preserving their path
relative to the source package directory. Source files are copied and verified
before any originals are removed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
import tomllib
from pathlib import Path


LAYERS = ("std", "spi", "saf")
DEPENDENCY_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")


class MappingError(ValueError):
    """The reviewed migration map is incomplete, ambiguous, or stale."""


def load_toml(path: Path) -> dict:
    with path.open("rb") as stream:
        return tomllib.load(stream)


def package_inventory(root: Path, main: Path, package_dir: Path, all_manifests: list[Path]) -> dict:
    manifest_path = package_dir / "Cargo.toml"
    manifest = load_toml(manifest_path)
    package = manifest.get("package", {})
    dependencies = {}
    for section in DEPENDENCY_SECTIONS:
        names = sorted(manifest.get(section, {}))
        if names:
            dependencies[section] = names

    source = package_dir / "src"
    files_by_layer = {}
    for layer in LAYERS:
        layer_root = source / layer
        files_by_layer[layer] = (
            sorted(path.relative_to(root).as_posix() for path in layer_root.rglob("*.rs"))
            if layer_root.is_dir()
            else []
        )

    validation_files = {}
    for kind in ("tests", "examples", "benches"):
        kind_root = package_dir / kind
        validation_files[kind] = (
            sorted(path.relative_to(root).as_posix() for path in kind_root.rglob("*.rs"))
            if kind_root.is_dir()
            else []
        )

    owned_artifacts = [
        {
            "path": path.relative_to(root).as_posix(),
            "sha256": sha256_file(path),
        }
        for path in sorted(package_dir.rglob("*"))
        if path.is_file() and path.name != "Cargo.toml"
    ]

    crate_name = package.get("name", package_dir.name)
    domain_name = crate_name.removesuffix("-std").removesuffix("-spi")
    current_layer = "spi" if crate_name.endswith("-spi") else "std"
    package_crates = []
    for sibling in sorted(main.iterdir()):
        sibling_manifest = sibling / "Cargo.toml"
        if not sibling.is_dir() or not sibling_manifest.is_file():
            continue
        sibling_name = load_toml(sibling_manifest).get("package", {}).get("name", "")
        if sibling_name == domain_name or sibling_name in (f"{domain_name}-std", f"{domain_name}-spi"):
            package_crates.append(sibling_name)

    consumers = []
    for consumer_manifest in all_manifests:
        if consumer_manifest == manifest_path:
            continue
        consumer = load_toml(consumer_manifest)
        consumer_name = consumer.get("package", {}).get("name", consumer_manifest.parent.name)
        for section in DEPENDENCY_SECTIONS:
            for dependency, value in consumer.get(section, {}).items():
                dependency_package = value.get("package", dependency) if isinstance(value, dict) else dependency
                if dependency_package == crate_name:
                    consumers.append({
                        "package": consumer_name,
                        "manifest": consumer_manifest.relative_to(root).as_posix(),
                        "dependency_section": section,
                    })
    consumers.sort(key=lambda item: (item["package"], item["dependency_section"], item["manifest"]))

    rust_sources = [path for kind in ("src", "tests", "examples", "benches")
                    for path in (package_dir / kind).rglob("*.rs") if (package_dir / kind).is_dir()]
    source_text = "\n".join(path.read_text(encoding="utf-8", errors="replace") for path in rust_sources)
    referenced = {}
    for section in DEPENDENCY_SECTIONS:
        names = sorted(manifest.get(section, {}))
        used = [name for name in names if re.search(rf"\b{re.escape(name.replace('-', '_'))}\s*::", source_text)]
        if used:
            referenced[section] = used
    return {
        "path": package_dir.relative_to(root).as_posix(),
        "package": crate_name,
        "current_layer": current_layer,
        "related_packages": package_crates,
        "domain_package": domain_name,
        "layers": files_by_layer,
        "unclassified_src_files": sorted(
            path.relative_to(root).as_posix()
            for path in source.rglob("*.rs")
            if not any(part in LAYERS for part in path.relative_to(source).parts)
        ),
        "validation_files": validation_files,
        "owned_artifacts": owned_artifacts,
        "declared_dependencies": dependencies,
        "source_referenced_dependencies": referenced,
        "workspace_consumers": consumers,
        "requires_saf_ownership_map": bool(files_by_layer["saf"]),
    }


def build_inventory(root: Path) -> dict:
    main = root / "scm" / "main"
    manifests = sorted(main.rglob("Cargo.toml"))
    packages = [
        package_inventory(root, main, path, manifests)
        for path in sorted(main.iterdir())
        if path.is_dir()
        and (path.name.endswith("-std") or path.name.endswith("-spi"))
        and (path / "Cargo.toml").is_file()
    ]
    return {
        "issue": 63,
        "mode": "read-only-inventory",
        "workspace": "scm/Cargo.toml",
        "package_count": len(packages),
        "packages": packages,
        "next_step": (
            "Review std/spi source ownership, saf factory ownership, and dependency ownership; "
            "then encode that reviewed map before enabling any filesystem or manifest changes."
        ),
    }


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def repo_relative(root: Path, raw: object, field: str) -> Path:
    if not isinstance(raw, str) or not raw:
        raise MappingError(f"{field} must be a non-empty repository-relative path")
    candidate = Path(raw)
    if candidate.is_absolute() or any(part in ("..", "") for part in candidate.parts):
        raise MappingError(f"{field} escapes the repository: {raw!r}")
    resolved = (root / candidate).resolve()
    try:
        resolved.relative_to(root.resolve())
    except ValueError as exc:
        raise MappingError(f"{field} escapes the repository: {raw!r}") from exc
    return resolved


def load_mapping(path: Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise MappingError(f"cannot read mapping {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise MappingError("mapping root must be a JSON object")
    if value.get("reviewed") is not True:
        raise MappingError("mapping must set reviewed to true after ownership review")
    for key in ("source_commit", "package_paths", "ownership", "writes"):
        if key not in value:
            raise MappingError(f"mapping is missing required field {key!r}")
    if not isinstance(value["source_commit"], str) or not value["source_commit"]:
        raise MappingError("source_commit must be a non-empty Git revision")
    if not isinstance(value["package_paths"], dict) or not value["package_paths"]:
        raise MappingError("package_paths must map every destination package to a directory")
    if not isinstance(value["ownership"], dict):
        raise MappingError("ownership must map artifact paths to package names and checksums")
    if not isinstance(value["writes"], list):
        raise MappingError("writes must be a list")
    return value


def build_migration_plan(root: Path, mapping: dict) -> dict:
    inventory = build_inventory(root)
    expected_artifacts = {
        artifact["path"]: artifact["sha256"]
        for package in inventory["packages"]
        for artifact in package["owned_artifacts"]
    }
    ownership = mapping["ownership"]
    if set(ownership) != set(expected_artifacts):
        missing = sorted(set(expected_artifacts) - set(ownership))
        extra = sorted(set(ownership) - set(expected_artifacts))
        raise MappingError(f"ownership map does not exactly cover inventoried files; missing={missing[:8]}, extra={extra[:8]}")

    package_paths = {}
    for package, raw_path in mapping["package_paths"].items():
        if not isinstance(package, str) or not package:
            raise MappingError("package_paths contains an empty package name")
        package_paths[package] = repo_relative(root, raw_path, f"package_paths[{package!r}]")
    if len(set(package_paths.values())) != len(package_paths):
        raise MappingError("package_paths maps multiple package names to the same directory")

    source_roots = {package["package"]: (root / package["path"]).resolve() for package in inventory["packages"]}
    destinations = {}
    moves = []
    for source_rel, source_hash in sorted(expected_artifacts.items()):
        owner = ownership[source_rel]
        if not isinstance(owner, dict) or set(owner) != {"package", "sha256"}:
            raise MappingError(f"ownership entry for {source_rel} must contain only package and sha256")
        package = owner["package"]
        if package not in package_paths:
            raise MappingError(f"ownership entry for {source_rel} names unmapped package {package!r}")
        if owner["sha256"] != source_hash:
            raise MappingError(f"ownership checksum is stale for {source_rel}")
        source = repo_relative(root, source_rel, "ownership path")
        source_root = next((base for base in source_roots.values() if source == base or base in source.parents), None)
        if source_root is None:
            raise MappingError(f"ownership path is outside an inventoried implementation package: {source_rel}")
        relative = source.relative_to(source_root)
        destination = package_paths[package] / relative
        try:
            destination.resolve().relative_to(root.resolve())
        except ValueError as exc:
            raise MappingError(f"destination for {source_rel} escapes the repository") from exc
        destination_rel = destination.relative_to(root.resolve()).as_posix()
        if destination_rel in destinations:
            raise MappingError(f"multiple source files map to {destination_rel}")
        destinations[destination_rel] = source_rel
        if source != destination:
            moves.append({"from": source_rel, "to": destination_rel, "sha256": source_hash})

    writes = []
    write_paths = set()
    for index, operation in enumerate(mapping["writes"]):
        if not isinstance(operation, dict) or set(operation) != {"path", "expected_sha256", "content"}:
            raise MappingError(f"writes[{index}] must contain exactly path, expected_sha256, and content")
        path = repo_relative(root, operation["path"], f"writes[{index}].path")
        relative = path.relative_to(root.resolve()).as_posix()
        if relative in write_paths or relative in destinations:
            raise MappingError(f"ambiguous or duplicate write target {relative}")
        write_paths.add(relative)
        if relative in expected_artifacts:
            raise MappingError(f"write target overlaps an owned artifact; map its package owner instead: {relative}")
        expected = operation["expected_sha256"]
        content = operation["content"]
        if expected is not None and (not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{64}", expected)):
            raise MappingError(f"writes[{index}].expected_sha256 must be a SHA-256 or null")
        if content is not None and not isinstance(content, str):
            raise MappingError(f"writes[{index}].content must be text or null")
        after_hash = sha256_bytes(content.encode("utf-8")) if content is not None else None
        writes.append({"path": relative, "expected_sha256": expected, "after_sha256": after_hash})

    required_writes = {"scm/Cargo.toml"}
    lockfile = root / "scm" / "Cargo.lock"
    if lockfile.exists():
        required_writes.add("scm/Cargo.lock")
    required_writes.update(f"{package['path']}/Cargo.toml" for package in inventory["packages"])
    required_writes.update(f"{path.relative_to(root.resolve()).as_posix()}/Cargo.toml" for path in package_paths.values())
    required_writes.update(
        consumer["manifest"]
        for package in inventory["packages"]
        for consumer in package["workspace_consumers"]
    )
    missing_writes = sorted(required_writes - write_paths)
    if missing_writes:
        raise MappingError(f"mapping omits required workspace/package/consumer manifest writes: {missing_writes[:8]}")

    if not moves and not writes:
        raise MappingError("mapping has no migration operations; refusing an incomplete no-op map")

    return {
        "issue": 63,
        "mode": "migration-plan",
        "source_commit": mapping["source_commit"],
        "artifact_count": len(expected_artifacts),
        "move_count": len(moves),
        "write_count": len(writes),
        "moves": moves,
        "writes": writes,
    }


def git_output(root: Path, *args: str) -> str:
    env = os.environ.copy()
    for key in (
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_PREFIX",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ):
        env.pop(key, None)
    result = subprocess.run(
        ["git", *args], cwd=root, env=env, text=True, capture_output=True, check=False
    )
    if result.returncode:
        raise MappingError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def is_plan_already_applied(root: Path, plan: dict) -> bool:
    for move in plan["moves"]:
        source = root / move["from"]
        destination = root / move["to"]
        if source.exists() or not destination.is_file() or sha256_file(destination) != move["sha256"]:
            return False
    for write in plan["writes"]:
        path = root / write["path"]
        if write["after_sha256"] is None:
            if path.exists():
                return False
        elif not path.is_file() or sha256_file(path) != write["after_sha256"]:
            return False
    return True


def ensure_clean_worktree(root: Path) -> None:
    status = git_output(root, "status", "--porcelain", "--untracked-files=all")
    if status:
        raise MappingError("--apply requires a clean worktree; commit or preserve all existing changes first")


def apply_migration(root: Path, mapping: dict, plan: dict) -> str:
    ensure_clean_worktree(root)
    if is_plan_already_applied(root, plan):
        return "already applied; verified destination checksums"
    head = git_output(root, "rev-parse", "HEAD")
    if mapping["source_commit"] != head:
        raise MappingError(f"mapping source_commit {mapping['source_commit']} does not match clean HEAD {head}")

    for move in plan["moves"]:
        source, destination = root / move["from"], root / move["to"]
        if not source.is_file() or source.is_symlink() or sha256_file(source) != move["sha256"]:
            raise MappingError(f"source is missing, not a regular file, or changed: {move['from']}")
        if destination.exists() or destination.is_symlink():
            raise MappingError(f"destination already exists: {move['to']}")
    for write in plan["writes"]:
        path = root / write["path"]
        expected = write["expected_sha256"]
        if expected is None:
            if path.exists():
                raise MappingError(f"new write target already exists: {write['path']}")
        elif not path.is_file() or path.is_symlink() or sha256_file(path) != expected:
            raise MappingError(f"write target is missing, not a regular file, or changed: {write['path']}")

    created_destinations = []
    created_dirs = set()
    previous_writes = {}
    temporary_files = []
    removed_sources = set()
    try:
        for move in plan["moves"]:
            source, destination = root / move["from"], root / move["to"]
            parent = destination.parent
            missing_parents = []
            while not parent.exists() and parent != root:
                missing_parents.append(parent)
                parent = parent.parent
            destination.parent.mkdir(parents=True, exist_ok=True)
            created_dirs.update(missing_parents)
            created_destinations.append(destination)
            shutil.copy2(source, destination)
            if sha256_file(destination) != move["sha256"]:
                raise OSError(f"checksum mismatch after staging {move['to']}")

        for operation, write in zip(mapping["writes"], plan["writes"]):
            path = root / write["path"]
            previous_writes[path] = path.read_bytes() if path.exists() else None
            path.parent.mkdir(parents=True, exist_ok=True)
            if operation["content"] is None:
                path.unlink()
                continue
            with tempfile.NamedTemporaryFile("w", encoding="utf-8", newline="", dir=path.parent, delete=False) as stream:
                temporary = Path(stream.name)
                temporary_files.append(temporary)
                stream.write(operation["content"])
            temporary.replace(path)

        # All destinations and reviewed writes are in place and verified before originals are removed.
        for move in plan["moves"]:
            source, destination = root / move["from"], root / move["to"]
            if sha256_file(destination) != move["sha256"]:
                raise OSError(f"destination changed before source removal: {move['to']}")
        for move in plan["moves"]:
            source = root / move["from"]
            source.unlink()
            removed_sources.add(source)
    except Exception:
        for temporary in temporary_files:
            temporary.unlink(missing_ok=True)
        for path, previous in reversed(list(previous_writes.items())):
            if previous is None:
                path.unlink(missing_ok=True)
            else:
                path.write_bytes(previous)
        for path in reversed(created_destinations):
            source = root / next(move["from"] for move in plan["moves"] if (root / move["to"]) == path)
            # Keep a verified copy if source cleanup had already removed the original.
            if source not in removed_sources:
                path.unlink(missing_ok=True)
        for directory in sorted(created_dirs, key=lambda item: len(item.parts), reverse=True):
            try:
                directory.rmdir()
            except OSError:
                pass
        raise
    return "migration applied; source files were checksum-verified before removal"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--mapping", type=Path, help="reviewed JSON ownership map; without --apply this only prints a plan")
    parser.add_argument("--apply", action="store_true", help="apply the reviewed mapping; requires a clean Git worktree")
    args = parser.parse_args()

    root = args.root.resolve()
    workspace = root / "scm" / "Cargo.toml"
    if not workspace.is_file():
        parser.error(f"{root} does not contain scm/Cargo.toml")

    if args.apply and not args.mapping:
        parser.error("--apply requires --mapping")
    try:
        if args.mapping:
            mapping_path = args.mapping if args.mapping.is_absolute() else root / args.mapping
            mapping = load_mapping(mapping_path)
            plan = build_migration_plan(root, mapping)
            if args.apply:
                result = apply_migration(root, mapping, plan)
                report = {**plan, "mode": "migration-applied", "result": result}
            else:
                report = plan
        else:
            report = build_inventory(root)
    except MappingError as exc:
        parser.error(str(exc))
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
