# Image releases

`ci-railway` is a trusted, disposable **Linux amd64 Bookworm** job container. It builds on the Forgejo host; Railway runs only the Bookworm slim runtime image. No container socket or nested Podman/Docker is needed. Keep the existing `ci-rust` and `node-24` labels.

## Provision once

- Include Rust from `rust-toolchain.toml`, native C/C++ build tools, CMake, Node 24, Git, Bash, GNU tar/coreutils, CA certificates, curl, OpenSSH client/agent, `setsid`, `psql`, SQLx CLI 0.9.0 with PostgreSQL support, Tailwind CLI 4.3.3 (`tailwindcss`), Railway CLI with `connect --tunnel-only` (5.74.1 supports it), and `crane` with `mutate --append`. Use the native `x86_64-unknown-linux-gnu` target. Bake tools into the runner rather than installing them on each release.
- Set Forgejo repository variable `RAILWAY_PROJECT_ID`. Keep `RAILWAY_SERVICE` (the app, not Postgres), `RAILWAY_STAGING_TOKEN` and `RAILWAY_TOKEN` (environment-scoped project tokens for deployment). Database service names are `Postgres-staging` in `staging` and `Postgres` in `production`. Set the app's runtime `DATABASE_URL` from its database's private URL.
- For tunnels, provision `TUNNEL_API_TOKEN` (a Railway account/workspace API token with access to both targets and its registered SSH keys) and `TUNNEL_SSH_KEY` (the corresponding multiline OpenSSH private key, usable noninteractively). Register the public key with Railway first. The release script loads the key into a temporary SSH agent. Its tunnel log contains credentials and is deleted without being printed. No database public endpoint or stored database password is needed.
- Set `REGISTRY_USER` and `REGISTRY_TOKEN` for Forgejo container and generic package read/write access in the repository owner's namespace. Give Railway separate **read-only** registry credentials for a private image (requires Railway Pro); ensure the registry is reachable from Railway. Keep the image manifests/layers and staging receipts for as long as releases can be promoted or rolled back. Do not overwrite or delete receipts.
- Scope deployment and package secrets to trusted workflows. Leave automatic image updates off. The deployment script sets the image source on the target service **instance**, `/app/ixirsii-tech` as its command, `/health` with a 300-second timeout, and restart-on-failure. The image sets `AUTH_CLIENT_IP_MODE=railway`. Keep environment-specific variables on the Railway service.

## Verify before the first release

1. Dispatch **Linux image smoke test** from a trusted ref. It compiles on the actual release runner against disposable PostgreSQL, packages the binary with `crane`, and starts the published image as a Linux service alongside another disposable database. `/health` verifies that the binary loads, starts, and can query PostgreSQL. No Railway environment is involved.
2. Run a staging deployment to verify unattended SSH authentication and Railway's ability to pull a digest-qualified image from the registry. The workflow fails if Railway rejects the reference or the deployment fails health checks. Do not substitute `latest`.
3. After a successful staging run, Forgejo stores the commit and image digest in a generic package named `<repository>-staging`, versioned by staging run ID. A production tag resolves the successful run for that exact commit, reads its receipt and checks the image before touching production. Missing receipts (including commits staged only by the old workflow) require a successful new staging run.

## Release ordering

Both jobs run migrations before compiling against a live schema. Staging publishes and deploys a release binary; production runs `cargo check --release --locked`, then deploys the recorded digest without rebuilding. Failed checks stop deployment. Migrations already applied remain in place. Design migrations for the previous binary as well as the new one; reverting an image does not revert the schema.
