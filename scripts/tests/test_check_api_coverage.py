import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from check_api_coverage import check


class ApiCoverageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.api = self.root / "scm/main/foo-api/src"
        self.api.mkdir(parents=True)
        (self.root / "scm/main/foo-api/Cargo.toml").write_text(
            '[package]\nname = "grpc-egress-foo-api"\n', encoding="utf-8"
        )
        (self.api / "traits").mkdir()
        (self.api / "traits/service.rs").write_text(
            "pub trait Service { fn call(&self); }\n", encoding="utf-8"
        )
        self.tests = self.root / "scm/main/foo-api/tests"
        self.tests.mkdir()

    def tearDown(self):
        self.temp.cleanup()

    def failures(self):
        return check(self.root)

    def test_uncovered_public_method_is_reported(self):
        self.assertTrue(any("method_test" in failure for failure in self.failures()))

    def test_explicit_method_test_coverage_passes(self):
        (self.tests / "service_test.rs").write_text(
            "// @covers: Service::call\n#[test] fn test_call() {}\n", encoding="utf-8"
        )
        self.assertFalse(any("method_test" in failure for failure in self.failures()))

    def test_method_coverage_in_spi_package_passes(self):
        spi_tests = self.root / "scm/main/foo-spi/tests"
        spi_tests.mkdir(parents=True)
        (spi_tests / "service_test.rs").write_text(
            "// @covers: Service::call\n#[test] fn test_call() {}\n", encoding="utf-8"
        )
        self.assertFalse(any("method_test" in failure for failure in self.failures()))

    def test_dto_placement_and_private_vo_encapsulation_are_checked(self):
        (self.api / "request.rs").write_text(
            "pub struct GetRequest { value: String }\n", encoding="utf-8"
        )
        vo = self.api / "vo"
        vo.mkdir()
        (vo / "value.rs").write_text(
            "pub struct Value { state: String }\n", encoding="utf-8"
        )
        failures = self.failures()
        self.assertTrue(any("dto_file" in failure for failure in failures))
        self.assertTrue(any("vo_encapsulation_test" in failure for failure in failures))


if __name__ == "__main__":
    unittest.main()
