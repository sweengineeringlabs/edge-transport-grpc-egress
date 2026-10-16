"""Usage: python scripts/mutation_proof_composition_root.py [MUTANT_ID ...]  (e.g. R01 or R07; none = all).
Set CARGO_TARGET_DIR to a private directory: mutation builds are large and a full disk makes
later mutants look caught for the wrong reason (compile errors are reported INVALID).
Each mutant is a one-line break applied to a source file, tested, and restored byte for byte.

One mutant per option or scenario of the composition-root example (#16 tasks 21, 22): removing the
option from the builder (or the rule from `build()`) must fail the test of its scenario.
CAUGHT = a named test fails and the build succeeded."""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.join(HERE, "..", "scm") + os.sep
OUT = os.environ.get("MUTATION_LOG", os.path.join(os.environ.get("TMPDIR", os.environ.get("TEMP", ".")), "mutation_proof_composition_root.log"))
ENV = dict(os.environ)
ENV.update({"CARGO_BUILD_JOBS": "1",
            "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_INCREMENTAL": "0"})
ARGS = ["-p", "grpc-egress-example-composition-root", "--test", "composition_root_int_test", "--no-fail-fast"]
B = "main/transport-std/src/spi/client/tonic/tonic_grpc_egress_builder.rs"
CHAIN = "main/transport-std/src/std/interceptor/egress_interceptor_chain.rs"

ALL_OPTIONS = ["every_transport_option"]
LOADED = ["loaded_config"]
INTERCEPTOR = ["interceptor_scenario"]
INVALID = ["invalid_combination"]

MUTANTS = [
    ("R01 with_compression ignored", B, "            compression: mode,\n", "            compression: self.compression,\n", ALL_OPTIONS + LOADED),
    ("R02 with_interceptor ignored", B, "interceptors: self.interceptors.push(interceptor),", "interceptors: self.interceptors,", ALL_OPTIONS + INTERCEPTOR),
    ("R03 with_trace_context ignored", B, "        self.with_interceptor(Arc::new(interceptor))\n", "        let _ = interceptor;\n        self\n", ALL_OPTIONS + INTERCEPTOR),
    ("R04 with_request_timeout ignored", B, ".with_request_timeout(self.request_timeout)", ".with_request_timeout(Duration::from_secs(3600))", ALL_OPTIONS),
    ("R05 with_max_message_bytes ignored", B, ".with_max_message_bytes(self.max_message_bytes)", ".with_max_message_bytes(64 * 1024 * 1024)", ALL_OPTIONS),
    ("R06 with_channel_config drops compression", B, "            compression: config.compression,\n", "            compression: self.compression,\n", LOADED),
    ("R07 with_channel_config drops the plaintext rule", B, "            plaintext_allowed: !config.tls_required,\n", "            plaintext_allowed: self.plaintext_allowed,\n", LOADED),
    ("R08 with_loadbalancer ignored", B, "Ok(match &self.loadbalancer {", "Ok(match &self.loadbalancer.as_ref().filter(|_| false) {", ["loadbalancer_scenario"]),
    ("R09 with_mtls identity not presented", B, ".with_tls(self.mtls.as_ref(), root_ca)?", ".with_tls(None, root_ca)?", ALL_OPTIONS),
    ("R10 prost option not refused without the feature", B, "if self.prost_codec && !cfg!(feature = \"prost\") {", "if false && self.prost_codec && !cfg!(feature = \"prost\") {", ["prost_scenario"]),
    ("R11 mtls with plaintext not refused", B, "if self.mtls.is_some() && self.plaintext_allowed {", "if false {", INVALID),
    ("R12 plaintext endpoint not rejected", B, "        if !self.plaintext_allowed {\n", "        if false {\n", INVALID),
    ("R13 zero message cap not refused", B, "if self.max_message_bytes == 0 {", "if false {", INVALID),
    ("R14 after_call runs in reverse order", CHAIN, "        for interceptor in &self.interceptors {\n            interceptor.after_call(", "        for interceptor in self.interceptors.iter().rev() {\n            interceptor.after_call(", INTERCEPTOR),
]


def run():
    p = subprocess.run(["cargo", "test", *ARGS], cwd=REPO, env=ENV, capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    return p.stdout + p.stderr


def main():
    only = set(sys.argv[1:])
    for name, rel, find, repl, expect in MUTANTS:
        if only and name.split()[0] not in only:
            continue
        path = REPO + rel
        raw = open(path, "rb").read()
        text = raw.decode("utf-8")
        crlf = "\r\n" in text
        norm = text.replace("\r\n", "\n")
        assert norm.count(find) >= 1, name
        mutated = norm.replace(find, repl, 1)
        try:
            open(path, "wb").write((mutated.replace("\n", "\r\n") if crlf else mutated).encode("utf-8"))
            out = run()
        finally:
            open(path, "wb").write(raw)
        failed = re.findall(r"^test (\S+) \.\.\. FAILED$", out, re.M)
        if "could not compile" in out or "error[E" in out:
            verdict = "INVALID (did not compile)"
        elif any(e in f for e in expect for f in failed):
            verdict = "CAUGHT"
        elif failed:
            verdict = "FAILED OTHER TESTS ONLY"
        else:
            verdict = "SURVIVED"
        line = f"[{verdict}] {name}: failing = {failed}"
        print(line, flush=True)
        open(OUT, "a", encoding="utf-8").write(line + "\n")
    open(OUT, "a", encoding="utf-8").write("MUTANTS_DONE\n")


if __name__ == "__main__":
    main()
