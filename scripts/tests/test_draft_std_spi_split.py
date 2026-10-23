import json
import hashlib
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from draft_std_spi_split import (  # noqa: E402
    MappingError,
    apply_migration,
    build_inventory,
    build_migration_plan,
)
import draft_std_spi_split  # noqa: E402

_GIT_HOOK_ENV_KEYS = (
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_PREFIX",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
)
_SAVED_GIT_HOOK_ENV = {}


def setUpModule():
    # Git exports these variables to hooks. They would redirect fixture Git
    # commands into the repository running the hook instead of the temp repo.
    for key in _GIT_HOOK_ENV_KEYS:
        value = os.environ.pop(key, None)
        if value is not None:
            _SAVED_GIT_HOOK_ENV[key] = value


def tearDownModule():
    os.environ.update(_SAVED_GIT_HOOK_ENV)


class DraftInventoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "scm").mkdir()
        (self.root / "scm/Cargo.toml").write_text('[workspace]\nmembers = []\n', encoding="utf-8")
        self.main = self.root / "scm/main"
        self.main.mkdir(parents=True)
        self.write("foo", "Cargo.toml", '[package]\nname = "grpc-egress-foo"\n')
        self.write("foo", "src/api/lib.rs", "")
        self.write("foo-std", "Cargo.toml", '[package]\nname = "grpc-egress-foo-std"\n[dependencies]\ngrpc-egress-foo = { path = "../foo" }\n')
        self.write("foo-std", "src/std/lib.rs", "")
        self.write("foo-std", "src/saf/mod.rs", "")
        self.write("foo-spi", "Cargo.toml", '[package]\nname = "grpc-egress-foo-spi"\n[dependencies]\ngrpc-egress-foo = { path = "../foo" }\nserde = "1"\n')
        self.write("foo-spi", "src/spi/lib.rs", "use serde::Serialize;\n")
        self.write("foo-spi", "src/saf/mod.rs", "")
        self.write("foo-spi", "tests/service_test.rs", "")
        self.write("consumer", "Cargo.toml", '[package]\nname = "consumer"\n[dependencies]\nprovider = { package = "grpc-egress-foo-spi", path = "../foo-spi" }\n')

    def tearDown(self):
        self.temp.cleanup()

    def write(self, crate, relative, content):
        path = self.main / crate / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def manifest_writes(self, inventory, package_paths):
        required = {"scm/Cargo.toml"}
        if (self.root / "scm/Cargo.lock").exists():
            required.add("scm/Cargo.lock")
        required.update(f"{package['path']}/Cargo.toml" for package in inventory["packages"])
        required.update(f"{path}/Cargo.toml" for path in package_paths.values())
        required.update(
            consumer["manifest"]
            for package in inventory["packages"]
            for consumer in package["workspace_consumers"]
        )
        writes = []
        for relative in sorted(required):
            path = self.root / relative
            if path.exists():
                current = path.read_text(encoding="utf-8")
                expected = hashlib.sha256(path.read_bytes()).hexdigest()
                content = current.rstrip() + "\n# reviewed migration\n"
            else:
                package_name = next((name for name, directory in package_paths.items() if relative == f"{directory}/Cargo.toml"), "generated-package")
                expected = None
                content = f'[package]\nname = "{package_name}"\n'
            writes.append({"path": relative, "expected_sha256": expected, "content": content})
        return writes

    def test_inventory_covers_both_layers_dependencies_and_consumers(self):
        inventory = build_inventory(self.root)
        self.assertEqual(inventory["package_count"], 2)
        by_name = {item["package"]: item for item in inventory["packages"]}
        spi = by_name["grpc-egress-foo-spi"]
        self.assertEqual(spi["layers"]["spi"], ["scm/main/foo-spi/src/spi/lib.rs"])
        self.assertIn("scm/main/foo-spi/src/saf/mod.rs", spi["layers"]["saf"])
        self.assertEqual(spi["source_referenced_dependencies"]["dependencies"], ["serde"])
        self.assertEqual(spi["workspace_consumers"][0]["package"], "consumer")
        self.assertEqual(spi["related_packages"], ["grpc-egress-foo", "grpc-egress-foo-spi", "grpc-egress-foo-std"])

    def test_inventory_is_read_only(self):
        before = sorted(path.relative_to(self.root).as_posix() for path in self.root.rglob("*"))
        build_inventory(self.root)
        after = sorted(path.relative_to(self.root).as_posix() for path in self.root.rglob("*"))
        self.assertEqual(after, before)

    def test_cli_inventory_is_read_only(self):
        before = sorted(path.relative_to(self.root).as_posix() for path in self.root.rglob("*"))
        script = Path(__file__).resolve().parents[1] / "draft_std_spi_split.py"
        result = subprocess.run(
            [sys.executable, str(script), "--root", str(self.root)],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(json.loads(result.stdout)["mode"], "read-only-inventory")
        after = sorted(path.relative_to(self.root).as_posix() for path in self.root.rglob("*"))
        self.assertEqual(after, before)

    def test_cli_apply_requires_a_reviewed_map(self):
        script = Path(__file__).resolve().parents[1] / "draft_std_spi_split.py"
        result = subprocess.run(
            [sys.executable, str(script), "--root", str(self.root), "--apply"],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("--apply requires --mapping", result.stderr)

    def test_migration_plan_is_deterministic_and_read_only(self):
        self.write("foo-std", "src/spi/provider.rs", "pub(crate) struct Provider;\n")
        inventory = build_inventory(self.root)
        ownership = {
            item["path"]: {
                "package": "grpc-egress-foo-spi" if package["package"] == "grpc-egress-foo-spi" or "/src/spi/" in item["path"] else "grpc-egress-foo-std",
                "sha256": item["sha256"],
            }
            for package in inventory["packages"]
            for item in package["owned_artifacts"]
        }
        mapping = {
            "reviewed": True,
            "source_commit": "a" * 40,
            "package_paths": {
                "grpc-egress-foo-std": "scm/main/foo-std",
                "grpc-egress-foo-spi": "scm/main/foo-spi",
            },
            "ownership": ownership,
            "writes": self.manifest_writes(inventory, {
                "grpc-egress-foo-std": "scm/main/foo-std",
                "grpc-egress-foo-spi": "scm/main/foo-spi",
            }),
        }
        before = sorted(path.relative_to(self.root).as_posix() for path in self.root.rglob("*"))
        first = build_migration_plan(self.root, mapping)
        second = build_migration_plan(self.root, mapping)
        after = sorted(path.relative_to(self.root).as_posix() for path in self.root.rglob("*"))
        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(second, sort_keys=True))
        self.assertEqual(before, after)
        self.assertEqual(first["move_count"], 1)

    def test_incomplete_and_ambiguous_mappings_are_rejected(self):
        self.write("foo-std", "src/spi/provider.rs", "pub(crate) struct Provider;\n")
        ownership = {
            item["path"]: {
                "package": "grpc-egress-foo-spi" if package["package"] == "grpc-egress-foo-spi" or "/src/spi/" in item["path"] else "grpc-egress-foo-std",
                "sha256": item["sha256"],
            }
            for package in build_inventory(self.root)["packages"]
            for item in package["owned_artifacts"]
        }
        mapping = {
            "reviewed": True,
            "source_commit": "a" * 40,
            "package_paths": {"grpc-egress-foo-std": "scm/main/foo-std"},
            "ownership": ownership,
            "writes": [],
        }
        del mapping["ownership"][next(iter(mapping["ownership"]))]
        with self.assertRaisesRegex(MappingError, "exactly cover"):
            build_migration_plan(self.root, mapping)

    def test_ambiguous_destination_is_rejected(self):
        self.write("foo-std", "src/spi/provider.rs", "combined copy\n")
        self.write("foo-spi", "src/spi/provider.rs", "existing copy\n")
        inventory = build_inventory(self.root)
        ownership = {
            item["path"]: {
                "package": "grpc-egress-foo-spi" if package["package"] == "grpc-egress-foo-spi" or "/src/spi/" in item["path"] else "grpc-egress-foo-std",
                "sha256": item["sha256"],
            }
            for package in inventory["packages"]
            for item in package["owned_artifacts"]
        }
        mapping = {
            "reviewed": True,
            "source_commit": "a" * 40,
            "package_paths": {
                "grpc-egress-foo-std": "scm/main/foo-std",
                "grpc-egress-foo-spi": "scm/main/foo-spi",
            },
            "ownership": ownership,
            "writes": self.manifest_writes(inventory, {
                "grpc-egress-foo-std": "scm/main/foo-std",
                "grpc-egress-foo-spi": "scm/main/foo-spi",
            }),
        }
        with self.assertRaisesRegex(MappingError, "multiple source files"):
            build_migration_plan(self.root, mapping)

    def _init_git(self):
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=self.root, check=True)
        subprocess.run(["git", "config", "user.name", "Test"], cwd=self.root, check=True)
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        subprocess.run(["git", "commit", "-qm", "baseline"], cwd=self.root, check=True)
        return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=self.root, text=True).strip()

    def _reviewed_mapping(self):
        source_commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=self.root, text=True).strip()
        inventory = build_inventory(self.root)
        ownership = {
            item["path"]: {
                "package": "grpc-egress-foo-spi" if package["package"] == "grpc-egress-foo-spi" or "/src/spi/" in item["path"] else "grpc-egress-foo-std",
                "sha256": item["sha256"],
            }
            for package in inventory["packages"]
            for item in package["owned_artifacts"]
        }
        package_paths = {
            "grpc-egress-foo-std": "scm/main/foo-std",
            "grpc-egress-foo-spi": "scm/main/foo-spi",
        }
        return {
            "reviewed": True,
            "source_commit": source_commit,
            "package_paths": package_paths,
            "ownership": ownership,
            "writes": self.manifest_writes(inventory, package_paths),
        }

    def test_apply_preserves_files_and_is_idempotent_after_commit(self):
        self.write("foo-std", "src/spi/provider.rs", "pub(crate) struct Provider;\n")
        self._init_git()
        mapping = self._reviewed_mapping()
        plan = build_migration_plan(self.root, mapping)
        self.assertIn("migration applied", apply_migration(self.root, mapping, plan))
        destination = self.main / "foo-spi/src/spi/provider.rs"
        self.assertEqual(destination.read_text(encoding="utf-8"), "pub(crate) struct Provider;\n")
        self.assertFalse((self.main / "foo-std/src/spi/provider.rs").exists())
        self.assertIn("reviewed migration", (self.main / "foo-spi/Cargo.toml").read_text(encoding="utf-8"))
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        subprocess.run(["git", "commit", "-qm", "applied split"], cwd=self.root, check=True)
        self.assertIn("already applied", apply_migration(self.root, mapping, plan))

    def test_cli_applies_only_with_explicit_flag_and_reviewed_map(self):
        self.write("foo-std", "src/spi/provider.rs", "pub(crate) struct Provider;\n")
        self._init_git()
        mapping = self._reviewed_mapping()
        mapping_file = self.root.parent / f"{self.root.name}-split-map.json"
        mapping_file.write_text(json.dumps(mapping), encoding="utf-8")
        self.addCleanup(lambda: mapping_file.unlink(missing_ok=True))
        script = Path(__file__).resolve().parents[1] / "draft_std_spi_split.py"
        result = subprocess.run(
            [sys.executable, str(script), "--root", str(self.root), "--mapping", str(mapping_file), "--apply"],
            check=True,
            capture_output=True,
            text=True,
        )
        report = json.loads(result.stdout)
        self.assertEqual(report["mode"], "migration-applied")
        self.assertTrue((self.main / "foo-spi/src/spi/provider.rs").is_file())
        self.assertFalse((self.main / "foo-std/src/spi/provider.rs").exists())

    def test_apply_refuses_dirty_worktree_without_touching_sources(self):
        self.write("foo-std", "src/spi/provider.rs", "pub(crate) struct Provider;\n")
        self._init_git()
        mapping = self._reviewed_mapping()
        dirty = self.root / "untracked.txt"
        dirty.write_text("keep me", encoding="utf-8")
        plan = build_migration_plan(self.root, mapping)
        with self.assertRaisesRegex(MappingError, "clean worktree"):
            apply_migration(self.root, mapping, plan)
        self.assertTrue((self.main / "foo-std/src/spi/provider.rs").is_file())
        self.assertEqual(dirty.read_text(encoding="utf-8"), "keep me")

    def test_apply_rolls_back_staged_copies_when_a_write_fails(self):
        self.write("foo-std", "src/spi/provider.rs", "pub(crate) struct Provider;\n")
        self._init_git()
        mapping = self._reviewed_mapping()
        plan = build_migration_plan(self.root, mapping)
        with mock.patch.object(draft_std_spi_split.tempfile, "NamedTemporaryFile", side_effect=OSError("simulated write failure")):
            with self.assertRaisesRegex(OSError, "simulated write failure"):
                apply_migration(self.root, mapping, plan)
        self.assertTrue((self.main / "foo-std/src/spi/provider.rs").is_file())
        self.assertFalse((self.main / "foo-spi/src/spi/provider.rs").exists())
        self.assertFalse((self.main / "foo-spi/README.md").exists())


if __name__ == "__main__":
    unittest.main()
