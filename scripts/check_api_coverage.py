#!/usr/bin/env python3
"""Check domain API method, DTO-file, and VO encapsulation coverage for #62."""
import os
import re
import sys
from pathlib import Path


def trait_methods(source):
    """Yield public trait names and method names using balanced trait bodies."""
    source = re.sub(r"//[^\n]*|/\*.*?\*/", " ", source, flags=re.S)
    for match in re.finditer(r"\bpub\s+(?:\([^)]*\)\s+)?trait\s+(\w+)\b[^\{]*\{", source):
        depth, end = 1, match.end()
        while end < len(source) and depth:
            depth += (source[end] == "{") - (source[end] == "}")
            end += 1
        body = source[match.end():end - 1]
        for method in re.finditer(r"\bfn\s+(\w+)\s*(?:<[^;{}]*>)?\s*\(", body):
            yield match.group(1), method.group(1)


def check(root):
    root = Path(root)
    failures = []
    main = root / "scm" / "main"
    for crate in sorted(main.iterdir()):
        api = crate / "src" / "api"
        tests = crate / "tests"
        if not api.is_dir() or crate.name.endswith("-std"):
            continue
        test_texts = {}
        for test_root in (tests,):
            if test_root.is_dir():
                for path in test_root.rglob("*.rs"):
                    test_texts[path] = path.read_text(encoding="utf-8")
        # Some domain traits are implemented and exercised from the paired implementation crate.
        paired_tests = main / f"{crate.name}-std" / "tests"
        if paired_tests.is_dir():
            for path in paired_tests.rglob("*.rs"):
                test_texts[path] = path.read_text(encoding="utf-8")
        corpus = "\n".join(test_texts.values())
        for source in api.rglob("*.rs"):
            text = source.read_text(encoding="utf-8")
            for trait, method in trait_methods(text):
                if not re.search(rf"\b{re.escape(method)}\s*\(", corpus) and not re.search(
                    rf"@covers:\s*{re.escape(trait)}::{re.escape(method)}\b", corpus
                ):
                    failures.append(f"{source.relative_to(root)}: method_test: {trait}::{method} has no test invocation")
            for match in re.finditer(r"\bpub\s+struct\s+(\w*(?:Request|Response))\b", text):
                name = match.group(1)
                if "/dto/" not in source.as_posix().replace("\\", "/"):
                    failures.append(f"{source.relative_to(root)}: dto_file: {name} must be declared under dto/")
                if not re.search(rf"\b{re.escape(name)}\b", corpus):
                    failures.append(f"{source.relative_to(root)}: dto_test: {name} is not referenced by tests")
            if "/vo/" in source.as_posix().replace("\\", "/"):
                for match in re.finditer(r"\bpub\s+struct\s+(\w+)\b[^\{]*\{([^}]*)\}", text, re.S):
                    body = match.group(2)
                    if re.search(r"^\s*pub(?:\([^)]*\))?\s+\w+\s*:", body, re.M):
                        continue
                    if "compile_fail" not in text[:match.start()]:
                        failures.append(f"{source.relative_to(root)}: vo_encapsulation_test: {match.group(1)} has private fields but no compile-fail example")
    return failures


def main():
    root = Path(__file__).resolve().parents[1]
    failures = check(root)
    for failure in failures:
        print(failure)
    print(f"{len(failures)} API coverage failure(s)")
    return bool(failures)


if __name__ == "__main__":
    sys.exit(main())
