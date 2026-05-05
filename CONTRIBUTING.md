# Contributing to Ferrox

Ferrox is a Converge extension for native optimization solvers and solver
suggestors.

## Development

```sh
just check
just test
just lint
```

Native solver work also needs:

```sh
just deps
just test-full
```

## Boundary

Ferrox owns solver models, native bindings, confidence semantics, and Converge
suggestors. Converge owns the generic suggestor contract and promotion path.
Products own runtime assembly and business-specific orchestration.

## Native Dependencies

Keep FFI isolated in the `*-sys` crates. Library code should use safe wrappers
and return typed errors.

## Solver Contributions

When adding a solver or problem class:

1. Add typed request and plan structs.
2. Add a deterministic pure-Rust or stub path where practical.
3. Make confidence semantics explicit.
4. Add examples or tests that show why the solver improves on a heuristic.
5. Document required native dependencies and feature flags.

## License

By contributing, you agree your contributions are licensed under MIT.
