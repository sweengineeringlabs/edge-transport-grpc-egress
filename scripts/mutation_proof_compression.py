"""Usage: python scripts/mutation_proof_compression.py [MUTANT_ID ...]  (e.g. B02 or C03; none = all).
Set CARGO_TARGET_DIR to a private directory: mutation builds are large and a full disk makes
later mutants look caught for the wrong reason (compile errors are reported INVALID).
Each mutant is a one-line break applied to a source file, tested, and restored byte for byte.

One-line breaks of message compression. CAUGHT = a named test fails and the build succeeded."""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.join(HERE, "..", "scm") + os.sep
OUT = os.environ.get("MUTATION_LOG", os.path.join(os.environ.get("TMPDIR", os.environ.get("TEMP", ".")), "mutation_proof_compression.log"))
ENV = dict(os.environ)
ENV.update({"CARGO_BUILD_JOBS": "1",
            "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_INCREMENTAL": "0"})
ARGS = ["-p", "edge-transport-grpc-egress-transport", "--test", "tonic_grpc_egress_compression_int_test"]
C = "main/transport/src/spi/client/tonic/grpc_message_codec.rs"
P = "main/transport/src/spi/client/tonic/tonic_grpc_egress_protocol.rs"

MUTANTS = [
    ("C01 request frame never flagged compressed", P, "Some(bytes) => (0x01, bytes.as_slice()),", "Some(bytes) => (0x00, bytes.as_slice()),",
     ["unzips_to_the_original", "decompresses_to_the_original", "empty_message_round_trips"]),
    ("C02 request sent uncompressed but flagged", P, "Some(bytes) => (0x01, bytes.as_slice()),", "Some(_) => (0x01, payload),",
     ["unzips_to_the_original", "decompresses_to_the_original"]),
    ("C03 cap check off by one (limit not +1)", C, ".saturating_add(1);", ".saturating_add(0);",
     ["expands_past_max_message_bytes"]),
    ("C04 cap check removed", C, "if out.len() > max_bytes {", "if false {", ["expands_past_max_message_bytes"]),
    ("C05 compressed flag without encoding passes through", P, "(0x01, None) => {\n                    tracing::warn!(\"compressed gRPC frame without a grpc-encoding header\");\n                    return Err(GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into()));\n                }",
     "(0x01, None) => data[..len].to_vec(),", ["without_an_announced_encoding"]),
    ("C06 unknown encoding treated as gzip", C, 'other => {\n                tracing::warn!(\n                    encoding = other,\n                    "peer used a grpc-encoding this client does not speak"\n                );\n                return Err(',
     'other if false => {\n                tracing::warn!(\n                    encoding = other,\n                    "peer used a grpc-encoding this client does not speak"\n                );\n                return Err(', None),
    ("C07 decompression error swallowed", C, "if let Err(e) = read {", "if let Err(e) = read.or_else(|_| Ok::<usize, std::io::Error>(0)) {",
     ["is_not_valid_gzip"]),
    ("C08 zstd answer decoded as gzip", C, '"zstd" => zstd::stream::read::Decoder::new(payload)', '"zstd" => zstd::stream::read::Decoder::new(&payload[..0])',
     ["zstd_answer_is_decompressed"]),
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
