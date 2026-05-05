# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 0.4.x | Yes |

## Reporting a Vulnerability

Please do not report security vulnerabilities through public GitHub issues.

Report through GitHub Security Advisories for the repository, or email
[kenneth@reflective.se](mailto:kenneth@reflective.se).

You should receive a response within 48 hours.

## Security Notes

- Native FFI boundaries must stay isolated in the `*-sys` crates.
- Solver inputs should be validated before reaching native libraries.
- The gRPC service must be deployed behind appropriate TLS, authentication, and
  network controls.
- Solver time limits and resource limits are operator responsibilities.

## Operator Responsibility

Operators are responsible for TLS configuration, request admission control,
resource isolation, audit logging, and supply-chain review of native solver
builds.
