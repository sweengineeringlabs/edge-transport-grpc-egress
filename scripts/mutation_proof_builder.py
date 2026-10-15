"""Usage: python scripts/mutation_proof_builder.py [MUTANT_ID ...]  (e.g. B02 or C03; none = all).
Set CARGO_TARGET_DIR to a private directory: mutation builds are large and a full disk makes
later mutants look caught for the wrong reason (compile errors are reported INVALID).
Each mutant is a one-line break applied to a source file, tested, and restored byte for byte.

One-line breaks of the client builder (#16 tasks 2 and 3). CAUGHT = the named test fails and the
build succeeded; a compile error proves nothing. Every file is restored byte for byte."""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.join(HERE, "..", "scm") + os.sep
OUT = os.environ.get("MUTATION_LOG", os.path.join(os.environ.get("TMPDIR", os.environ.get("TEMP", ".")), "mutation_proof_builder.log"))
ENV = dict(os.environ)
ENV.update({"CARGO_BUILD_JOBS": "1",
            "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_INCREMENTAL": "0"})

B = "main/transport/src/spi/client/tonic/tonic_grpc_egress_builder.rs"
ARGS = ["-p", "edge-transport-grpc-egress-transport", "--test", "tonic_grpc_egress_builder_int_test"]

MUTANTS = [
    ("B01 with_endpoint ignored",
     "    fn with_endpoint(self, endpoint: impl Into<String>) -> Self {\n        Self {\n            endpoint: endpoint.into(),\n            ..self\n        }\n    }",
     "    fn with_endpoint(self, endpoint: impl Into<String>) -> Self {\n        let _ = endpoint.into();\n        self\n    }",
     ["test_with_endpoint_overrides_the_endpoint_of_the_factory"]),
    ("B02 plaintext check removed", "if !self.plaintext_allowed && TonicGrpcEgressProtocol", "if !(self.plaintext_allowed || true) && TonicGrpcEgressProtocol",
     ["test_build_plaintext_endpoint_without_plaintext_allowed_returns_plaintext_rejected_error"]),
    ("B03 with_plaintext_allowed does nothing", "        Self {\n            plaintext_allowed: true,\n            ..self\n        }",
     "        Self {\n            plaintext_allowed: false,\n            ..self\n        }",
     ["test_with_plaintext_allowed_plaintext_endpoint_builds_a_client_that_reaches_the_peer_happy"]),
    ("B04 message cap dropped at build", ".with_max_message_bytes(self.max_message_bytes)", ".with_max_message_bytes(usize::MAX)",
     ["test_with_max_message_bytes_response_over_the_cap_returns_resource_exhausted_error"]),
    ("B05 interceptors dropped at build", ".with_interceptors(self.interceptors)", ".with_interceptors(GrpcEgressInterceptorChain::new())",
     ["test_with_interceptor_header_it_adds_reaches_the_peer_happy"]),
    ("B06 only the last interceptor kept", "interceptors: self.interceptors.push(interceptor),", "interceptors: GrpcEgressInterceptorChain::new().push(interceptor),",
     ["test_with_interceptor_two_added_run_in_the_order_they_were_added_edge"]),
    ("B07 static trace context ignored", "} => TraceContextGrpcEgressInterceptor::with_static(traceparent, tracestate),",
     "} => {\n                let _ = (traceparent, tracestate);\n                TraceContextGrpcEgressInterceptor::pass_through()\n            }",
     ["test_with_trace_context_static_injects_the_traceparent_the_peer_receives_happy"]),
    ("B08 channel config cap ignored", "            max_message_bytes: config.max_message_bytes,", "            max_message_bytes: self.max_message_bytes,",
     ["test_with_channel_config_applies_endpoint_plaintext_and_cap_happy"]),
    ("B09 channel config endpoint ignored", "            endpoint: config.endpoint.clone(),", "            endpoint: self.endpoint,",
     ["test_with_channel_config_applies_endpoint_plaintext_and_cap_happy"]),
    ("B10 channel config plaintext rule inverted", "            plaintext_allowed: !config.tls_required,", "            plaintext_allowed: config.tls_required,",
     ["test_with_channel_config_requiring_tls_for_a_plaintext_endpoint_returns_plaintext_rejected_error"]),
    ("B11 mtls silently accepted", '            self.unapplied.push("mtls");\n', "",
     ["test_with_channel_config_mtls_is_refused_until_the_client_applies_it_error"]),
    ("B12 keep_alive silently accepted", '            self.unapplied.push("keep_alive");\n', "",
     ["test_with_channel_config_keep_alive_is_refused_until_the_client_applies_it_error"]),
]


def run():
    p = subprocess.run(["cargo", "test", *ARGS], cwd=REPO, env=ENV, capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    return p.stdout + p.stderr


def main():
    only = set(sys.argv[1:])
    lines = []
    for name, find, repl, expect in MUTANTS:
        if only and name.split()[0] not in only:
            continue
        path = REPO + B
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
            verdict = "FAILED OTHER TESTS ONLY: " + ", ".join(failed)
        else:
            verdict = "SURVIVED"
        line = f"[{verdict}] {name}: failing = {failed}"
        print(line, flush=True)
        lines.append(line)
        open(OUT, "a", encoding="utf-8").write(line + "\n")
    open(OUT, "a", encoding="utf-8").write("MUTANTS_DONE\n")


main()
