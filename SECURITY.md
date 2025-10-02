# Security Policy

## Supported Versions

The following versions of RustCMS are currently receiving security updates:

| Version | Supported          | Notes                           |
| ------- | ------------------ | ------------------------------- |
| 3.0.x   | :white_check_mark: | Current stable release          |
| 2.x.x   | :x:                | End of life                     |
| < 2.0   | :x:                | End of life                     |

## Reporting a Vulnerability

We take the security of RustCMS seriously. If you discover a security vulnerability, please follow these steps:

### Where to Report

- **Email**: Send details to the project maintainer via GitHub
- **GitHub Security Advisories**: Use the [Security tab](https://github.com/jungamer-64/RustCMS/security/advisories) to report privately
- **Do NOT** open public issues for security vulnerabilities

### What to Include

Please include the following information in your report:

1. **Description**: Clear description of the vulnerability
2. **Impact**: Potential impact and attack scenario
3. **Reproduction Steps**: Detailed steps to reproduce the issue
4. **Affected Versions**: Which versions are affected
5. **Suggested Fix**: If you have ideas for a fix (optional)

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Updates**: Every 7 days until resolved
- **Resolution**: Target within 30 days for critical issues

### What to Expect

**If Accepted:**

- We will work with you to understand and reproduce the issue
- A security patch will be developed and tested
- You will be credited in the security advisory (unless you prefer to remain anonymous)
- A CVE may be requested for significant vulnerabilities

**If Declined:**

- We will provide a detailed explanation of why the report was not accepted
- You may request a second review if you disagree

## Security Best Practices

When deploying RustCMS:

1. **Environment Variables**: Never commit secrets to version control
   - Use `.env` files (excluded from git)
   - Set `DATABASE_URL`, `BISCUIT_ROOT_KEY`, and other secrets via environment

2. **Authentication**:
   - Use strong, unique keys for Biscuit authentication
   - Rotate API keys regularly
   - Enable HTTPS in production

3. **Database**:
   - Use SSL/TLS for database connections in production
   - Apply principle of least privilege to database users
   - Keep PostgreSQL updated

4. **Dependencies**:
   - Run `cargo audit` regularly
   - Keep dependencies updated
   - Review security advisories

5. **Docker**:
   - Use `Dockerfile.security` for production deployments
   - Run as non-root user (UID 10001)
   - Keep base images updated

## Security Features

RustCMS includes the following security features:

- **Biscuit Token Authentication**: Cryptographically secure token-based auth
- **Argon2 Password Hashing**: Industry-standard password hashing
- **Rate Limiting**: Protection against brute force attacks
- **Input Validation**: Using `garde` and `validator` crates
- **SQL Injection Prevention**: Diesel ORM with prepared statements
- **Pure Rust TLS**: Using `rustls` (no OpenSSL dependencies)
- **Secure Defaults**: Security-focused configuration out of the box

## Dependency Security

We use the following tools to maintain dependency security:

- **cargo-audit**: Automated vulnerability scanning
- **cargo-deny**: License and security policy enforcement
- **Dependabot**: Automated dependency updates (when available)

Run security checks locally:

```bash
cargo audit
cargo deny check
```

## Disclosure Policy

- Vulnerabilities are disclosed after a patch is available
- Critical vulnerabilities may be disclosed with mitigations before a full patch
- We follow responsible disclosure principles

## Contact

For security-related questions not related to vulnerabilities, please open a regular GitHub issue or discussion.
