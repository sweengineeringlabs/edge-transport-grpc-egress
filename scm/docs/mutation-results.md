# Mutation results (issue 18)

The results below are from the original issue 18 migration, before the upgrades to
`proto-pattern` v0.12.0 and `proto-svc` v0.4.2. The migration updated mutation
targets to match the new API; the 61 mutants have not yet been rerun against these
dependency versions.

`scripts/mutation/run_mutants.py` applies each mutant of `scripts/mutation/mutants.json` to one source file, runs the
tests of the crates that cover it, and restores the file byte for byte. A mutant is **caught** when at least one test
fails. The list has 61 mutants over the wrappers and mappings this issue wrote.

| Group | Mutants | Covered by | Result |
|---|---|---|---|
| Interceptors, chain, trace, call (`I`, `C`, `T`, `H`) | I01-I05, C01-C03, T01-T02, H01 | `grpc-egress-transport-std` | all caught |
| Metadata extension, balancing, errors (`M`, `E`) | M01-M02, E01, E03-E04 | `grpc-egress-transport-std` | all caught |
| Cancellation, caller identity (`X`, `Y`) | X01-X04, Y01-Y04 | `grpc-egress-transport-std` | all caught |
| Builder, TLS, compression (`L`, `B`, `N`) | L01-L03, B01, B03-B13, N01-N02 | `grpc-egress-transport-std` | all caught |
| Retry (`RT`) | RT01-RT10 | `grpc-egress-retry-std`, `grpc-egress-resilient-std` | all caught |
| Breaker (`BR`) | BR01-BR06 | `grpc-egress-breaker-std`, `grpc-egress-resilient-std` | all caught |
| Bearer authentication (`BA`) | BA01-BA04 | `grpc-egress-auth-bearer-std` | all caught |

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
