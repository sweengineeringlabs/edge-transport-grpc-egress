"""Usage: python scripts/mutation_proof_loadbalancer.py [MUTANT_ID ...]  (e.g. L02; none = all).
Set CARGO_TARGET_DIR to a private directory: mutation builds are large and a full disk makes
later mutants look caught for the wrong reason (compile errors are reported INVALID).
Each mutant is a one-line break applied to a source file, tested, and restored byte for byte.

One-line breaks of the load balancer and of the option validation. CAUGHT = a named test fails and the build succeeded."""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.join(HERE, "..", "scm") + os.sep
OUT = os.environ.get("MUTATION_LOG", os.path.join(os.environ.get("TMPDIR", os.environ.get("TEMP", ".")), "mutation_proof_loadbalancer.log"))
ENV = dict(os.environ)
ENV.update({"CARGO_BUILD_JOBS": "1",
            "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_INCREMENTAL": "0"})
ARGS = ["-p", "grpc-egress-transport-std", "--test", "tonic_grpc_egress_loadbalancer_int_test", "--test", "tonic_grpc_egress_builder_validation_int_test"]
L = "main/transport-std/src/spi/loadbalancer/tonic/lb_grpc_egress.rs"
B = "main/transport-std/src/spi/client/tonic/tonic_grpc_egress_builder.rs"

MUTANTS = [
    ("L01 balanced call not retargeted to the backend", L, "call(self.template.retarget(&backend.url)).await", "call(self.template.clone()).await",
     ["alternates_between_them", "one_backend_down", "weighted_strategy"]),
    ("L02 a refused connection does not degrade the backend", L, "                e @ (GrpcEgressError::ConnectionFailed(_)", "                e @ (GrpcEgressError::Internal(_)",
     ["one_backend_down"]),
    ("L03 weighted strategy mapped to round robin", L, "BalancingStrategy::Weighted => Strategy::Weighted,", "BalancingStrategy::Weighted => Strategy::RoundRobin,",
     ["weighted_strategy"]),
    ("L04 weights not passed to the pool", L, "                    weight: b.weight,", "                    weight: 1,",
     ["weighted_strategy"]),
    ("L05 zero message cap accepted", B, "        if self.max_message_bytes == 0 {", "        if false {", ["max_message_bytes_zero"]),
    ("L06 zero request timeout accepted", B, "        if self.request_timeout.is_zero() {", "        if false {", ["request_timeout_zero"]),
    ("L07 mtls with plaintext allowed accepted", B, "        if self.mtls.is_some() && self.plaintext_allowed {", "        if false {",
     ["mtls_with_plaintext_allowed"]),
    ("L08 loadbalancer with an endpoint accepted", B, "        if self.loadbalancer.is_some() && self.endpoint_explicit {", "        if false {",
     ["loadbalancer_with_an_endpoint", "loadbalancer_with_a_channel_config_endpoint"]),
    ("L09 empty backends not checked", B, "        if settings.backends.is_empty() {", "        if false {", ["no_backends"]),
    ("L10 backend URL not checked", B, "            if backend.url.parse::<http::Uri>().is_err() {", "            if false {", ["invalid_backend_url"]),
    ("L11 channel config endpoint not counted as explicit", B, "            endpoint_explicit: self.endpoint_explicit || !config.endpoint.is_empty(),", "            endpoint_explicit: self.endpoint_explicit,",
     ["channel_config_endpoint"]),
    ("L12 plaintext check looks at no backend", B, "                .find(|url| TonicGrpcEgressProtocol::is_plaintext_endpoint(url))", "                .find(|url| url.is_empty() && TonicGrpcEgressProtocol::is_plaintext_endpoint(url))",
     ["plaintext_backend_without_plaintext_allowed"]),
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
        if expect is None:
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


main()
