# Security Policy

BitConcepts takes security seriously. We appreciate the efforts of researchers who help keep Kairos users safe.

## Reporting a Vulnerability

Please follow responsible disclosure — **do not open a public GitHub issue** for security vulnerabilities.

Report privately through one of these channels:

- **Email:** [info@bitconcepts.tech](mailto:info@bitconcepts.tech)
- **GitHub Security Advisory:** [Open a private advisory](https://github.com/BitConcepts/kairos/security/advisories/new)

We will acknowledge your report promptly and work with you to resolve the issue as quickly as possible.

## Scope

Kairos is a locally-run terminal with no cloud backend. The primary security surface is:
- The `specsmith governance-serve` child process and its local HTTP interface (`127.0.0.1:7700`)
- The Rust terminal binary itself (memory safety, input handling)
- Dependency vulnerabilities in third-party crates

## Out of Scope

- Warp cloud services (Kairos makes no calls to warp.dev)
- Social engineering
