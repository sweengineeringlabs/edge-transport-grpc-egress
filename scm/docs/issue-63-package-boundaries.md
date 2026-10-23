# Issue 63 package ownership and boundaries

The detailed generated inventory is [issue-63-package-inventory.json](../issue-63-package-inventory.json). It lists every implementation source by `std`, `spi`, or `saf`, every test and example, declared and source-referenced dependencies, workspace consumers, and checksummed package artifacts. Regenerate it by directing the read-only inventory output to that path.

The migration driver defaults to read-only inventory or plan output. A reviewed JSON map supplies the source Git revision, exact package ownership for every inventoried artifact (with its checksum), destination package directories, and explicit manifest/document writes. Review the plan with `python scripts/draft_std_spi_split.py --mapping split-map.json`; apply it only from a clean worktree with `python scripts/draft_std_spi_split.py --mapping split-map.json --apply`. The driver refuses missing or ambiguous ownership, stale checksums, path escapes, conflicting destinations, dirty worktrees, and source revisions that do not match. It verifies staged copies before removing originals, rolls back staged outputs on write failure, and recognizes a committed already-applied plan by destination checksums.

| Domain | Standard package | SPI package | Ownership |
|---|---|---|---|
| `auth-bearer` | `auth-bearer-std` | `auth-bearer-spi` | Standard config and claims builders stay dependency-light. JWT signing, constant-time secret handling, security integration, examples, and tests belong to SPI. |
| `retry` | `retry-std` | `retry-spi` | Standard retry classification stays dependency-light. Policy parsing/building, the retry decorator, scheduler, and their provider/runtime dependencies belong to SPI. |
| `transport` | `transport-std` | `transport-spi` | Standard channel config, TLS resolution, and interceptor implementations stay dependency-light. The tonic client, Pattern lifecycle adapter, cancellation, caller identity, load-balancer integration, and their tests/examples belong to SPI. |
| `breaker` | — | `breaker-spi` | The breaker policy provider and client depend on provider/runtime crates, so this theme has no standard implementation package. |
| `resilient` | — | `resilient-spi` | The composition factory and policy projection depend on retry and breaker SPI packages, so this theme has no standard implementation package. |

Each implementation package depends on its domain contract. The standard packages have no direct third-party dependencies. SPI packages use the domain factory contracts for cross-package composition: `transport-spi` receives channel, TLS, and interceptor implementations through public transport factories; `retry-spi` obtains the standard classifier through `RetryClassifierFactory`. The chain contract now returns an opaque `Send + Sync` chain that the SPI transport can own without exposing a standard implementation type. No public bridge was added for a crate-private implementation type.

Tests and examples moved with the implementation they exercise. The composition-root and test-support crates now import provider-backed transport APIs from `grpc-egress-transport-spi`. The inventory lists any consumer package that selects each implementation package.

The annotated test inventory is unchanged from the pre-migration `dev` baseline: 435 tests across 44 Rust files before and after the split, with no test names lost. The complete workspace test suite passed. The mutation manifest now has 127 valid targets; Y02 was removed because its `SecurityContext` behavior is implemented in the external `edge-security-runtime` dependency, while the unauthenticated-context behavior remains covered by the caller-identity integration test.
