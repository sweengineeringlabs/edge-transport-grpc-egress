# Representative composition build comparison

This measures the release-mode composition-root integration test executable before and after issue #62's API and composition changes. It is one representative static composition, not a whole-workspace build or a claim about every consumer.

| Condition | Source | Clean build | Warm build median (5 runs) | Executable |
|---|---|---:|---:|---:|
| Before | baseline commit `0e409dbd7a47b89b0d6f321708c68ed7f71217f7` | 375.060 s | 1.008 s | 8,914,944 bytes |
| After | issue #62 implementation commit `27ecf5a` | 290.966 s | 1.031 s | 8,974,336 bytes |

The clean build was 84.094 seconds (22.4%) faster in this run. The executable grew by 59,392 bytes (0.67%). Warm compilation was effectively unchanged: the measured medians differ by 0.023 seconds, which is below the scale needed to support a performance conclusion. These results do not justify a dispatch or architecture change.

Both source trees used the same Windows 11 Home Single Language machine, AMD Ryzen 5 7520U with Radeon Graphics, Rust `1.99.1 (58cf8f9c3 2026-10-13)`, Cargo `1.99.1 (5f94df478 2026-08-27)`, and `x86_64-pc-windows-msvc`. The profile was Cargo `--release` with workspace-default features. The target was `grpc-egress-example-composition-root`, integration test `composition_root_int_test`, built with `--no-run`. Each clean measurement used a new, empty `CARGO_TARGET_DIR`; the Cargo registry/source cache was already populated, so this is a clean target build rather than a cold-machine build. The warm result is the median wall time of five immediate repetitions against that same target directory.

Exact command, run from `scm/` for each source tree (with the target directory changed to a fresh path for the clean run):

```powershell
cargo test --manifest-path Cargo.toml --target-dir <target-dir> --release -p grpc-egress-example-composition-root --test composition_root_int_test --no-run
```

The executable size is the length of the generated `release/deps/composition_root_int_test-*.exe`. Clean timings are wall-clock measurements around that command. No build flags or target features were changed between the two runs.
