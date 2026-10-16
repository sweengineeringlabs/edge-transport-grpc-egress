#!/usr/bin/env python3
"""Check the project layout of #17 (acceptance criteria 13, 19, 20, 21). Run from anywhere.

    python scripts/check_layout.py [--root REPO_ROOT]

Every failure line names the path and the rule. Exit code 1 when there is a failure.

The layout: `scm/main/<crate>/` is a crate (`scm/main/examples/<name>/` an example crate). A domain
crate `grpc-egress-<theme>` holds only the domain (traits, plain data, errors; one folder per theme,
each with `dto/`, `vo/`, `error/`, `traits/` as needed). Its implementation crate
`grpc-egress-<theme>-std` holds `std/`, `spi/` (only for a library boundary) and `saf/` (factories),
and depends on the domain crate by path.
"""
import argparse
import os
import re
import sys

KIND_DIRS = {"dto", "vo", "entity", "error", "traits"}
LAYER_DIRS = ("std", "spi", "saf")
IMPLEMENTATION_LAYERS = ("std", "spi", "saf", "core")
# The crate that holds fixtures of the tests is not a domain or an implementation of one.
EXEMPT_CRATES = {"grpc-egress-test-support"}
PACKAGE_NAME = re.compile(r"^grpc-egress-[a-z][a-z0-9-]*?(-std)?$")
LOGIC_LINE = re.compile(r"^\s*(pub(\([a-z]+\))?\s+)?(unsafe\s+)?(async\s+)?(fn|struct|enum|trait|impl|const|static|type)\b")
TYPE_DECL = re.compile(r"^\s*pub(?:\([a-z]+\))?\s+(?:struct|enum|trait|type)\s+([A-Za-z0-9_]+)")


class LayoutFailure:
    """One failed check: the path and the rule."""

    def __init__(self, path, rule, detail):
        self.path = path.replace("\\", "/")
        self.rule = rule
        self.detail = detail

    def format_line(self):
        return "%s: %s: %s" % (self.path, self.rule, self.detail)


class LayoutChecker:
    """Walks a repository and collects the failures of every layout rule."""

    def __init__(self, root):
        self.root = os.path.abspath(root)
        self.failures = []

    # ------------------------------------------------------------------ plumbing
    def fail(self, path, rule, detail):
        self.failures.append(LayoutFailure(os.path.relpath(path, self.root), rule, detail))

    @staticmethod
    def read_text(path):
        with open(path, encoding="utf-8") as handle:
            return handle.read()

    def read_package_name(self, manifest):
        for line in self.read_text(manifest).splitlines():
            m = re.match(r'\s*name\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
        return None

    def list_manifests(self):
        found = []
        for dirpath, dirnames, filenames in os.walk(self.root):
            dirnames[:] = [d for d in dirnames if d not in ("target", ".git", "node_modules")]
            if "Cargo.toml" in filenames:
                found.append(os.path.join(dirpath, "Cargo.toml"))
        return sorted(found)

    def list_rust_files(self, base):
        out = []
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in ("target", ".git")]
            out += [os.path.join(dirpath, f) for f in filenames if f.endswith(".rs")]
        return sorted(out)

    def crates(self):
        """{crate directory: package name} for every manifest that has a [package]."""
        crates = {}
        for manifest in self.list_manifests():
            text = self.read_text(manifest)
            if "[package]" in text:
                crates[os.path.dirname(manifest)] = self.read_package_name(manifest)
        return crates

    # ------------------------------------------------------------------ rules
    def check_all(self):
        crates = self.crates()
        main = os.path.join(self.root, "scm", "main")
        for crate_dir, package in crates.items():
            self.check_crate_location(crate_dir, main)
            self.check_package_name(crate_dir, package)
            self.check_no_nested_main(crate_dir)
            self.check_layer_folders(crate_dir, package)
            self.check_source_folders(crate_dir, package)
        self.check_pairs(crates)
        self.check_theme_mirroring(crates)
        self.check_adapter_names(crates)
        return self.failures

    def check_crate_location(self, crate_dir, main):
        rel = os.path.relpath(crate_dir, main).replace("\\", "/")
        if rel.startswith("..") or rel.count("/") not in (0, 1):
            self.fail(crate_dir, "crate_location", "a crate lies directly in scm/main/<crate>/ (an example in scm/main/examples/<name>/)")
        elif rel.count("/") == 1 and not rel.startswith("examples/"):
            self.fail(crate_dir, "crate_location", "only an example crate may lie below scm/main/<folder>/")

    def check_package_name(self, crate_dir, package):
        if package is None or package in EXEMPT_CRATES:
            return
        base = os.path.basename(crate_dir)
        if "/examples/" in crate_dir.replace("\\", "/") + "/":
            return
        if package.startswith("edge-transport-"):
            self.fail(crate_dir, "package_name", "'%s' starts with edge-transport-; our crates are grpc-egress-<theme>[-std]" % package)
        elif not PACKAGE_NAME.match(package):
            self.fail(crate_dir, "package_name", "'%s' is not grpc-egress-<theme> or grpc-egress-<theme>-std" % package)
        if package.endswith("-std") and base != package[len("grpc-egress-"):]:
            self.fail(crate_dir, "package_name", "folder '%s' does not match package '%s'" % (base, package))

    def check_no_nested_main(self, crate_dir):
        if os.path.isdir(os.path.join(crate_dir, "main")):
            self.fail(crate_dir, "no_nested_main", "a crate holds no main/ folder of its own")

    def check_layer_folders(self, crate_dir, package):
        src = os.path.join(crate_dir, "src")
        if package is None or package in EXEMPT_CRATES or not os.path.isdir(src):
            return
        if package.endswith("-std"):
            if os.path.isdir(os.path.join(src, "core")):
                self.fail(os.path.join(src, "core"), "no_core_folder", "the implementation folder is std/, not core/")
            if os.path.isdir(os.path.join(src, "api")):
                self.fail(os.path.join(src, "api"), "implementation_declares_domain", "an implementation crate declares no domain item; it has no api/")
        else:
            for layer in IMPLEMENTATION_LAYERS:
                if os.path.isdir(os.path.join(src, layer)):
                    self.fail(os.path.join(src, layer), "domain_has_no_implementation", "a domain crate has no %s/ folder" % layer)

    def check_source_folders(self, crate_dir, package):
        src = os.path.join(crate_dir, "src")
        if not os.path.isdir(src):
            return
        for dirpath, dirnames, filenames in os.walk(src):
            for d in dirnames:
                if d == "types":
                    self.fail(os.path.join(dirpath, d), "no_types_folder", "a type goes in the folder of its kind: dto/, vo/, entity/, error/, traits/")
                if d == "core":
                    self.fail(os.path.join(dirpath, d), "no_core_folder", "the implementation folder is std/, not core/")
            for f in filenames:
                if f.endswith(".rs"):
                    self.check_rust_file(os.path.join(dirpath, f), check_kinds=package not in EXEMPT_CRATES)

    def check_rust_file(self, path, check_kinds=True):
        name = os.path.basename(path)
        lines = self.read_text(path).splitlines()
        if name in ("mod.rs", "lib.rs"):
            for number, line in enumerate(lines, start=1):
                code = line.split("//")[0]
                if LOGIC_LINE.match(code):
                    self.fail(path, "declarations_only", "line %d holds code, not a declaration: %s" % (number, line.strip()[:60]))
                    break
        parent = os.path.basename(os.path.dirname(path))
        for number, line in enumerate(lines, start=1):
            m = TYPE_DECL.match(line)
            if not m or not check_kinds:
                continue
            declared = m.group(1)
            if re.search(r"(Request|Response)(<.*)?$", declared) and parent != "dto":
                self.fail(path, "dto_placement", "line %d declares %s outside a dto/ folder" % (number, declared))
            if declared.endswith("Error") and parent not in ("error",):
                self.fail(path, "error_placement", "line %d declares %s outside an error/ folder" % (number, declared))

    def check_pairs(self, crates):
        by_name = {package: crate_dir for crate_dir, package in crates.items() if package}
        for crate_dir, package in crates.items():
            if not package or not package.endswith("-std") or package in EXEMPT_CRATES:
                continue
            domain = package[: -len("-std")]
            if domain not in by_name:
                self.fail(crate_dir, "pair", "implementation crate '%s' has no domain crate '%s'" % (package, domain))
                continue
            manifest = self.read_text(os.path.join(crate_dir, "Cargo.toml"))
            if not re.search(r"(?m)^%s\s*(=|\.workspace)" % re.escape(domain), manifest):
                self.fail(crate_dir, "pair", "'%s' does not depend on its domain crate '%s'" % (package, domain))
        for crate_dir, package in crates.items():
            if package and not package.endswith("-std") and package not in EXEMPT_CRATES:
                if "/examples/" in crate_dir.replace("\\", "/") + "/":
                    continue
                if not os.path.isdir(os.path.join(crate_dir, "src")):
                    continue
                # a domain crate needs an implementation crate unless it is a pure port with none yet
                if package + "-std" not in by_name:
                    self.fail(crate_dir, "pair", "domain crate '%s' has no implementation crate '%s-std'" % (package, package))

    def theme_folders(self, src):
        return {d for d in os.listdir(src) if os.path.isdir(os.path.join(src, d)) and d not in KIND_DIRS}

    def check_theme_mirroring(self, crates):
        by_name = {package: crate_dir for crate_dir, package in crates.items() if package}
        for crate_dir, package in crates.items():
            if not package or not package.endswith("-std") or package in EXEMPT_CRATES:
                continue
            domain_dir = by_name.get(package[: -len("-std")])
            if not domain_dir or not os.path.isdir(os.path.join(domain_dir, "src")):
                continue
            themes = self.theme_folders(os.path.join(domain_dir, "src"))
            src = os.path.join(crate_dir, "src")
            for layer in LAYER_DIRS:
                layer_dir = os.path.join(src, layer)
                if not os.path.isdir(layer_dir):
                    continue
                folders = {d for d in os.listdir(layer_dir) if os.path.isdir(os.path.join(layer_dir, d))}
                files = {f for f in os.listdir(layer_dir) if f.endswith(".rs") and f != "mod.rs"}
                if themes:
                    for d in sorted(folders - themes):
                        self.fail(os.path.join(layer_dir, d), "theme_mirroring", "'%s' is not a theme of the domain crate (%s)" % (d, ", ".join(sorted(themes))))
                    if folders and files:
                        self.fail(layer_dir, "theme_mirroring", "files sit beside theme folders: %s" % ", ".join(sorted(files)))
                elif folders and layer == "saf":
                    self.fail(layer_dir, "theme_mirroring", "a crate with one theme has a flat saf/")

    def check_adapter_names(self, crates):
        scm = os.path.join(self.root, "scm", "main")
        for dirpath, dirnames, filenames in os.walk(scm):
            dirnames[:] = [d for d in dirnames if d not in ("target",)]
            for entry in dirnames + filenames:
                if "adapter" in entry.lower():
                    self.fail(os.path.join(dirpath, entry), "no_adapter_name", "no file or folder is named adapter; spi/ and std/ are the adapters")
        for crate_dir, package in crates.items():
            if package and "adapter" in package.lower():
                self.fail(crate_dir, "no_adapter_name", "package name contains 'adapter'")
            src = os.path.join(crate_dir, "src")
            if os.path.isdir(src):
                for path in self.list_rust_files(src):
                    for number, line in enumerate(self.read_text(path).splitlines(), start=1):
                        m = TYPE_DECL.match(line)
                        if m and "adapter" in m.group(1).lower():
                            self.fail(path, "no_adapter_name", "line %d declares %s" % (number, m.group(1)))


class LayoutCommand:
    """Command line of the script."""

    @staticmethod
    def main(argv):
        parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
        parser.add_argument("--root", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
        args = parser.parse_args(argv)
        failures = LayoutChecker(args.root).check_all()
        for failure in failures:
            print(failure.format_line())
        print("%d layout failure(s)" % len(failures))
        return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(LayoutCommand.main(sys.argv[1:]))
