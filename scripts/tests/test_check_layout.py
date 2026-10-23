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
    "scm/main/foo/src/api/client/mod.rs": "pub mod dto;\npub use dto::FooRequest;\n",
    "scm/main/foo/src/api/client/dto/mod.rs": "pub mod foo_request;\npub use foo_request::FooRequest;\n",
    "scm/main/foo/src/api/client/dto/foo_request.rs": "pub struct FooRequest;\n",
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
            with open(path, "w", newline="\n") as handle:
                handle.write(text)

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

    def test_check_all_a_domain_and_spi_only_implementation_passes(self):
        files = {k: v for k, v in GOOD.items() if not k.startswith("scm/main/foo-std/")}
        files.update({
            "scm/main/foo-spi/Cargo.toml": manifest("grpc-egress-foo-spi", 'grpc-egress-foo = { path = "../foo" }\n'),
            "scm/main/foo-spi/src/lib.rs": "mod spi;\nmod saf;\n",
            "scm/main/foo-spi/src/spi/mod.rs": "",
            "scm/main/foo-spi/src/saf/mod.rs": "",
        })
        tree = Tree(files)
        self.addCleanup(tree.close)
        self.assertEqual([f.format_line() for f in tree.check()], [])

    def test_check_all_spi_package_rejects_std_layer(self):
        files = {
            "scm/main/foo/Cargo.toml": manifest("grpc-egress-foo"),
            "scm/main/foo/src/lib.rs": "mod api;\n",
            "scm/main/foo/src/api/mod.rs": "",
            "scm/main/foo-spi/Cargo.toml": manifest("grpc-egress-foo-spi", 'grpc-egress-foo = { path = "../foo" }\n'),
            "scm/main/foo-spi/src/lib.rs": "mod spi;\nmod std;\n",
            "scm/main/foo-spi/src/spi/mod.rs": "",
            "scm/main/foo-spi/src/std/mod.rs": "",
        }
        self.assertIn("spi_package_has_std", self.rules(files))

    def test_check_all_std_package_rejects_spi_layer(self):
        files = dict(GOOD)
        files["scm/main/foo-std/src/spi/mod.rs"] = ""
        self.assertIn("std_package_has_spi", self.rules(files))

    def test_check_all_std_package_requires_actual_std_implementation(self):
        files = {k: v for k, v in GOOD.items() if k != "scm/main/foo-std/src/std/mod.rs" and not k.startswith("scm/main/foo-std/src/std/client/")}
        self.assertIn("std_package_missing_std", self.rules(files))

    def test_check_all_spi_package_requires_actual_spi_implementation(self):
        files = {
            "scm/main/foo/Cargo.toml": manifest("grpc-egress-foo"),
            "scm/main/foo/src/lib.rs": "mod api;\n",
            "scm/main/foo/src/api/mod.rs": "",
            "scm/main/foo-spi/Cargo.toml": manifest("grpc-egress-foo-spi", 'grpc-egress-foo = { path = "../foo" }\n'),
            "scm/main/foo-spi/src/lib.rs": "mod saf;\n",
            "scm/main/foo-spi/src/saf/mod.rs": "",
        }
        self.assertIn("spi_package_missing_spi", self.rules(files))

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
        files["scm/main/foo/src/api/client/types/mod.rs"] = ""
        self.assertIn("no_types_folder", self.rules(files))

    def test_check_all_request_declared_outside_a_dto_folder_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/api/client/vo/bar_request.rs"] = "pub struct BarRequest;\n"
        self.assertIn("dto_placement", self.rules(files))

    def test_check_all_response_with_generics_outside_a_dto_folder_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/api/client/vo/bar_response.rs"] = "pub struct BarResponse<T>(T);\n"
        self.assertIn("dto_placement", self.rules(files))

    def test_check_all_error_declared_outside_an_error_folder_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/api/client/vo/bar_error.rs"] = "pub enum BarError {}\n"
        self.assertIn("error_placement", self.rules(files))

    def test_check_all_error_in_an_error_folder_passes(self):
        files = dict(GOOD)
        files["scm/main/foo/src/api/client/error/bar_error.rs"] = "pub enum BarError {}\n"
        self.assertEqual(self.rules(files), set())

    def test_check_all_public_value_object_field_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/api/client/vo/policy.rs"] = "pub struct Policy { pub attempts: u32, value: bool, }\n"
        self.assertIn("private_vo_fields", self.rules(files))

    def test_check_all_private_value_object_fields_pass(self):
        files = dict(GOOD)
        files["scm/main/foo/src/api/client/vo/policy.rs"] = "pub struct Policy { attempts: u32, value: bool, }\n"
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
        files["scm/main/foo/src/api/dto/mod.rs"] = ""
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

    def test_check_all_crate_missing_from_the_workspace_members_fails(self):
        files = dict(GOOD)
        files["scm/Cargo.toml"] = '[workspace]\nmembers = ["main/foo"]\n'
        self.assertIn("workspace_members", self.rules(files))

    def test_check_all_crate_listed_in_the_workspace_members_passes(self):
        files = dict(GOOD)
        files["scm/Cargo.toml"] = '[workspace]\nmembers = [\n  "main/foo",\n  "main/foo-std",\n]\n'
        self.assertEqual(self.rules(files), set())

    def test_check_all_domain_crate_without_an_api_folder_fails(self):
        files = {k: v for k, v in GOOD.items() if "/foo/src/api/" not in k}
        self.assertIn("domain_has_api", self.rules(files))

    def test_check_all_domain_crate_with_a_folder_beside_api_fails(self):
        files = dict(GOOD)
        files["scm/main/foo/src/client/mod.rs"] = ""
        self.assertIn("domain_has_api", self.rules(files))

    def test_check_all_every_failure_line_names_the_path_and_the_rule(self):
        files = dict(GOOD)
        files["scm/main/foo/src/api/client/types/mod.rs"] = ""
        tree = Tree(files)
        self.addCleanup(tree.close)

        lines = [f.format_line() for f in tree.check()]

        self.assertTrue(any("types" in line and "no_types_folder" in line for line in lines), lines)


STD = "scm/main/foo-std/src/std/client/service.rs"
SPI = "scm/main/foo-std/src/spi/client/service.rs"
SAF = "scm/main/foo-std/src/saf/client/service.rs"


class ProgramOverTheInterfaceTest(unittest.TestCase):
    """Rule 19: nothing new is defined in std/ or spi/, and a trait is declared only under api/."""

    def failures(self, **files):
        tree = Tree({**GOOD, **files})
        self.addCleanup(tree.close)
        return [f for f in tree.check() if f.rule == "program_over_the_interface"]

    def rules_of(self, files):
        tree = Tree({**GOOD, **files})
        self.addCleanup(tree.close)
        return {f.rule for f in tree.check()}

    # ---- a trait outside api/
    def test_check_all_trait_declared_in_std_fails(self):
        found = self.failures(**{STD: "pub trait Service {}\n"})
        self.assertEqual(len(found), 1)
        self.assertIn("declares trait Service outside api/", found[0].detail)

    def test_check_all_pub_crate_trait_declared_in_std_fails(self):
        self.assertEqual(len(self.failures(**{STD: "pub(crate) trait Service: Sized {}\n"})), 1)

    def test_check_all_private_trait_declared_in_spi_fails(self):
        self.assertEqual(len(self.failures(**{SPI: "trait Service {\n    fn run(&self);\n}\n"})), 1)

    def test_check_all_trait_declared_in_saf_fails(self):
        self.assertEqual(len(self.failures(**{SAF: "pub trait Service {}\n"})), 1)

    def test_check_all_trait_declared_in_the_test_support_crate_fails(self):
        files = {
            "scm/main/test-support/Cargo.toml": manifest("grpc-egress-test-support"),
            "scm/main/test-support/src/lib.rs": "mod fixtures;\n",
            "scm/main/test-support/src/fixtures.rs": "pub trait Fixture {}\n",
        }
        self.assertEqual(len(self.failures(**files)), 1)

    def test_check_all_trait_declared_in_an_example_crate_fails(self):
        files = {
            "scm/main/examples/demo/Cargo.toml": manifest("grpc-egress-example-demo"),
            "scm/main/examples/demo/src/lib.rs": "mod scenario;\n",
            "scm/main/examples/demo/src/scenario.rs": "pub trait Scenario {}\n",
        }
        self.assertEqual(len(self.failures(**files)), 1)

    def test_check_all_trait_declared_under_api_passes(self):
        files = {"scm/main/foo/src/api/client/traits/service.rs": "pub trait Service: Send {\n    fn run(&self);\n}\n"}
        self.assertEqual(self.failures(**files), [])

    def test_check_all_the_word_trait_in_a_comment_or_a_string_passes(self):
        text = '// pub trait Service {}\n/// a trait Service\nconst NOTE: &str = "pub trait Service {}";\n'
        self.assertEqual(self.failures(**{STD: text}), [])

    # ---- a public function in std/ or spi/ that is not a trait impl
    def test_check_all_pub_fn_in_an_inherent_impl_of_std_fails(self):
        text = "pub(crate) struct Builder;\nimpl Builder {\n    pub fn new() -> Self {\n        Self\n    }\n}\n"
        found = self.failures(**{STD: text})
        self.assertEqual(len(found), 1)
        self.assertIn("pub fn new", found[0].detail)
        self.assertIn("in an inherent impl", found[0].detail)

    def test_check_all_pub_crate_fn_in_an_inherent_impl_of_spi_fails(self):
        text = "struct Client;\nimpl Client {\n    pub(crate) fn create() -> Self {\n        Self\n    }\n}\n"
        found = self.failures(**{SPI: text})
        self.assertEqual(len(found), 1)
        self.assertIn("pub(crate) fn create", found[0].detail)

    def test_check_all_pub_fn_in_a_generic_inherent_impl_with_a_where_clause_fails(self):
        text = "struct Wrap<T>(T);\nimpl<T>\nWrap<T>\nwhere\n    T: Clone,\n{\n    pub fn get(&self) -> T {\n        self.0.clone()\n    }\n}\n"
        self.assertEqual(len(self.failures(**{STD: text})), 1)

    def test_check_all_pub_free_function_in_std_fails(self):
        found = self.failures(**{STD: "pub fn helper() {}\n"})
        self.assertEqual(len(found), 1)
        self.assertIn("as a free function", found[0].detail)

    def test_check_all_pub_crate_free_function_in_spi_fails(self):
        self.assertEqual(len(self.failures(**{SPI: "pub(crate) fn helper() {}\n"})), 1)

    def test_check_all_pub_super_function_in_std_fails(self):
        self.assertEqual(len(self.failures(**{STD: "pub(super) async fn helper() {}\n"})), 1)

    def test_check_all_pub_fn_in_a_nested_inline_module_of_std_fails(self):
        self.assertEqual(len(self.failures(**{STD: "mod inner {\n    pub fn helper() {}\n}\n"})), 1)

    def test_check_all_every_pub_fn_of_a_block_is_reported(self):
        text = "struct B;\nimpl B {\n    pub fn a(&self) {}\n    pub(crate) fn b(&self) {}\n    fn c(&self) {}\n}\n"
        self.assertEqual(len(self.failures(**{STD: text})), 2)

    def test_check_all_private_fn_in_an_inherent_impl_passes(self):
        text = "struct B;\nimpl B {\n    fn helper(&self) {}\n}\nfn free() {}\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_pub_fn_in_a_trait_impl_passes(self):
        text = "struct B;\nimpl Service for B {\n    pub fn run(&self) {}\n}\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_from_and_default_impls_pass(self):
        text = (
            "struct B;\nimpl From<u8> for B {\n    fn from(_: u8) -> Self {\n        B\n    }\n}\n"
            "impl Default for B {\n    fn default() -> Self {\n        B\n    }\n}\n"
        )
        self.assertEqual(self.failures(**{SPI: text}), [])

    def test_check_all_pub_crate_fn_in_a_generic_trait_impl_with_a_where_clause_passes(self):
        text = "impl<T>\nService for Wrap<T>\nwhere\n    T: Clone,\n{\n    pub(crate) fn run(&self) {}\n}\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_a_for_with_higher_ranked_bounds_is_not_a_trait_impl(self):
        text = "struct B;\nimpl B {\n    pub fn run<F>(&self, f: F)\n    where\n        F: for<'a> Fn(&'a str),\n    {\n        let _ = f;\n    }\n}\n"
        self.assertEqual(len(self.failures(**{STD: text})), 1)

    def test_check_all_return_position_impl_trait_is_not_an_impl_block(self):
        text = "fn make() -> impl Iterator<Item = u8> {\n    std::iter::empty()\n}\nstruct B;\nimpl Service for B {\n    pub fn run(&self) {}\n}\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_pub_fn_in_a_comment_or_a_string_passes(self):
        text = '// pub fn hidden() {}\n/* pub fn hidden() {} */\nconst NOTE: &str = "pub fn hidden() {}";\nconst RAW: &str = r#"pub fn hidden() {}"#;\n'
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_pub_fn_in_saf_is_not_checked_by_this_rule(self):
        text = "pub struct Factory;\nimpl Factory {\n    pub fn new() -> Self {\n        Self\n    }\n}\n"
        self.assertEqual(self.failures(**{SAF: text}), [])

    def test_check_all_pub_fn_outside_std_and_spi_is_not_checked_by_this_rule(self):
        files = {"scm/main/foo/src/api/client/dto/foo_request.rs": "pub struct FooRequest;\nimpl Default for FooRequest {\n    fn default() -> Self {\n        Self\n    }\n}\n"}
        self.assertEqual(self.failures(**files), [])

    def test_check_all_failure_line_names_the_path_and_the_rule(self):
        tree = Tree({**GOOD, STD: "pub fn helper() {}\n"})
        self.addCleanup(tree.close)
        lines = [f.format_line() for f in tree.check()]
        self.assertTrue(any("std/client/service.rs" in l and "program_over_the_interface" in l for l in lines), lines)


class StdNamesNoThirdPartyTest(unittest.TestCase):
    """Rule 20: a file under std/ names the domain crate, core, std, alloc and the crate itself, nothing else."""

    def failures(self, **files):
        tree = Tree({**GOOD, **files})
        self.addCleanup(tree.close)
        return [f for f in tree.check() if f.rule == "std_names_no_third_party"]

    def test_check_all_std_file_naming_a_third_party_crate_by_path_fails(self):
        found = self.failures(**{STD: "fn wait() {\n    tokio::time::sleep(d);\n}\n"})
        self.assertEqual(len(found), 1)
        self.assertIn("names the crate 'tokio'", found[0].detail)

    def test_check_all_std_file_importing_a_third_party_crate_fails(self):
        self.assertEqual(len(self.failures(**{STD: "use serde::Deserialize;\n"})), 1)

    def test_check_all_std_file_importing_a_third_party_crate_by_name_only_fails(self):
        self.assertEqual(len(self.failures(**{STD: "use tracing;\n"})), 1)

    def test_check_all_std_file_importing_a_group_from_a_third_party_crate_fails(self):
        self.assertEqual(len(self.failures(**{STD: "use proto_pattern::{CallError, StatusCode};\n"})), 1)

    def test_check_all_std_file_naming_a_derive_of_a_third_party_crate_fails(self):
        self.assertEqual(len(self.failures(**{STD: "#[derive(Clone, prost::Message)]\nstruct M;\n"})), 1)

    def test_check_all_std_file_with_extern_crate_fails(self):
        self.assertEqual(len(self.failures(**{STD: "extern crate rand;\n"})), 1)

    def test_check_all_each_third_party_crate_of_a_file_is_reported_once(self):
        text = "use serde::Deserialize;\nuse toml::from_str;\nfn f() {\n    toml::from_str(t);\n}\n"
        self.assertEqual(len(self.failures(**{STD: text})), 2)

    def test_check_all_std_file_naming_the_domain_crate_passes(self):
        self.assertEqual(self.failures(**{STD: "use grpc_egress_foo::FooRequest;\nfn f() -> grpc_egress_foo::FooRequest {\n    todo()\n}\n"}), [])

    def test_check_all_std_file_naming_core_std_alloc_and_the_crate_passes(self):
        text = "use std::time::Duration;\nuse core::fmt;\nuse alloc::vec::Vec;\nuse crate::std::client::Other;\nuse super::Sibling;\nuse self::inner::Leaf;\nuse grpc_egress_foo_std::Own;\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_std_file_naming_a_primitive_or_a_local_module_passes(self):
        text = "mod inner {}\nfn f() -> u64 {\n    inner::g() + u64::MAX + usize::from(1u8) as u64\n}\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_std_file_naming_a_third_party_crate_in_a_comment_or_a_string_passes(self):
        text = '//! uses `tokio::select!`\n// use serde::Deserialize;\nconst NOTE: &str = "toml::from_str";\n'
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_spi_file_naming_a_third_party_crate_passes(self):
        self.assertEqual(self.failures(**{SPI: "use tokio::time::sleep;\nuse serde::Deserialize;\n"}), [])

    def test_check_all_saf_file_naming_a_third_party_crate_is_not_checked_by_this_rule(self):
        self.assertEqual(self.failures(**{SAF: "use tokio::time::sleep;\n"}), [])

    def test_check_all_std_file_of_a_crate_without_a_domain_pair_names_only_itself(self):
        files = {
            "scm/main/examples/demo/Cargo.toml": manifest("grpc-egress-example-demo"),
            "scm/main/examples/demo/src/lib.rs": "mod std;\n",
            "scm/main/examples/demo/src/std/mod.rs": "use tokio::time::sleep;\n",
        }
        self.assertEqual(len(self.failures(**files)), 1)

    def test_check_all_failure_line_names_the_path_and_the_rule(self):
        tree = Tree({**GOOD, STD: "use serde::Deserialize;\n"})
        self.addCleanup(tree.close)
        lines = [f.format_line() for f in tree.check()]
        self.assertTrue(any("std/client/service.rs" in l and "std_names_no_third_party" in l for l in lines), lines)

    def test_check_all_the_repository_passes_both_new_rules(self):
        root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
        failures = LayoutChecker(root).check_all()
        offending = [f.format_line() for f in failures if f.rule in ("program_over_the_interface", "std_names_no_third_party")]
        self.assertEqual(offending, [])


class EveryFunctionAMethodTest(unittest.TestCase):
    """Rule 21: no free function in src/; every function is a method on a type."""

    def failures(self, **files):
        tree = Tree({**GOOD, **files})
        self.addCleanup(tree.close)
        return [f for f in tree.check() if f.rule == "every_function_a_method"]

    def test_check_all_free_function_in_std_fails(self):
        found = self.failures(**{STD: "fn helper() {}\n"})
        self.assertEqual(len(found), 1)
        self.assertIn("no free function in src: every function is a method on a type", found[0].detail)

    def test_check_all_free_function_in_the_domain_api_fails(self):
        self.assertEqual(len(self.failures(**{"scm/main/foo/src/api/client/dto/foo_request.rs": "pub fn make() {}\n"})), 1)

    def test_check_all_free_function_in_saf_fails(self):
        self.assertEqual(len(self.failures(**{SAF: "pub(crate) fn make() {}\n"})), 1)

    def test_check_all_free_function_in_lib_rs_fails(self):
        self.assertEqual(len(self.failures(**{"scm/main/foo-std/src/lib.rs": "mod std;\nmod saf;\nfn make() {}\n"})), 1)

    def test_check_all_async_const_and_unsafe_free_functions_fail(self):
        text = "async fn a() {}\nconst fn b() {}\nunsafe fn c() {}\npub async fn d() {}\n"
        self.assertEqual(len(self.failures(**{STD: text})), 4)

    def test_check_all_free_function_in_an_inline_module_fails(self):
        self.assertEqual(len(self.failures(**{STD: "mod inner {\n    fn helper() {}\n}\n"})), 1)

    def test_check_all_free_function_in_the_test_support_crate_fails(self):
        files = {
            "scm/main/test-support/Cargo.toml": manifest("grpc-egress-test-support"),
            "scm/main/test-support/src/lib.rs": "mod fixtures;\n",
            "scm/main/test-support/src/fixtures.rs": "pub fn fixture() {}\n",
        }
        self.assertEqual(len(self.failures(**files)), 1)

    def test_check_all_free_function_in_an_example_crate_fails(self):
        files = {
            "scm/main/examples/demo/Cargo.toml": manifest("grpc-egress-example-demo"),
            "scm/main/examples/demo/src/lib.rs": "mod scenario;\n",
            "scm/main/examples/demo/src/scenario.rs": "fn run() {}\n",
        }
        self.assertEqual(len(self.failures(**files)), 1)

    def test_check_all_main_of_a_binary_passes(self):
        files = {
            "scm/main/examples/demo/Cargo.toml": manifest("grpc-egress-example-demo"),
            "scm/main/examples/demo/src/main.rs": "fn main() {\n    Runner.run();\n}\n",
        }
        self.assertEqual(self.failures(**files), [])

    def test_check_all_another_free_function_in_main_rs_fails(self):
        files = {
            "scm/main/examples/demo/Cargo.toml": manifest("grpc-egress-example-demo"),
            "scm/main/examples/demo/src/main.rs": "fn main() {}\nfn helper() {}\n",
        }
        self.assertEqual(len(self.failures(**files)), 1)

    def test_check_all_fn_main_outside_a_binary_fails(self):
        self.assertEqual(len(self.failures(**{STD: "fn main() {}\n"})), 1)

    def test_check_all_methods_in_inherent_and_trait_impls_pass(self):
        text = "struct B;\nimpl B {\n    fn a(&self) {}\n    pub fn b() {}\n}\nimpl T for B {\n    fn c(&self) {}\n}\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_trait_declarations_with_and_without_a_body_pass(self):
        text = "trait T {\n    fn a(&self);\n    fn b(&self) {}\n}\n"
        self.assertEqual(self.failures(**{"scm/main/foo/src/api/client/traits/t.rs": text}), [])

    def test_check_all_function_nested_in_a_method_passes(self):
        text = "struct B;\nimpl B {\n    fn a(&self) {\n        fn inner() {}\n        inner();\n    }\n}\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_impl_with_a_where_clause_and_a_return_position_impl_trait_pass(self):
        text = "struct W<T>(T);\nimpl<T> W<T>\nwhere\n    T: Clone,\n{\n    fn make(&self) -> impl Iterator<Item = u8> {\n        std::iter::empty()\n    }\n}\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_fn_pointer_types_and_fn_bounds_are_not_functions(self):
        text = "struct B {\n    f: fn(u8) -> u8,\n}\nimpl B {\n    fn g<F: Fn(u8)>(&self, _f: F) {}\n}\n"
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_fn_in_a_comment_or_a_string_passes(self):
        text = '// fn hidden() {}\n/* fn hidden() {} */\nstruct S;\nimpl S {\n    const N: &str = "fn hidden() {}";\n}\n'
        self.assertEqual(self.failures(**{STD: text}), [])

    def test_check_all_free_function_in_tests_or_examples_folders_is_not_read(self):
        files = {
            "scm/main/foo/tests/foo_test.rs": "fn helper() {}\n",
            "scm/main/foo/examples/demo.rs": "fn main() {}\nfn helper() {}\n",
        }
        self.assertEqual(self.failures(**files), [])

    def test_check_all_failure_line_names_the_path_and_the_rule(self):
        tree = Tree({**GOOD, STD: "fn helper() {}\n"})
        self.addCleanup(tree.close)
        lines = [f.format_line() for f in tree.check()]
        self.assertTrue(any("std/client/service.rs" in l and "every_function_a_method" in l for l in lines), lines)

    def test_check_all_the_repository_passes_rule_21(self):
        root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
        offending = [f.format_line() for f in LayoutChecker(root).check_all() if f.rule == "every_function_a_method"]
        self.assertEqual(offending, [])


if __name__ == "__main__":
    unittest.main()
