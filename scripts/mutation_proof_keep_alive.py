"""Usage: python scripts/mutation_proof_keep_alive.py [MUTANT_ID ...]  (e.g. K02; none = all).
Set CARGO_TARGET_DIR to a private directory: mutation builds are large and a full disk makes
later mutants look caught for the wrong reason (compile errors are reported INVALID).
Each mutant is a one-line break applied to a source file, tested, and restored byte for byte.

One-line breaks of the HTTP/2 keep-alive option. CAUGHT = a named test fails and the build succeeded."""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.join(HERE, "..", "scm") + os.sep
OUT = os.environ.get("MUTATION_LOG", os.path.join(os.environ.get("TMPDIR", os.environ.get("TEMP", ".")), "mutation_proof_keep_alive.log"))
ENV = dict(os.environ)
ENV.update({"CARGO_BUILD_JOBS": "1",
            "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_INCREMENTAL": "0"})
ARGS = ["-p", "edge-transport-grpc-egress-transport", "--test", "tonic_grpc_egress_keep_alive_int_test"]
G = "main/transport/src/spi/client/tonic/tonic_grpc_egress.rs"
B = "main/transport/src/spi/client/tonic/tonic_grpc_egress_builder.rs"
F = "main/transport/src/saf/transport_construction_svc_factory.rs"

MUTANTS = [
    ("K01 builder drops the keep-alive at build", B, "            client = client.with_keep_alive(keep_alive);", "            let _ = keep_alive;",
     ["with_keep_alive_peer_that_stops_answering_pings", "with_channel_config_keep_alive_is_applied"]),
    ("K02 permit_without_calls inverted", G, ".http2_keep_alive_while_idle(keep_alive.permit_without_calls)", ".http2_keep_alive_while_idle(!keep_alive.permit_without_calls)",
     ["permit_without_calls_idle_connection", "without_permit_without_calls_idle"]),
    ("K03 ping timeout ignored", G, ".http2_keep_alive_timeout(keep_alive.timeout)", ".http2_keep_alive_timeout(std::time::Duration::from_secs(60))",
     ["peer_that_stops_answering_pings"]),
    ("K04 ping interval ignored", G, ".http2_keep_alive_interval(keep_alive.interval)", ".http2_keep_alive_interval(std::time::Duration::from_secs(60))",
     ["peer_that_stops_answering_pings", "permit_without_calls_idle_connection"]),
    ("K05 channel config drops keep_alive", B, "            keep_alive: config.keep_alive.or(self.keep_alive),\n", "            keep_alive: self.keep_alive,\n",
     ["with_channel_config_keep_alive_is_applied"]),
    ("K06 saf factory ignores keep_alive", F, "            client = client.with_keep_alive(keep_alive);", "            let _ = keep_alive;",
     ["create_transport_from_config_keep_alive_is_applied"]),
    ("K07 keep-alive timer missing", G, "            .timer(hyper_util::rt::TokioTimer::new())\n", "",
     ["peer_that_stops_answering_pings"]),
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
