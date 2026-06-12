# Releasing

Releases are driven by git tags. Pushing a `v*` tag runs
`.github/workflows/release.yml`, which:

1. Builds `mkt` binaries for 5 targets (Linux musl x86_64/aarch64,
   macOS x86_64/aarch64, Windows x86_64).
2. Creates a GitHub Release with the packaged archives.
3. Publishes all workspace crates to crates.io in dependency order
   (`mkt-cli-core` → providers → `mkt-cli`), skipping versions that
   already exist.

## One-time setup: crates.io Trusted Publishing

The `crates` job authenticates via [Trusted Publishing](https://crates.io/docs/trusted-publishing)
(GitHub OIDC) — no API token is stored in the repository. Each crate must be
configured once on crates.io:

1. Open `https://crates.io/crates/<crate>/settings` for each of:
   `mkt-cli-core`, `mkt-meta`, `mkt-google`, `mkt-tiktok`, `mkt-linkedin`, `mkt-cli`.
2. Under **Trusted Publishing**, add a GitHub publisher:
   - Repository owner: `diorrego`
   - Repository name: `mkt-cli`
   - Workflow filename: `release.yml`
3. Optionally enable **require Trusted Publishing** to disable token-based
   publishes entirely (recommended once CI publishing is verified).

Until that is configured (or if GitHub Actions is unavailable), publish
manually in the same order:

```sh
for crate in mkt-cli-core mkt-meta mkt-google mkt-tiktok mkt-linkedin mkt-cli; do
  cargo publish -p "$crate" --locked
done
```

## Release checklist

1. Start from a clean `main` with CI green.
2. Move the `Unreleased` section of `CHANGELOG.md` under the new version
   with today's date.
3. Bump `workspace.package.version` in the root `Cargo.toml` **and** the
   `version` field of every internal `mkt-*` path dependency
   (`crates/*/Cargo.toml`), then run `cargo build` so `Cargo.lock` updates.
4. Run the full gate locally:
   ```sh
   cargo fmt --all --check
   cargo clippy --all-targets --all-features
   cargo test --workspace
   ```
5. Commit on a release branch, merge to `main`, then tag and push:
   ```sh
   git tag -a vX.Y.Z -m "vX.Y.Z — summary"
   git push origin main vX.Y.Z
   ```
6. Verify the Release workflow: binaries attached to the GitHub Release
   and new versions visible on crates.io.
7. Regenerate `mktcli.com/public/llms-full.txt` from the updated README,
   AGENTS.md, and CHANGELOG so AI agents read current docs.

`mkt-testkit` is a path-only dev-dependency and is intentionally **not**
published.
