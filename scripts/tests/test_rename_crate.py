"""Tests of scripts/rename_crate.py against a throwaway git repository."""
import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from rename_crate import CrateRenamer  # noqa: E402

OLD = "edge-transport-grpc-egress-retry"
NEW = "grpc-egress-retry-std"

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


class RenameFixture:
    """A git repository with a crate to rename, a dependent, an external crate and a CHANGELOG."""

    FILES = {
        "scm/Cargo.toml": '[workspace.dependencies]\nedge-transport-grpc-egress-retry = { path = "main/retry" }\n'
                          'edge-transport-retry = { git = "https://example.invalid/retry.git", tag = "v0.1.0" }\n'
                          'edge-transport-retry-policy = { git = "https://example.invalid/policy.git" }\n',
        "scm/main/retry/Cargo.toml": '[package]\nname = "edge-transport-grpc-egress-retry"\n',
        "scm/main/resilient/Cargo.toml": '[dependencies]\nedge-transport-grpc-egress-retry = { workspace = true }\n',
        "scm/main/resilient/src/lib.rs": "use edge_transport_grpc_egress_retry::GrpcRetryFacade;\n",
        "scm/main/retry/README.md": "# edge-transport-grpc-egress-retry\n",
        "scm/main/retry/CHANGELOG.md": "## Unreleased\n- the crate was called edge-transport-grpc-egress-retry\n",
        "scm/main/retry/src/lib.rs": "pub struct GrpcRetryFacade;\n",
        "scm/main/policy/Cargo.toml": '[dependencies]\nedge-transport-grpc-egress-retry-policy = "1"\n',
        "scripts/rename_crate.py": 'TABLE = {"edge-transport-grpc-egress-retry": "grpc-egress-retry-std"}\n',
    }

    def __init__(self):
        self.dir = tempfile.TemporaryDirectory()
        self.root = self.dir.name
        for rel, text in self.FILES.items():
            path = os.path.join(self.root, rel)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            open(path, "w", newline="\n").write(text)
        for args in (["init", "-q"], ["add", "."]):
            subprocess.run(["git", "-C", self.root] + args, check=True, capture_output=True)

    def read(self, rel):
        return open(os.path.join(self.root, rel), newline="").read()

    def close(self):
        self.dir.cleanup()


class CrateRenamerTest(unittest.TestCase):
    def setUp(self):
        self.repo = RenameFixture()
        self.addCleanup(self.repo.close)

    def renamer(self, old=OLD, new=NEW, **kw):
        return CrateRenamer(self.repo.root, old, new, refresh_lock=False, **kw)

    def test_run_dependent_crate_renames_manifest_key_package_and_identifier(self):
        code = self.renamer().run()

        self.assertEqual(code, 0)
        self.assertIn('name = "grpc-egress-retry-std"', self.repo.read("scm/main/retry/Cargo.toml"))
        self.assertIn("grpc-egress-retry-std = { path", self.repo.read("scm/Cargo.toml"))
        self.assertIn("grpc-egress-retry-std = { workspace = true }", self.repo.read("scm/main/resilient/Cargo.toml"))
        self.assertIn("use grpc_egress_retry_std::GrpcRetryFacade;", self.repo.read("scm/main/resilient/src/lib.rs"))
        self.assertIn("# grpc-egress-retry-std", self.repo.read("scm/main/retry/README.md"))

    def test_run_longer_name_that_starts_with_the_old_name_is_not_touched(self):
        self.renamer().run()

        self.assertIn("edge-transport-grpc-egress-retry-policy", self.repo.read("scm/main/policy/Cargo.toml"))

    def test_run_external_crates_of_other_repositories_are_not_touched(self):
        self.renamer().run()

        manifest = self.repo.read("scm/Cargo.toml")
        self.assertIn("edge-transport-retry = {", manifest)
        self.assertIn("edge-transport-retry-policy = {", manifest)

    def test_run_changelog_keeps_the_old_name_as_the_record_of_the_rename(self):
        self.renamer().run()

        self.assertIn("edge-transport-grpc-egress-retry", self.repo.read("scm/main/retry/CHANGELOG.md"))

    def test_run_external_crate_is_refused(self):
        with self.assertRaises(SystemExit) as raised:
            self.renamer(old="edge-transport-retry", new="grpc-egress-x").run()

        self.assertIn("not in the rename table", str(raised.exception))
        self.assertIn("edge-transport-retry = {", self.repo.read("scm/Cargo.toml"))

    def test_run_pair_that_is_not_the_table_pair_is_refused(self):
        with self.assertRaises(SystemExit):
            self.renamer(new="grpc-egress-other").run()

    def test_run_the_script_itself_is_never_rewritten(self):
        self.renamer().run()

        self.assertIn('"edge-transport-grpc-egress-retry"', self.repo.read("scripts/rename_crate.py"))

    def test_run_second_time_changes_nothing(self):
        self.renamer().run()
        before = {rel: self.repo.read(rel) for rel in RenameFixture.FILES}

        code = self.renamer().run()

        self.assertEqual(code, 0)
        self.assertEqual(before, {rel: self.repo.read(rel) for rel in RenameFixture.FILES})

    def test_run_dry_run_prints_the_changes_and_leaves_every_file_untouched(self):
        before = {rel: self.repo.read(rel) for rel in RenameFixture.FILES}

        code = self.renamer(dry_run=True).run()

        self.assertEqual(code, 0)
        self.assertEqual(before, {rel: self.repo.read(rel) for rel in RenameFixture.FILES})

    def test_find_leftovers_old_name_in_an_untouched_file_is_reported(self):
        renamer = self.renamer()
        renamer.run()
        extra = os.path.join(self.repo.root, "scm/main/retry/NOTES.md")
        open(extra, "w").write("see edge-transport-grpc-egress-retry\n")
        subprocess.run(["git", "-C", self.repo.root, "add", "."], check=True, capture_output=True)

        leftovers = renamer.find_leftovers()

        self.assertEqual(len(leftovers), 1)
        self.assertIn("NOTES.md:1", leftovers[0])


if __name__ == "__main__":
    unittest.main()
