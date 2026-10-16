"""Usage: python scripts/mutation_proof_tls.py [MUTANT_ID ...]  (e.g. T02; none = all).
Set CARGO_TARGET_DIR to a private directory: mutation builds are large and a full disk makes
later mutants look caught for the wrong reason (compile errors are reported INVALID).
Each mutant is a one-line break applied to a source file, tested, and restored byte for byte.

One-line breaks of the TLS options (mtls, pinned CA). CAUGHT = a named test fails and the build succeeded."""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.join(HERE, "..", "scm") + os.sep
OUT = os.environ.get("MUTATION_LOG", os.path.join(os.environ.get("TMPDIR", os.environ.get("TEMP", ".")), "mutation_proof_tls.log"))
ENV = dict(os.environ)
ENV.update({"CARGO_BUILD_JOBS": "1",
            "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_INCREMENTAL": "0"})
ARGS = ["-p", "edge-transport-grpc-egress-transport", "--test", "tonic_grpc_egress_tls_int_test"]
C = "main/transport/src/spi/client/tonic/tonic_tls_connector.rs"
G = "main/transport/src/spi/client/tonic/tonic_grpc_egress.rs"
B = "main/transport/src/spi/client/tonic/tonic_grpc_egress_builder.rs"
F = "main/transport/src/saf/transport_construction_svc_factory.rs"

MUTANTS = [
    ("T02 pinned CA ignored (web roots always)", C, "            Some(path) => Self::read_roots(path)?,", "            Some(_path) => rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() },",
     ["pinned_ca_peer_proving_itself_with_that_authority", "mtls_peer_demanding"]),
    ("T03 mtls CA bundle not used as pinned CA", B, ".or_else(|| self.mtls.as_ref().and_then(|m| m.ca_pem_path.as_deref()))", "",
     ["with_mtls_peer_demanding_a_client_certificate_answers"]),
    ("T04 TLS option on plaintext endpoint accepted", G, "        if super::tonic_grpc_egress_protocol::TonicGrpcEgressProtocol::is_plaintext_endpoint(", "        if false && super::tonic_grpc_egress_protocol::TonicGrpcEgressProtocol::is_plaintext_endpoint(",
     ["on_a_plaintext_endpoint_fails_the_build"]),
    ("T05 saf factory ignores mtls (issue 13)", F, ".with_tls(config.mtls.as_ref(), pinned_ca)?", ".with_tls(None, None)?",
     ["create_transport_from_config_mtls_is_applied", "create_transport_from_config_unusable_mtls"]),
    ("T06 channel config drops mtls", B, "            mtls: config.mtls.clone().or(self.mtls),\n", "            mtls: self.mtls,\n",
     ["with_channel_config_mtls_is_applied"]),
    ("T07 certificate-less file accepted", C, "        if chain.is_empty() {", "        if false && chain.is_empty() {",
     ["file_without_a_certificate_fails_the_build"]),
    ("T08 key read errors swallowed into a default", C, "        PrivateKeyDer::from_pem_file(path).map_err(|e| Self::refuse(path, e))",
     "        PrivateKeyDer::from_pem_file(path).map_err(|e| Self::refuse(\"\", e))",
     ["key_file_without_a_key_fails_the_build_naming_the_path"]),
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
