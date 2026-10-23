import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from migrate_api_packages import MigrationError, apply_plan, plan_migration  # noqa: E402
import migrate_api_packages  # noqa: E402


PACKAGES = {
    "auth-bearer": "grpc-egress-auth-bearer",
    "breaker": "grpc-egress-breaker",
    "resilient": "grpc-egress-resilient",
    "retry": "grpc-egress-retry",
    "transport": "grpc-egress-transport",
}


class ApiPackageMigrationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.main = self.root / "scm/main"
        self.main.mkdir(parents=True)
        members = "\n".join(f'    "main/{name}",' for name in PACKAGES)
        (self.root / "scm/Cargo.toml").write_text(f"[workspace]\nmembers = [\n{members}\n]\n", encoding="utf-8")
        for directory, package in PACKAGES.items():
            crate = package.replace("-", "_")
            base = self.main / directory
            (base / "src/api/traits").mkdir(parents=True)
            (base / "Cargo.toml").write_text(f'[package]\nname = "{package}"\n', encoding="utf-8")
            (base / "src/api/mod.rs").write_text("//! Contract modules.\n\npub(crate) mod traits;\n", encoding="utf-8")
            (base / "src/api/traits/mod.rs").write_text("pub trait ExampleContract {}\n", encoding="utf-8")
            (base / "src/lib.rs").write_text(
                f"//! use {crate}::api::traits::ExampleContract;\nmod api;\npub use api::traits::ExampleContract;\n",
                encoding="utf-8",
            )
        consumer = self.main / "consumer"
        (consumer / "src").mkdir(parents=True)
        (consumer / "Cargo.toml").write_text(
            '[package]\nname = "consumer"\n[dependencies]\n'
            'grpc-egress-transport = { path = "../transport" }\n'
            'tonic = { version = "0.14", features = ["transport"] }\n'
            'edge-transport-breaker = { git = "https://example.test/edge-transport-breaker" }\n',
            encoding="utf-8"
        )
        (consumer / "src/lib.rs").write_text(
            "use grpc_egress_transport::api::traits::ExampleContract;\n", encoding="utf-8"
        )
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        subprocess.run(["git", "-C", str(self.root), "config", "user.name", "Fixture"], check=True)
        subprocess.run(["git", "-C", str(self.root), "config", "user.email", "fixture@example.test"], check=True)
        subprocess.run(["git", "-C", str(self.root), "add", "."], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "fixture"], check=True)

    def tearDown(self):
        self.temp.cleanup()

    def test_plan_is_deterministic_and_read_only(self):
        before = {p.relative_to(self.root).as_posix(): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        plan1 = plan_migration(self.root)
        plan2 = plan_migration(self.root)
        after = {p.relative_to(self.root).as_posix(): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        self.assertEqual(plan1, plan2)
        self.assertEqual(before, after)
        self.assertTrue(any(item.destination and item.destination.as_posix().endswith("transport-api/src/traits/mod.rs") for item in plan1))

    def test_apply_renames_contract_packages_and_flattens_api_modules(self):
        apply_plan(self.root, plan_migration(self.root), refresh_lock=False)
        for directory, package in PACKAGES.items():
            new = self.main / f"{directory}-api"
            self.assertTrue((new / "src/traits/mod.rs").is_file())
            self.assertFalse((new / "src/api").exists())
            self.assertIn(package + "-api", (new / "Cargo.toml").read_text(encoding="utf-8"))
            self.assertIn("mod traits;", (new / "src/lib.rs").read_text(encoding="utf-8"))
            self.assertIn("pub use traits::", (new / "src/lib.rs").read_text(encoding="utf-8"))
            self.assertFalse((self.main / directory).exists())
        workspace = (self.root / "scm/Cargo.toml").read_text(encoding="utf-8")
        self.assertIn('"main/transport-api"', workspace)
        consumer_manifest = (self.main / "consumer/Cargo.toml").read_text(encoding="utf-8")
        self.assertIn("grpc-egress-transport-api", consumer_manifest)
        self.assertIn("../transport-api", consumer_manifest)
        self.assertIn('features = ["transport"]', consumer_manifest)
        self.assertIn("edge-transport-breaker =", consumer_manifest)
        consumer_source = (self.main / "consumer/src/lib.rs").read_text(encoding="utf-8")
        self.assertIn("grpc_egress_transport_api::traits", consumer_source)

    def test_apply_refuses_dirty_worktree(self):
        (self.root / "scm/Cargo.toml").write_text("dirty", encoding="utf-8")
        with self.assertRaisesRegex(MigrationError, "dirty worktree"):
            apply_plan(self.root, plan_migration(self.root), refresh_lock=False)

    def test_allow_dirty_preserves_current_manifest_edits(self):
        manifest = self.root / "scm/Cargo.toml"
        manifest.write_text(manifest.read_text(encoding="utf-8") + "\n# retained local edit\n", encoding="utf-8")
        apply_plan(self.root, plan_migration(self.root), refresh_lock=False, allow_dirty=True)
        migrated = (self.root / "scm/Cargo.toml").read_text(encoding="utf-8")
        self.assertIn('"main/transport-api"', migrated)
        self.assertIn("# retained local edit", migrated)

    def test_plan_is_empty_after_successful_apply(self):
        apply_plan(self.root, plan_migration(self.root), refresh_lock=False)
        self.assertEqual(plan_migration(self.root), [])

    def test_failure_during_lock_refresh_rolls_back_file_moves(self):
        before = {p.relative_to(self.root).as_posix(): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        original_run = subprocess.run

        def fail_cargo_metadata(args, *positional, **kwargs):
            if args and args[0] == "cargo":
                return subprocess.CompletedProcess(args, 1, "", "forced metadata failure")
            return original_run(args, *positional, **kwargs)

        with mock.patch.object(migrate_api_packages.subprocess, "run", side_effect=fail_cargo_metadata):
            with self.assertRaisesRegex(MigrationError, "forced metadata failure"):
                apply_plan(self.root, plan_migration(self.root), refresh_lock=True)
        after = {p.relative_to(self.root).as_posix(): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        self.assertEqual(before, after)

    def test_plan_refuses_existing_destination_collision(self):
        collision = self.main / "transport-api/Cargo.toml"
        collision.parent.mkdir()
        collision.write_text("untracked collision", encoding="utf-8")
        with self.assertRaisesRegex(MigrationError, "destination already exists"):
            plan_migration(self.root)


if __name__ == "__main__":
    unittest.main()
