# Deploying the paccradle API

| Branch | Environment | Runs on | URL |
|---|---|---|---|
| `main` | production | VPS, container `paccradle-api-production` | https://api.paccradle.com |
| `development` | development | VPS, container `paccradle-api-development` | https://api-dev.paccradle.com |

Both run with plain Docker behind Nginx, which handles HTTPS. The server
itself (Docker, Nginx, Let's Encrypt, logs, SSH keys) is set up once with
[`deploy/vps/README.md`](deploy/vps/README.md), which also covers the PWA and
the landing page.

## What happens on a push

`.github/workflows/deploy.yml` runs on every push to `main` or `development`
(and from **Actions → Deploy → Run workflow** to redeploy a branch). Pushes
that only touch docs, `postman/`, `deploy/`, `scripts/dev/` or editor folders
don't deploy.

1. **Resolve version**: the branch picks the environment and `Cargo.toml` the
   version (see below).
2. **CI**: `ci.yml` (fmt, clippy, tests against Postgres with the `.sqlx`
   check, cargo audit). Nothing deploys unless all of it passes. Pull requests
   run the same checks.
3. **Build image**: Docker Buildx builds `paccradle-api:<version>`. Compiled
   dependencies are cached between runs, so a code change only recompiles
   this crate. `.dockerignore` lets in only `Cargo.*`, `build.rs`, `.sqlx`,
   `migrations` and `src`.
4. **Deploy**: the image goes to the VPS over SSH and `paccradle-deploy` swaps
   the container: stop the old one, start the new one, wait for
   `/readyz`. If that fails, the previous version starts again and the
   job fails. Then a public smoke test calls `/healthz` through Nginx
   and TLS.
5. **Tag the release** (production only): git tag `v<version>` on the commit.
6. **Telegram**: a summary on success, a short alert on failure (below).

One deploy per environment runs at a time; a newer push waits and never
cancels a deploy halfway. Migrations run when the new container starts
(`DATABASE_RUN_MIGRATIONS=true`).

## Versions

- **Production** uses the version in `Cargo.toml` as is, e.g. `1.4.0`
  (semver: patch for fixes, minor for new endpoints or fields, major for
  breaking API changes). This is the release's version, shown by `/healthz`.
  The API contract has its own semver per surface (client, admin): the URL
  holds its major (`/api/client/v1`), the `api-version` header the full one
  (`src/shared/api_surface.rs`). Each production release needs a new version:
  pushing another commit to `main` with a version that is already tagged
  fails at **Resolve version** with "Bump the version". After editing
  `Cargo.toml`, run `cargo check` so `Cargo.lock` is updated too (the build
  uses `--locked`), and commit both.
- **Development** builds get a pre-release version, `1.4.0-dev.<run number>`.

Every image carries its metadata as labels:

| Label | Example |
|---|---|
| `org.opencontainers.image.version`, `com.paccradle.version` | `1.4.0`, `1.4.0-dev.57` |
| `com.paccradle.environment` | `production` or `development` |
| `com.paccradle.app` | `api` |
| `org.opencontainers.image.revision` | the git commit |
| `org.opencontainers.image.created` | build time (UTC) |
| `org.opencontainers.image.title`, `.description`, `.source` | what it is and where it comes from |

```bash
docker inspect paccradle-api:1.4.0 --format '{{json .Config.Labels}}' | jq
```

## One-time setup

1. **The VPS**: follow [`deploy/vps/README.md`](deploy/vps/README.md) (steps 1 to 7),
   including both API env files, `/etc/paccradle/env/api.production.env` and
   `/etc/paccradle/env/api.development.env` (templates in [`deploy/env/`](deploy/env)).
   Since M1-D3 production also needs `AUTH_JWT_SECRET` (`openssl rand -base64 48`;
   the template's placeholder is refused), the SMTP settings and `EMAIL_FROM`, and
   `WEBAUTHN_RP_ID` set to the domain the web app and the back office share
   (`paccradle.com`). Never change `WEBAUTHN_RP_ID` after launch: saved passkeys
   would stop working.
2. **Commit `.sqlx`**: the image and CI compile with `SQLX_OFFLINE=true`, so
   every query's cache file must be in git (`git add .sqlx`). Run
   `cargo sqlx prepare -- --all-targets` after changing a query.
3. **The `development` branch**: `git switch -c development && git push -u origin development`.
4. **GitHub environments, secrets and variables** (below).
5. **Paddle webhooks**: point the live notification destination at
   `https://api.paccradle.com/api/webhooks/v1/paddle` and the sandbox one at
   `https://api-dev.paccradle.com/api/webhooks/v1/paddle`.

### GitHub: environments, secrets, variables

Create the environments `production` (deploys only from `main`; add yourself
as a required reviewer if you want to approve each production deploy) and
`development` (deploys only from `development`).

| Name | Kind | Scope | Value |
|---|---|---|---|
| `VPS_HOST` | secret | repository | the VPS's IP or host name |
| `VPS_SSH_PORT` | secret | repository | SSH port (optional, default 22) |
| `VPS_SSH_USER` | secret | repository | `deploy` |
| `VPS_SSH_KNOWN_HOSTS` | secret | repository | the checked `ssh-keyscan` line (VPS guide, step 6) |
| `VPS_SSH_PRIVATE_KEY` | secret | environment `production` | `~/.ssh/gha-paccradle-api-production` |
| `VPS_SSH_PRIVATE_KEY` | secret | environment `development` | `~/.ssh/gha-paccradle-api-development` |
| `DEPLOY_URL` | variable | environment `production` | `https://api.paccradle.com` |
| `DEPLOY_URL` | variable | environment `development` | `https://api-dev.paccradle.com` |
| `TELEGRAM_BOT_BEGAMIS_TOKEN` | secret | repository | bot that posts successful deploys |
| `TELEGRAM_CHANNEL_CHAT_IDS` | secret | repository | channel ID(s) for those, comma-separated: `-1001390000000,-1001390000001` |
| `TELEGRAM_BOT_BEGAMIS_ALERT_DEV_TOKEN` | secret | repository | bot that posts failures |
| `TELEGRAM_CHANNEL_ALERT_CHAT_ID` | secret | repository | channel ID for failures |

With the GitHub CLI (`brew install gh`, then `gh auth login`); `gh secret set`
without `--body` asks for the value, so it never lands in your shell history:

```bash
REPO=harmnot/paccradle-backend   # owner/name on GitHub

# environment:branch it may deploy from
for pair in production:main development:development; do
  gh api -X PUT "repos/$REPO/environments/${pair%%:*}" --input - <<'EOF'
{"deployment_branch_policy": {"protected_branches": false, "custom_branch_policies": true}}
EOF
  gh api -X POST "repos/$REPO/environments/${pair%%:*}/deployment-branch-policies" -f name="${pair#*:}" -f type=branch
done

gh secret set VPS_HOST --repo "$REPO"
gh secret set VPS_SSH_USER --repo "$REPO" --body deploy
gh secret set VPS_SSH_KNOWN_HOSTS --repo "$REPO" < vps_known_hosts
gh secret set VPS_SSH_PRIVATE_KEY --repo "$REPO" --env production < ~/.ssh/gha-paccradle-api-production
gh secret set VPS_SSH_PRIVATE_KEY --repo "$REPO" --env development < ~/.ssh/gha-paccradle-api-development
gh variable set DEPLOY_URL --repo "$REPO" --env production --body https://api.paccradle.com
gh variable set DEPLOY_URL --repo "$REPO" --env development --body https://api-dev.paccradle.com

for name in TELEGRAM_BOT_BEGAMIS_TOKEN TELEGRAM_CHANNEL_CHAT_IDS \
  TELEGRAM_BOT_BEGAMIS_ALERT_DEV_TOKEN TELEGRAM_CHANNEL_ALERT_CHAT_ID; do
  gh secret set "$name" --repo "$REPO"
done
```

The Telegram secrets are the same in all three repositories. Each bot must be
an admin of its channel. To find a channel's ID, post a message in it, then
open `https://api.telegram.org/bot<token>/getUpdates` and take
`channel_post.chat.id` (it starts with `-100`).

## Telegram messages

Both come with buttons. Success messages for development are sent silently;
only production pings phones.

```text
✅ paccradle API deployed to production

Version: 1.4.0 (was 1.3.2)
Commit: 3f9c2ab Add contest prep history
Branch: main · By: harmnot
Duration: 7m 41s
URL: https://api.paccradle.com
[🌐 Open] [📋 Run log]
[🔍 Commit] [🧾 Changes]
```

```text
🚨 paccradle API deploy failed · production

Version: 1.4.0
Failed at: Deploy to production → Deploy on the VPS
Reason: paccradle-api:1.4.0 failed its health check; rolled back to 1.3.2
Commit: 3f9c2ab Add contest prep history · harmnot
[📋 Open failed run]
[🔍 Commit]
```

A failure names the failed job and step and, when the scripts know it, the
reason (version not bumped, health check, missing setting). Cancelled runs
send nothing. If Telegram is down or not configured, the workflow only logs a
warning.

## Everyday tasks

| Task | How |
|---|---|
| Deploy development | push to `development` |
| Deploy production | bump `Cargo.toml` (and `Cargo.lock`), merge to `main` |
| Redeploy a branch | **Actions → Deploy → Run workflow**, pick the branch |
| Roll back | **Actions → Rollback → Run workflow** (from `main`), pick the environment, optionally a version |
| Change a setting or secret | `sudoedit /etc/paccradle/env/api.<env>.env` on the VPS, then redeploy the running version (`sudo -u deploy paccradle-deploy deploy api production 1.4.0`) |
| See what runs | `sudo -u deploy paccradle-deploy status` |
| Read logs | `sudo -u deploy paccradle-logs app api production`, `sudo paccradle-logs tail api` |

A rollback starts an older image; it does not undo migrations. Keep
migrations additive (new tables and nullable columns) so the previous release
still runs against the new schema.

## Building and running the image locally

```bash
docker build -t paccradle-api:local .
docker run --rm -p 127.0.0.1:8080:8080 --env-file .env.docker \
  -e APP_HOST=0.0.0.0 -e APP_PORT=8080 --add-host host.docker.internal:host-gateway \
  paccradle-api:local
```

`.env.docker` is a copy of `.env` without quotes and with
`DATABASE_URL=postgres://paccradle:change-me@host.docker.internal:5432/paccradle`.

## Files

| Path | Purpose |
|---|---|
| `.github/workflows/deploy.yml` | CI, build, deploy, tag, Telegram |
| `.github/workflows/rollback.yml` | manual rollback |
| `.github/workflows/ci.yml` | checks for pull requests and deploys |
| `.github/scripts/` | version, SSH and Telegram helpers (the same in all three repositories) |
| `Dockerfile`, `.dockerignore` | the image |
| `deploy/env/` | API env file templates (production, development) |
| `deploy/vps/` | VPS tooling, Nginx sites, log rotation ([README](deploy/vps/README.md)) |
