# Mutation results (issues 18 and 60)

The results below are from the original issue 18 migration, before the upgrades to
`proto-pattern` v0.12.0 and `proto-svc` v0.4.2. The migration updated mutation
targets to match the new API; the 61 mutants have not yet been rerun against these
dependency versions.

`scripts/mutation/run_mutants.py` applies each mutant of `scripts/mutation/mutants.json` to one source file, runs the
tests of the crates that cover it, and restores the file byte for byte. A mutant is **caught** when at least one test
fails. The list has 107 mutants: 61 over the wrappers and mappings of issue 18, and 46 added by issue 60, one or more
for every trait implementation that `std/` and `spi/` now hold (`D`, `RT11`-`RT19`, `BR07`-`BR11`, `RS`, `BA05`-`BA07`).
Run it in groups, one test command per group (from the repository root, `--manifest-path scm/Cargo.toml`).

| Group | Mutants | Covered by | Result |
|---|---|---|---|
| Interceptors, chain, trace, call (`I`, `C`, `T`, `H`) | I01-I05, C01-C03, T01-T02, H01 | `grpc-egress-transport-std` | all caught |
| Trait impls of `std/`, `spi/` and `saf/` of the transport (`D`) | D01-D24: channel builder (D01-D07), mutual-TLS factory (D08-D09), load balancer settings provider (D10), interceptor chain (D11-D13), metadata extender (D14), pending call (D15-D16), trace-context interceptor and its factory (D17, D24), caller identity (D18-D19), token cancellation (D20), endpoint and builder constructors (D21-D23) | `grpc-egress-transport-std` | all caught |
| Metadata extension, balancing, errors (`M`, `E`) | M01-M02, E01, E03-E04 | `grpc-egress-transport-std` | all caught |
| Cancellation, caller identity (`X`, `Y`) | X01-X04, Y01-Y04 | `grpc-egress-transport-std` | all caught |
| Builder, TLS, compression (`L`, `B`, `N`) | L01-L03, B01, B03-B13, N01-N02 | `grpc-egress-transport-std` | all caught |
| Retry (`RT`) | RT01-RT19 (the classifier, the policy provider and builder, the client constructor) | `grpc-egress-retry-std`, `grpc-egress-resilient-std` | all caught |
| Breaker (`BR`) | BR01-BR11 (the classifier, the policy provider, the client constructor and the factory) | `grpc-egress-breaker-std`, `grpc-egress-resilient-std` | all caught |
| Resilient (`RS`) | RS01-RS05 (the policy provider and its projections, the factory) | `grpc-egress-resilient-std` | all caught |
| Bearer authentication (`BA`) | BA01-BA07 (the secret guard and both builders included) | `grpc-egress-auth-bearer-std` | all caught |

## What the first run found, and what was done

| Mutant | First result | Resolution |
|---|---|---|
| M01 an extension overwrites a header the call carries | survived | the real peer returns the first value of a repeated header, so the old test could not see a duplicate; a test on the hand-written peer (last value wins) now fails when the header is sent twice |
| M02 binary entries are lost when metadata is extended | survived | a test sends a binary header through the identity view and expects the peer to receive it |
| B06 mutual TLS from files is not seen as an identity | survived | a test names mutual-TLS files with plaintext allowed and expects `ConflictingOptions` |
| RT07 one attempt more than the limit is made | survived (a wasted pause after the last attempt) | a test with three attempts and 500 ms pauses expects two pauses, not three |
| RT10 the budget spent before an attempt reports another code | survived | a test with a zero timeout expects `DeadlineExceeded` and no request at the peer |
| X02, X03 a raised signal does not end a read | caught only by hanging forever | the three reads are bounded by a timeout, so the tests fail instead of hanging |
| B02 the same accepted compression is added twice | survived | removed the builder's duplicate check: the engine enables each encoding once, so a repeat changes nothing on the wire; the mutant was dropped with the code it mutated |
| E02 a passed deadline does not mark a backend as failed (transport code) | survived | removed the unreachable arm: the engine reports a passed deadline as a status (`status_adapter.rs` of `proto-svc`), which the remaining arm still counts; the mutant was dropped with the code |

The list had 63 mutants; B02 and E02 left with their code, which makes 61. Every other survivor got a test, not a
reason: none was accepted as equivalent.

## What the run of issue 60 found, and what was done

| Mutant | First result | Resolution |
|---|---|---|
| D15 a pending call forgets the timeout of the options | survived | a test of an interceptor that reads the timeout of the call: `Some(7 s)` when the caller set one, `None` when not |
| RT14 the retry policy accepts zero attempts | survived | tests on the provider (`validate`), on `parse_config` of a text with `max_attempts = 0`, and on the builder |
| RT15 the initial backoff is read in seconds | survived | a test that reads the four derived durations of one policy in milliseconds |
| RT16 a parsed retry policy is not validated | survived | the `parse_config` test above |
| BA06 the outbound config builder drops the subject | survived | the builder test asserts every field, and a test names the missing field |
| BR09 (as first written, a new breaker starts open) | invalid: the variant has a field | rewritten as "starts half-open"; caught by 15 tests |
| N02 the health call goes to Watch | caught only after 10 minutes: the live-server tests waited on a streaming answer for ever | every health call of the tests is bounded by a limit (30 s) so a call that does not end fails the test; the mutant is now caught by 4 tests in seconds |

The issue 60 table above is historical. The final issue #62 rerun below records the current manifest and the expected survivors justified by construction invariants.

## Issue #62 mutation rerun

The complete 107-entry manifest was rerun against the strengthened API. Each package group first passed its unmutated baseline, then ran each mutation against the domain and implementation crates that cover it. The mutation runner restored the mutated source bytes after each case. The final total was **101 caught, 6 expected to survive, 0 invalid or unexplained survivors**. All mutation `find` fragments are unique in the current source tree.

| Group | Manifest entries | Test command packages | Result |
|---|---|---|---|
| Transport, metadata, identity, TLS, cancellation, health, channel and load-balancer (`I`, `C`, `T`, `H`, `M`, `E`, `X`, `Y`, `L`, `B`, `N`, `D`) | 65 | `grpc-egress-transport-std` | All behavior mutations caught. The corrected endpoint-bypass mutant D07 is caught by the existing missing-endpoint builder test. |
| Breaker (`BR`) | 11 | `grpc-egress-breaker`, `grpc-egress-breaker-std`, `grpc-egress-resilient`, `grpc-egress-resilient-std` | 9 caught; BR07 and BR11 are expected survivors because private `GrpcBreakerConfig` construction/deserialization validates the policy before provider/factory validation. |
| Bearer authentication (`BA`) | 7 | `grpc-egress-auth-bearer`, `grpc-egress-auth-bearer-std` | All caught. |
| Retry (`RT`) | 19 | `grpc-egress-retry`, `grpc-egress-retry-std`, `grpc-egress-resilient`, `grpc-egress-resilient-std` | 16 caught; RT14, RT16 and RT17 are expected survivors because `GrpcRetryConfig` construction and deserialization validate before the redundant provider checks. |
| Resilience (`RS`) | 5 | `grpc-egress-resilient`, `grpc-egress-resilient-std`, `grpc-egress-retry`, `grpc-egress-retry-std`, `grpc-egress-breaker`, `grpc-egress-breaker-std` | 4 caught; RS01 is an expected survivor because private `ResilienceConfig` construction and deserialization validate its embedded policies. |

The stale mutations from the previous API revision were repaired before their final rerun: I05 now swaps `InboundReply::new` arguments; M02 targets the binary-copy loop in the extension path; Y04 uses `CallerIdentity::headers`; B01/B04/B13 use current channel accessors and the TLS resolver; D04-D10/D14 target current builder, mTLS, load-balancer and metadata code; BR07/BR08 and RT14/RT15/RT17/RT18 use value-object accessors and the current builders; RS01-RS05 target the current projection and factory paths; and BA06 targets the builder's current `try_new` argument. D10 now tests rejection of a valid parsed backend list because the immutable `LoadbalancerSettings` constructor correctly prevents creating an invalid empty-backend value.

Representative commands (all run from the repository root with the common target directory and a 2 GB free-space reserve):

```powershell
python scripts/mutation/run_mutants.py --mutants scripts/mutation/mutants.json --cmd "cargo test --manifest-path scm/Cargo.toml -p grpc-egress-transport-std" --only I C T H M E X Y L N D B01 B03 B04 B05 B06 B07 B08 B09 B10 B11 B12 B13 --target-dir C:\Users\elvis\.cargo\target --min-free-gb 2
python scripts/mutation/run_mutants.py --mutants scripts/mutation/mutants.json --cmd "cargo test --manifest-path scm/Cargo.toml -p grpc-egress-breaker -p grpc-egress-breaker-std -p grpc-egress-resilient -p grpc-egress-resilient-std" --only BR --target-dir C:\Users\elvis\.cargo\target --min-free-gb 2
python scripts/mutation/run_mutants.py --mutants scripts/mutation/mutants.json --cmd "cargo test --manifest-path scm/Cargo.toml -p grpc-egress-auth-bearer -p grpc-egress-auth-bearer-std" --only BA --target-dir C:\Users\elvis\.cargo\target --min-free-gb 2
python scripts/mutation/run_mutants.py --mutants scripts/mutation/mutants.json --cmd "cargo test --manifest-path scm/Cargo.toml -p grpc-egress-retry -p grpc-egress-retry-std -p grpc-egress-resilient -p grpc-egress-resilient-std" --only RT --target-dir C:\Users\elvis\.cargo\target --min-free-gb 2
python scripts/mutation/run_mutants.py --mutants scripts/mutation/mutants.json --cmd "cargo test --manifest-path scm/Cargo.toml -p grpc-egress-resilient -p grpc-egress-resilient-std -p grpc-egress-retry -p grpc-egress-retry-std -p grpc-egress-breaker -p grpc-egress-breaker-std" --only RS --target-dir C:\Users\elvis\.cargo\target --min-free-gb 2
```

The transport group was split into package-scoped batches and corrected-entry reruns to keep the target directory within the reserve. The breaker, bearer, retry and resilience groups were run separately so unrelated mutants were tested by their owning package suites.

## Issue #61 Pattern lifecycle mutants

Seven lifecycle mutations were added to `scripts/mutation/mutants.json` and run against the unmutated real-peer interceptor suite and then individually. All seven were caught: changed pre-call options, omitted `Started`, `Opened`, `MessageRead`, `Completed`, `Failed`, or `Cancelled`. The first `Failed` mutation targeted an unused helper and survived; it was retargeted to the actual event-forwarding branch and caught by `test_pattern_lifecycle_reports_typed_raw_stream_completion_and_cancellation`. No invalid mutants or unexplained survivors remain. The test suite contains 26 interceptor integration tests, one more than before this change.

## Issue #60 public implementation paths

The six health contract mutations (`H60-01`–`H60-06`) and the two older health mutations (`N01`, `N02`) were run against `grpc-egress-transport::health_check_payload_int_test`. All eight were caught, covering request service encoding/decoding, response status encoding/decoding, unknown statuses, and the standard method path. The obsolete `D20` token-adapter mutation was removed because `TokenCancellation` is no longer an implementation type; cancellation is supplied by a caller-owned `CancellationSignal` implementation. Combined with the documented issue #62 rerun (101 caught, 6 expected survivors) and issue #61 lifecycle rerun (7 caught), the current 119-entry manifest has **113 caught, 6 expected survivors, and 0 invalid or unexplained survivors**. The prior package-group runs were not repeated because the local build volume had less than 1 GB free; this follow-up reran both health mutations against the moved API.
