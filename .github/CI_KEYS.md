 # CI Biscuit keys

 # Purpose

 This document explains how to provide Biscuit key material to GitHub Actions for running integration tests.

 ## Recommended (secure): GitHub Secrets

 - Create two repository secrets:
   - `BISCUIT_PRIVATE_KEY_B64`
   - `BISCUIT_PUBLIC_KEY_B64`

 These should contain the base64-encoded 32-byte private/public key pair used by the `biscuit_auth` crate.
 When present the CI workflow will prefer these secrets and skip ephemeral key generation.

 ## Alternative (ephemeral keys generated in CI)

 - If you don't want to store keys in Secrets, the CI workflow will build `gen_biscuit_keys` and generate keys at runtime.
 - This is less secure (keys exist during the job), but acceptable for ephemeral integration test runs.

 ## How CI reads the keys

 - The workflow checks `BISCUIT_PRIVATE_KEY_B64` / `BISCUIT_PUBLIC_KEY_B64` env vars first (populated from `secrets.*`), and if absent it runs the binary `gen_biscuit_keys` to generate and export them into the job's environment via `GITHUB_ENV`.

 ## Notes

 - If you add secrets, go to: [Repository secrets settings](https://github.com/jungamer-64/RustCMS/settings/secrets/actions) and add the two secrets.
 - Prefer secrets for CI on protected branches and main to avoid accidental leakage.
