@spec/CLAUDE.md
@grey/CLAUDE.md

## Monorepo Layout

- `spec/` — JAR formal specification (Lean 4)
- `grey/` — Grey protocol node (Rust)
- `spec/tests/vectors/` — Shared conformance test vectors (used by both)

## Conventions

- Commit early, commit often. Small logical changes per commit.
- Don't "work around" an issue. Always fix the root cause.
