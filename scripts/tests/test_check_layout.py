"""Tests of scripts/check_layout.py: a tree that passes, and one tree per rule that must fail."""
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from check_layout import LayoutChecker  # noqa: E402


def manifest(name, extra=""):
    return '[package]\nname = "%s"\n%s' % (name, extra)


GOOD = {
    "scm/main/foo/Cargo.toml": manifest("grpc-egress-foo"),
    "scm/main/foo/src/lib.rs": "mod client;\npub use client::FooRequest;\n",
    "scm/main/foo/src/client/mod.rs": "pub mod dto;\npub use dto::FooRequest;\n",
    "scm/main/foo/src/client/dto/mod.rs": "pub mod foo_request;\npub use foo_request::FooRequest;\n",
    "scm/main/foo/src/client/dto/foo_request.rs": "pub struct FooRequest;\n",
    "scm/main/foo-std/Cargo.toml": manifest("grpc-egress-foo-std", 'grpc-egress-foo = { path = "../foo" }\n'),
    "scm/main/foo-std/src/lib.rs": "mod std;\nmod saf;\n",
    "scm/main/foo-std/src/std/mod.rs": "pub(crate) mod client;\n",
    "scm/main/foo-std/src/std/client/mod.rs": "",
    "scm/main/foo-std/src/saf/mod.rs": "mod client;\n",
    "scm/main/foo-std/src/saf/client/mod.rs": "",
}


class Tree:
    """A throwaway repository tree."""

    def __init__(self, files):
        self.dir = tempfile.TemporaryDirectory()
        self.root = self.dir.name
        for rel, text in files.items():
            path = os.path.join(self.root, rel)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            open(path, "w", newline="\n").write(text)

    def check(self):
        return LayoutChecker(self.root).check_all()

    def close(self):
        self.dir.cleanup()


class LayoutCheckerTest(unittest.TestCase):
    def tree(self, **changes):
        files = dict(GOOD)
        for rel, text in changes.items():
            files[rel.replace("__", "/")] = text
        tree = Tree({k: v for k, v in files.items() if v is not None})
        self.addCleanup(tree.close)
        return tree

    def rules(self, files):
        tree = Tree(files)
        self.addCleanup(tree.close)
        return {f.rule for f in tree.check()}

    def test_check_all_a_domain_and_its_implementation_crate_passes(self):
        tree = self.tree()
        self.assertEqual([f.format_line() for f in tree.check()], [])

    def test_check_all_package_starting_with_edge_transport_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/Cargo.toml"] = manifest("edge-transport-grpc-egress-foo")
        self.assertIn("package_name", self.rules(files))

    def test_check_all_crate_outside_scm_main_fails(self):
        files = dict(GOOD)
        files["scm/other/Cargo.toml"] = manifest("grpc-egress-other")
        self.assertIn("crate_location", self.rules(files))

    def test_check_all_crate_holding_its_own_main_folder_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/main/src/lib.rs"] = ""
        self.assertIn("no_nested_main", self.rules(files))

    def test_check_all_domain_crate_with_a_std_folder_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/std/mod.rs"] = ""
        self.assertIn("domain_has_no_implementation", self.rules(files))

    def test_check_all_implementation_crate_without_a_domain_crate_fails(self):
        files = {k: v for k, v in GOOD.items() if not k.startswith("scm/main/foo/")}
        self.assertIn("pair", self.rules(files))

    def test_check_all_implementation_crate_not_depending_on_its_domain_crate_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/Cargo.toml"] = manifest("grpc-egress-foo-std")
        self.assertIn("pair", self.rules(files))

    def test_check_all_domain_crate_without_an_implementation_crate_fails(self):
        files = {k: v for k, v in GOOD.items() if not k.startswith("scm/main/foo-std/")}
        self.assertIn("pair", self.rules(files))

    def test_check_all_core_folder_in_an_implementation_crate_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/core/mod.rs"] = ""
        self.assertIn("no_core_folder", self.rules(files))

    def test_check_all_types_folder_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/client/types/mod.rs"] = ""
        self.assertIn("no_types_folder", self.rules(files))

    def test_check_all_request_declared_outside_a_dto_folder_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/client/vo/bar_request.rs"] = "pub struct BarRequest;\n"
        self.assertIn("dto_placement", self.rules(files))

    def test_check_all_response_with_generics_outside_a_dto_folder_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/client/vo/bar_response.rs"] = "pub struct BarResponse<T>(T);\n"
        self.assertIn("dto_placement", self.rules(files))

    def test_check_all_error_declared_outside_an_error_folder_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/client/vo/bar_error.rs"] = "pub enum BarError {}\n"
        self.assertIn("error_placement", self.rules(files))

    def test_check_all_error_in_an_error_folder_passes(self):
        files = dict(GOOD)
        files["scm/main/foo/src/client/error/bar_error.rs"] = "pub enum BarError {}\n"
        self.assertEqual(self.rules(files), set())

    def test_check_all_lib_rs_with_a_function_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/lib.rs"] = "mod client;\nfn helper() {}\n"
        self.assertIn("declarations_only", self.rules(files))

    def test_check_all_mod_rs_with_an_impl_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/std/mod.rs"] = "pub(crate) mod client;\nimpl X {}\n"
        self.assertIn("declarations_only", self.rules(files))

    def test_check_all_comment_mentioning_a_function_in_mod_rs_passes(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/std/mod.rs"] = "// fn is a keyword\npub(crate) mod client;\n"
        self.assertEqual(self.rules(files), set())

    def test_check_all_file_named_adapter_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/std/client/adapter.rs"] = ""
        self.assertIn("no_adapter_name", self.rules(files))

    def test_check_all_type_named_adapter_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/std/client/wrapper.rs"] = "pub struct HttpAdapter;\n"
        self.assertIn("no_adapter_name", self.rules(files))

    def test_check_all_std_theme_folder_the_domain_does_not_have_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/std/stray/mod.rs"] = ""
        self.assertIn("theme_mirroring", self.rules(files))

    def test_check_all_file_beside_theme_folders_in_a_layer_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/std/loose.rs"] = ""
        self.assertIn("theme_mirroring", self.rules(files))

    def test_check_all_single_theme_crate_with_a_themed_saf_fails(self):
        files = {k: v for k, v in GOOD.items() if "/client/" not in k}
        files["scm/main/foo/src/lib.rs"] = "mod dto;\n"
        files["scm/main/foo/src/dto/mod.rs"] = ""
        files["scm/main/foo-std/src/saf/sub/mod.rs"] = ""
        self.assertIn("theme_mirroring", self.rules(files))

    def test_check_all_allow_attribute_in_a_test_file_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/tests/a_test.rs"] = "#![allow(clippy::unwrap_used)]\nfn x() {}\n"
        self.assertIn("no_allow_attribute", self.rules(files))

    def test_check_all_cfg_attr_allow_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/std/client/impl.rs"] = "#![cfg_attr(test, allow(clippy::unwrap_used))]\n"
        self.assertIn("no_allow_attribute", self.rules(files))

    def test_check_all_allow_mentioned_in_a_comment_passes(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/std/client/impl.rs"] = "// never #[allow(x)] a lint\n"
        self.assertEqual(self.rules(files), set())

    def test_check_all_cfg_test_in_src_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/std/client/impl.rs"] = "#[cfg(test)]\nmod tests {}\n"
        self.assertIn("no_test_code_in_src", self.rules(files))

    def test_check_all_cfg_test_in_tests_folder_passes(self):
        files = dict(GOOD)
        files["scm/main/foo-std/tests/a_test.rs"] = "#[cfg(test)]\nfn x() {}\n"
        self.assertEqual(self.rules(files), set())

    def test_check_all_workspace_dependencies_table_fails(self):
        files = dict(GOOD)
        files["scm/Cargo.toml"] = '[workspace]\n[workspace.dependencies]\nserde = "1"\n'
        self.assertIn("no_workspace_dependencies", self.rules(files))

    def test_check_all_inherited_dependency_fails(self):
        files = dict(GOOD)
        files["scm/main/foo-std/Cargo.toml"] = manifest(
            "grpc-egress-foo-std", 'grpc-egress-foo = { path = "../foo" }\nserde = { workspace = true }\n'
        )
        self.assertIn("no_workspace_dependencies", self.rules(files))

    def test_check_all_inherited_package_key_passes(self):
        files = dict(GOOD)
        files["scm/main/foo-std/Cargo.toml"] = manifest(
            "grpc-egress-foo-std", 'version.workspace = true\ngrpc-egress-foo = { path = "../foo" }\n'
        )
        self.assertEqual(self.rules(files), set())

    def test_check_all_every_failure_line_names_the_path_and_the_rule(self):
        files = dict(GOOD)
        files["scm/main/foo/src/client/types/mod.rs"] = ""
        tree = Tree(files)
        self.addCleanup(tree.close)

        lines = [f.format_line() for f in tree.check()]

        self.assertTrue(any("types" in line and "no_types_folder" in line for line in lines), lines)


if __name__ == "__main__":
    unittest.main()
