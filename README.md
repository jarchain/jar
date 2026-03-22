# JAR — Join-Accumulate Refine

[![Matrix](https://img.shields.io/matrix/jar%3Amatrix.org?logo=matrix&label=chat)](https://matrix.to/#/#jar:matrix.org)

JAR is a blockchain protocol based on JAM (Join-Accumulate Machine). This monorepo contains both the formal specification and a full node implementation.

## Repository Structure

| Directory | Description |
|-----------|-------------|
| [spec/](spec/) | Lean 4 formal specification — executable, machine-checked, tested against conformance vectors |
| [grey/](grey/) | Grey — Rust protocol node implementation |

## Genesis — Proof of Intelligence

JAR uses a Proof-of-Intelligence model for its genesis token distribution. Every merged PR is scored on difficulty, novelty, and design quality by ranked comparison against past commits. See [GENESIS.md](GENESIS.md) for the full protocol design.

## Quick Start

### Spec (Lean 4)

```sh
cd spec
cd crypto-ffi && cargo build --release && cd ..
lake build
make test
```

### Grey (Rust)

```sh
cd grey
cargo test --workspace
```

## License

Apache-2.0
