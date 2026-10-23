#!/usr/bin/env python3
"""Check the project layout of #17 (acceptance criteria 13, 19, 20, 21) and #60 (rules 19 and 20). Run from anywhere.

    python scripts/check_layout.py [--root REPO_ROOT]

Every failure line names the path and the rule. Exit code 1 when there is a failure.

The layout: `scm/main/<crate>/` is a crate (`scm/main/examples/<name>/` an example crate). A domain
crate `grpc-egress-<theme>` holds only the domain, all of it under `src/api/` (traits, plain data,
errors; one folder per theme, each with `dto/`, `vo/`, `error/`, `traits/` as needed). Its implementation crate
`grpc-egress-<theme>-std` holds `std/`, `spi/` (only for a library boundary) and `saf/` (factories),
and depends on the domain crate by path.

Rule 19, `program_over_the_interface` (#60): nothing new is defined in `std/` or `spi/`. A trait is declared only
under `api/` (checked in every crate, the test-support crate and the examples included); a `pub` or `pub(crate)`
function, or an inherent impl that exposes one, in `std/` or `spi/` must be a trait impl.

Rule 21, `every_function_a_method` (#60): no free function in `src/` of any crate; every function is a method on a type
(`fn main` of a binary excepted). Tests, benches and examples are not read.

Rule 20, `std_names_no_third_party` (#60): a file under `std/` of an implementation crate names no crate other than
the domain crate, `core`, `std`, `alloc` and the crate itself. Code that needs another crate is in `spi/`.
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
ALLOW_ATTRIBUTE = re.compile(r"#!?\[(?:cfg_attr\([^\]]*?)?allow\(")
TEST_CONFIG = re.compile(r"#\[cfg\(test\)\]")
INHERITED_DEPENDENCY = re.compile(r"^\s*[A-Za-z0-9_-]+\s*(?:=\s*\{\s*workspace\s*=\s*true|\.workspace\s*=\s*true)")
PACKAGE_KEYS = ("version", "edition", "license", "rust-version", "authors", "repository", "description", "keywords", "categories")
TRAIT_DECL = re.compile(r"(?:^|(?<=[;{}\]\s]))(?:pub(?:\s*\([^)]*\))?\s+)?(?:unsafe\s+)?(?:auto\s+)?trait\s+([A-Za-z0-9_]+)")
ANY_FN = re.compile(r"\b(?:(pub(?:\s*\([^)]*\)))\s+|(pub)\s+)?(?:(?:const|async|unsafe|default)\s+)*fn\s+([A-Za-z0-9_]+)")
IMPL_TRAIT_FOR = re.compile(r"\bfor\b(?!\s*<)")
PRIMITIVES = {"bool", "char", "str", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32", "f64"}
PATH_KEYWORDS = {"crate", "self", "super", "core", "std", "alloc"}
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
        self.check_no_workspace_dependencies(crates)
        self.check_workspace_members(crates)
        for crate_dir, package in crates.items():
            self.check_no_attributes(crate_dir)
            self.check_value_object_fields(crate_dir, package)
            self.check_program_over_the_interface(crate_dir)
            self.check_std_names_no_third_party(crate_dir, package)
            self.check_every_function_a_method(crate_dir)
        return self.failures

    def check_value_object_fields(self, crate_dir, package):
        """Every named field of a domain VO is private; construction goes through its API."""
        if package is None or package.endswith("-std"):
            return
        api = os.path.join(crate_dir, "src", "api")
        if not os.path.isdir(api):
            return
        for path in self.list_rust_files(api):
            rel = os.path.relpath(path, api).replace("\\", "/")
            if "/vo/" not in "/" + rel:
                continue
            text = self.read_text(path)
            for match in re.finditer(r"pub(?:\s*\([^)]*\))?\s+struct\s+([A-Za-z0-9_]+)[^{]*\{([^}]*)\}", text, re.S):
                name, body = match.group(1), match.group(2)
                for field in re.finditer(r"^\s*pub(?:\s*\([^)]*\))?\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", body, re.M):
                    self.fail(path, "private_vo_fields", "line %d exposes field `%s` on value object `%s`; keep VO state private" % (self.line_of(text, match.start(2) + field.start()), field.group(1), name))

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
        if "/examples/" in crate_dir.replace("\\", "/") + "/":
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
            if not os.path.isdir(os.path.join(src, "api")):
                self.fail(src, "domain_has_api", "a domain crate holds its domain under src/api/")
            for entry in sorted(os.listdir(src)):
                if os.path.isdir(os.path.join(src, entry)) and entry != "api":
                    self.fail(os.path.join(src, entry), "domain_has_api", "'%s' lies beside api/; a domain crate holds only src/api/ and lib.rs" % entry)

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

    def theme_folders(self, domain_src):
        api = os.path.join(domain_src, "api")
        if not os.path.isdir(api):
            return set()
        return {d for d in os.listdir(api) if os.path.isdir(os.path.join(api, d)) and d not in KIND_DIRS}

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

    def check_no_workspace_dependencies(self, crates):
        root = os.path.join(self.root, "scm", "Cargo.toml")
        if os.path.isfile(root) and "[workspace.dependencies]" in self.read_text(root):
            self.fail(root, "no_workspace_dependencies", "each crate declares its own dependencies; the root has no [workspace.dependencies]")
        for crate_dir in crates:
            manifest = os.path.join(crate_dir, "Cargo.toml")
            for number, line in enumerate(self.read_text(manifest).splitlines(), start=1):
                if INHERITED_DEPENDENCY.match(line) and not line.strip().startswith(PACKAGE_KEYS):
                    self.fail(manifest, "no_workspace_dependencies", "line %d inherits a dependency from the workspace: %s" % (number, line.strip()[:60]))

    def check_workspace_members(self, crates):
        root = os.path.join(self.root, "scm", "Cargo.toml")
        if not os.path.isfile(root):
            return
        text = self.read_text(root)
        m = re.search(r"members\s*=\s*\[(.*?)\]", text, re.S)
        listed = re.findall(r'"([^"]+)"', m.group(1)) if m else []
        scm = os.path.join(self.root, "scm")
        for crate_dir in crates:
            rel = os.path.relpath(crate_dir, scm).replace("\\", "/")
            if rel not in listed:
                self.fail(crate_dir, "workspace_members", "'%s' is not a member of the workspace in scm/Cargo.toml" % rel)

    def check_no_attributes(self, crate_dir):
        for path in self.list_rust_files(crate_dir):
            in_src = os.path.relpath(path, crate_dir).replace("\\", "/").startswith("src/")
            for number, line in enumerate(self.read_text(path).splitlines(), start=1):
                code = line.split("//")[0]
                if ALLOW_ATTRIBUTE.search(code):
                    self.fail(path, "no_allow_attribute", "line %d silences a lint: fix the code instead: %s" % (number, line.strip()[:60]))
                if in_src and TEST_CONFIG.search(code):
                    self.fail(path, "no_test_code_in_src", "line %d puts test code in src/; tests live in tests/<theme>/" % number)

    # ------------------------------------------------------------------ rules 19 and 20 (#60)
    @staticmethod
    def blank_out(text):
        """`text` with comments, string literals and char literals replaced by spaces (newlines kept), so that a
        keyword or a path inside a comment or a string is never read as code."""
        out = []
        i = 0
        n = len(text)
        while i < n:
            c = text[i]
            two = text[i : i + 2]
            if two == "//":
                while i < n and text[i] != "\n":
                    out.append(" ")
                    i += 1
            elif two == "/*":
                depth = 0
                while i < n:
                    if text[i : i + 2] == "/*":
                        depth += 1
                        out.append("  ")
                        i += 2
                    elif text[i : i + 2] == "*/":
                        depth -= 1
                        out.append("  ")
                        i += 2
                        if depth == 0:
                            break
                    else:
                        out.append("\n" if text[i] == "\n" else " ")
                        i += 1
            elif c == "r" and re.match(r'r#*"', text[i:]) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
                hashes = re.match(r"r(#*)\"", text[i:]).group(1)
                end = text.find('"' + hashes, i + 2 + len(hashes))
                end = n if end < 0 else end + 1 + len(hashes)
                out.extend("\n" if ch == "\n" else " " for ch in text[i:end])
                i = end
            elif c == '"':
                out.append(" ")
                i += 1
                while i < n and text[i] != '"':
                    if text[i] == "\\" and i + 1 < n:
                        out.append(" ")
                        i += 1
                    out.append("\n" if text[i] == "\n" else " ")
                    i += 1
                out.append(" ")
                i += 1
            elif c == "'" and re.match(r"'(\\.[^']*|[^\\'])'", text[i:]):
                end = i + re.match(r"'(\\.[^']*|[^\\'])'", text[i:]).end()
                out.append(" " * (end - i))
                i = end
            else:
                out.append(c)
                i += 1
        return "".join(out)

    @staticmethod
    def line_of(text, position):
        return text.count("\n", 0, position) + 1

    def check_program_over_the_interface(self, crate_dir):
        """Rule 19: a trait is declared under api/ only; std/ and spi/ hold no public function that is not a trait impl."""
        src = os.path.join(crate_dir, "src")
        if not os.path.isdir(src):
            return
        for path in self.list_rust_files(src):
            parts = os.path.relpath(path, src).replace("\\", "/").split("/")
            text = self.blank_out(self.read_text(path))
            if "api" not in parts[:-1]:
                for m in TRAIT_DECL.finditer(text):
                    self.fail(path, "program_over_the_interface", "line %d declares trait %s outside api/: a trait is defined in the domain crate, under api/<theme>/traits/" % (self.line_of(text, m.start()), m.group(1)))
            if parts[0] in ("std", "spi"):
                for position, visibility, name, where in self.list_public_functions(text):
                    self.fail(path, "program_over_the_interface", "line %d declares `%s fn %s` %s; in %s/ only trait impls are public: a constructor is a From/TryFrom/Default impl or a domain factory trait method, an operation is a domain trait method, a helper is private" % (self.line_of(text, position), visibility, name, where, parts[0]))

    def list_public_functions(self, text):
        """(position, visibility, name, where) of every `pub` / `pub(..)` function outside a trait impl."""
        found = []
        for position, visibility, name, governing in self.list_functions(text):
            if not visibility:
                continue
            if governing == "inherent_impl":
                found.append((position, visibility, name, "in an inherent impl"))
            elif governing is None:
                found.append((position, visibility, name, "as a free function"))
        return found

    def list_functions(self, text):
        """(position, visibility, name, governing block) of every `fn` of `text` (comments and strings blanked out).
        The governing block is the nearest enclosing `trait`, `trait_impl` or `inherent_impl`, or None at module level."""
        functions = {}
        for m in ANY_FN.finditer(text):
            visibility = (m.group(1) or m.group(2) or "").replace(" ", "")
            functions[m.start()] = (visibility, m.group(3))
        headers = {}
        for m in re.finditer(r"\b(impl|trait)\b", text):
            before = text[: m.start()].rstrip()
            before = re.sub(r"\b(unsafe|pub(?:\s*\([^)]*\))?)$", "", before).rstrip()
            if before and before[-1] not in ";{}]":
                continue
            brace = self.find_header_end(text, m.end())
            if brace < 0:
                continue
            header = text[m.end() : brace]
            if m.group(1) == "trait":
                kind = "trait"
            elif IMPL_TRAIT_FOR.search(header):
                kind = "trait_impl"
            else:
                kind = "inherent_impl"
            headers[brace] = kind
        found = []
        stack = []
        for position, char in enumerate(text):
            if position in functions:
                visibility, name = functions[position]
                governing = next((k for k in reversed(stack) if k != "other"), None)
                found.append((position, visibility, name, governing))
            if char == "{":
                stack.append(headers.get(position, "other"))
            elif char == "}" and stack:
                stack.pop()
        return found

    def check_every_function_a_method(self, crate_dir):
        """Rule 21: no free function in src/ -- every function is a method on a type (`fn main` of a binary excepted)."""
        src = os.path.join(crate_dir, "src")
        if not os.path.isdir(src):
            return
        for path in self.list_rust_files(src):
            rel = os.path.relpath(path, src).replace("\\", "/")
            is_binary = rel == "main.rs" or rel.startswith("bin/")
            text = self.blank_out(self.read_text(path))
            for position, visibility, name, governing in self.list_functions(text):
                if governing is None and not (is_binary and name == "main"):
                    self.fail(path, "every_function_a_method", "line %d declares the free function %s: no free function in src: every function is a method on a type" % (self.line_of(text, position), name))

    @staticmethod
    def find_header_end(text, start):
        """Index of the `{` that opens the item whose header begins at `start`, or -1 when a `;` comes first."""
        depth = 0
        for index in range(start, len(text)):
            char = text[index]
            if char in "([":
                depth += 1
            elif char in ")]":
                depth -= 1
            elif depth == 0 and char == "{":
                return index
            elif depth == 0 and char == ";":
                return -1
        return -1

    def check_std_names_no_third_party(self, crate_dir, package):
        """Rule 20: a file under std/ names the domain crate, core, std, alloc and the crate itself, and nothing else."""
        std_dir = os.path.join(crate_dir, "src", "std")
        if not os.path.isdir(std_dir) or package is None:
            return
        allowed = set(PATH_KEYWORDS) | PRIMITIVES | {package.replace("-", "_")}
        if package.endswith("-std"):
            allowed.add(package[: -len("-std")].replace("-", "_"))
        for path in self.list_rust_files(std_dir):
            text = self.blank_out(self.read_text(path))
            local = set(re.findall(r"\bmod\s+([a-z_][a-z0-9_]*)", text))
            local |= set(re.findall(r"\b(?:crate|super|self)\s*::\s*(?:[a-z_][a-z0-9_]*\s*::\s*)*([a-z_][a-z0-9_]*)", text))
            reported = set()
            for m in re.finditer(r"(?<![\w:.$])([a-z_][a-z0-9_]*)\s*::|\buse\s+([a-z_][a-z0-9_]*)\s*(?:;|as\b)|\bextern\s+crate\s+([a-z_][a-z0-9_]*)", text):
                name = m.group(1) or m.group(2) or m.group(3)
                if name in allowed or name in local or name in reported:
                    continue
                reported.add(name)
                self.fail(path, "std_names_no_third_party", "line %d names the crate '%s': a file under std/ names the domain crate, core, std and alloc only; code that needs another crate is in spi/" % (self.line_of(text, m.start()), name))

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
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        from check_api_coverage import check as check_api_coverage

        api_failures = check_api_coverage(args.root)
        for failure in failures:
            print(failure.format_line())
        for failure in api_failures:
            print(failure)
        print("%d layout failure(s), %d API coverage failure(s)" % (len(failures), len(api_failures)))
        return 1 if failures or api_failures else 0


if __name__ == "__main__":
    sys.exit(LayoutCommand.main(sys.argv[1:]))
