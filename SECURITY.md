# Security Policy

## Reporting a Vulnerability

If you find a security vulnerability in mkt, please report it responsibly. Do not open a public issue.

Instead, use [GitHub Security Advisories](https://github.com/diorrego/mkt-cli/security/advisories/new) to report the vulnerability privately.

We will acknowledge your report within 48 hours and work with you to understand and fix the issue.

## What We Consider Security Issues

- Token or credential exposure in logs, output, or error messages
- Vulnerabilities in dependency chains
- Injection attacks through user input
- Insecure defaults in configuration

## Security Practices

- All tokens are wrapped with `secrecy::SecretString` so they never appear in debug output or logs
- Environment variables take precedence over config file values
- All HTTP requests use TLS (rustls, no OpenSSL)
- `cargo-deny` runs on every PR to block known vulnerabilities
- Weekly `cargo audit` checks for dependency issues
- `unsafe` code is forbidden at the workspace level

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |
