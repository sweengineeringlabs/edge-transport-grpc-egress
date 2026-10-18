# Mutation results (issues 18 and 60)

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

Every one of the 107 mutants is caught; none is expected to survive. The groups were run with
`scripts/mutation/run_mutants.py --only <prefixes> --cmd "cargo test --manifest-path scm/Cargo.toml -p <crates> --no-fail-fast"`.
