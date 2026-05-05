# Ferrox Agent Guide

This is the canonical agent entrypoint for `ferrox`.

`ferrox` is a Converge extension for native optimization solvers and
solver-backed suggestors.

## Start Here

1. Read `README.md`.
2. Read `/Users/kpernyer/dev/extensions/kb/Modules/Ferrox.md`.
3. Check `Cargo.toml` and feature flags before enabling native solvers.
4. Use `just --list` for local commands.

## Commands

```bash
just check       # default, OR-Tools, HiGHS, and full feature checks
just test        # pure Rust tests
just test-full   # native solver tests, requires native deps
just lint
just deps        # build native solver dependencies
```

## Boundaries

- Converge owns the suggestor contract and promotion path.
- `ferrox` owns solver models, native bindings, and solver suggestors.
- Products decide whether to embed `ferrox-solver` or run `ferrox-server`.

## Rules

- Keep unsafe native FFI isolated in the `*-sys` crates.
- Do not add solver orchestration that belongs in a product or formation
  compiler.
- Keep confidence semantics explicit and documented.
- Update `README.md`, `CHANGELOG.md`, and the extensions KB when solver
  behavior or supported problem classes change.
