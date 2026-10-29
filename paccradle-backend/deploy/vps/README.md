# paccradle VPS: Docker + Nginx + Let's Encrypt

One VPS (Contabo) runs every production app and the development API. Each app
is a Docker container (plain `docker run`, no Compose) that listens on
`127.0.0.1` only. Nginx on the host is the only way in: it terminates TLS,
forces HTTPS, writes JSON access logs and proxies each domain to its container.

| Domain | App | Container | Loopback port | Deployed from |
|---|---|---|---|---|
| `paccradle.com` (`www.` redirects) | landing page | `paccradle-landing-production` | 3001 | `paccradle-landing`, branch `main` |
| `app.paccradle.com` | PWA | `paccradle-app-production` | 3000 | `paccradle-frontend-app`, branch `main` |
| `api.paccradle.com` | API | `paccradle-api-production` | 8080 | `paccradle-backend`, branch `main` |
| `api-dev.paccradle.com` | API (development) | `paccradle-api-development` | 8081 | `paccradle-backend`, branch `development` |
| `paccradle-app.netlify.app`, `paccradle.netlify.app` | PWA and landing page (development) | Netlify | n/a | `development` branches |

```text
Browser ──HTTPS──▶ Nginx on the host (TLS, HSTS, JSON logs, rate limits)
                    ├─ paccradle.com          → 127.0.0.1:3001  paccradle-landing-production
                    ├─ app.paccradle.com      → 127.0.0.1:3000  paccradle-app-production
                    ├─ api.paccradle.com      → 127.0.0.1:8080  paccradle-api-production
                    └─ api-dev.paccradle.com  → 127.0.0.1:8081  paccradle-api-development

GitHub Actions ──SSH, key pinned to one app──▶ paccradle-remote ─▶ docker load
                                                              └▶ paccradle-deploy (swap, health check, rollback)
```

Domains, ports and limits come from `/etc/paccradle/vps.env`; nothing in the
templates is hard-coded. The per-repository pipelines are described in
`DEPLOYMENT.md` of each repository.

## What's in this folder

| Path | Installed as | Purpose |
|---|---|---|
| `vps.env.example` | `/etc/paccradle/vps.env` | domains, ports, container limits, log retention |
| `install.sh` | (run it) | installs everything below; safe to rerun |
| `issue-certificates.sh` | (run it) | Let's Encrypt certificates for every site |
| `bin/paccradle-deploy` | `/usr/local/bin/` | runs containers: deploy, rollback, status, history |
| `bin/paccradle-remote` | `/usr/local/bin/` | forced command for the CI keys (load, deploy, rollback, status) |
| `bin/paccradle-logs` | `/usr/local/bin/` | interactive log viewer (tail, find a request, GoAccess) |
| `nginx/conf.d/00-paccradle.conf.template` | `/etc/nginx/conf.d/00-paccradle.conf` | JSON log format, rate-limit zones, catch-all servers |
| `nginx/sites/*.conf.template` | `/etc/nginx/conf.d/paccradle-<site>.conf` | one file per site: landing, app, api, api-dev |
| `nginx/snippets/*.conf` | `/etc/nginx/snippets/paccradle/` | TLS, HSTS, proxy headers, ACME, server defaults |
| `logrotate/paccradle-nginx.template` | `/etc/logrotate.d/paccradle-nginx` | daily rotation, 183 days kept |
| `letsencrypt/paccradle-reload-nginx.sh` | `/etc/letsencrypt/renewal-hooks/deploy/` | reloads Nginx after a renewal |
| `../env/api.*.env.example` | `/etc/paccradle/env/api.<env>.env` | the API's settings and secrets |

## 1. Before you start

- **DNS:** `A` records (and `AAAA` if the VPS has IPv6) for `paccradle.com`,
  `www.paccradle.com`, `app.paccradle.com`, `api.paccradle.com` and
  `api-dev.paccradle.com`, all pointing at the VPS. Check with
  `dig +short api.paccradle.com`. If you use Cloudflare DNS, keep these records
  **DNS only** (grey cloud): this setup terminates TLS itself.
- **Optional, recommended:** a CAA record so only Let's Encrypt may issue
  certificates for the domain: `paccradle.com. CAA 0 issue "letsencrypt.org"`.
- **VPS:** Ubuntu 24.04 or 26.04 LTS, and root SSH access with your key.

## 2. Server basics (once)

Run as root. Replace `admin-paccradle` with the name you want for your own account.

```bash
apt update && apt full-upgrade -y
timedatectl set-timezone UTC
adduser --gecos "" admin-paccradle && usermod -aG sudo admin-paccradle
install -d -m 700 -o admin-paccradle -g admin-paccradle /home/admin-paccradle/.ssh
install -m 600 -o admin-paccradle -g admin-paccradle ~/.ssh/authorized_keys /home/admin-paccradle/.ssh/authorized_keys
```

Lock SSH down to keys (open a **second** session as `admin-paccradle` to check it
works before you close the root one):

```bash
cat > /etc/ssh/sshd_config.d/10-paccradle.conf <<'EOF'
PermitRootLogin no
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
X11Forwarding no
AllowAgentForwarding no
MaxAuthTries 3
EOF
sshd -t && systemctl reload ssh
```

Firewall, automatic security updates and SSH brute-force protection:

```bash
apt install -y ufw unattended-upgrades fail2ban
ufw default deny incoming && ufw default allow outgoing
ufw allow OpenSSH && ufw allow 80/tcp && ufw allow 443/tcp
ufw enable
dpkg-reconfigure -plow unattended-upgrades
```

Docker publishes container ports past UFW. That is why every paccradle
container publishes on `127.0.0.1` only: nothing but Nginx can reach it.

## 3. Install Docker, Nginx, certbot and tools

**Docker Engine** (Docker's own repository; Compose is not needed):

```bash
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
chmod a+r /etc/apt/keyrings/docker.asc
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" \
  > /etc/apt/sources.list.d/docker.list
apt update && apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin
```

Daemon defaults: compressed, size-capped container logs; containers keep
running while Docker itself is upgraded; no privilege escalation; containers
can't talk to each other.

```bash
cat > /etc/docker/daemon.json <<'EOF'
{
  "log-driver": "local",
  "log-opts": { "max-size": "20m", "max-file": "5" },
  "live-restore": true,
  "no-new-privileges": true,
  "icc": false
}
EOF
systemctl restart docker
```

**Nginx** from nginx.org's stable repository. The sites use `http2 on`, which
needs Nginx 1.25.1 or newer (Ubuntu 24.04's own package is 1.24):

```bash
apt install -y curl gnupg2 ca-certificates lsb-release ubuntu-keyring
curl -fsSL https://nginx.org/keys/nginx_signing.key | gpg --dearmor -o /usr/share/keyrings/nginx-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/nginx-archive-keyring.gpg] https://nginx.org/packages/ubuntu $(lsb_release -cs) nginx" \
  > /etc/apt/sources.list.d/nginx.list
printf 'Package: *\nPin: origin nginx.org\nPin: release o=nginx\nPin-Priority: 900\n' > /etc/apt/preferences.d/99nginx
apt update && apt install -y nginx
systemctl enable --now nginx
nginx -v
```

**certbot and the tools** the scripts use (`envsubst` is in `gettext-base`,
`flock` in `util-linux`):

```bash
apt install -y certbot jq gettext-base goaccess logrotate util-linux
systemctl list-timers | grep certbot   # renewals run on their own, twice a day
```

## 4. Install the paccradle files

From the `paccradle-backend` checkout on your computer (run it again whenever
these files change):

```bash
rsync -a --delete deploy/vps/ admin@VPS_IP:~/paccradle-vps/
```

On the VPS:

```bash
cd ~/paccradle-vps
sudo ./install.sh                 # first run: creates /etc/paccradle/vps.env and stops
sudo nano /etc/paccradle/vps.env    # check domains, ports and CERTBOT_EMAIL
sudo ./install.sh                 # tools, deploy user, Nginx (HTTP only for now), log rotation
```

`install.sh` creates the `deploy` user (no password, SSH keys only, member of
the `docker` group), installs the three commands, renders one Nginx file per
site and reloads Nginx. Before it changes anything it backs up the current
files to `/var/backups/paccradle-nginx/<time>/`, and if `nginx -t` fails it puts
them back. It also disables the distribution's default site, whose
`default_server` would clash with paccradle's.

## 5. HTTPS certificates (free, Let's Encrypt)

Until a site has its certificate, `install.sh` gives it an HTTP-only stub
that answers Let's Encrypt's challenge and nothing else. Then:

```bash
sudo CERTBOT_STAGING=1 ./issue-certificates.sh   # optional rehearsal on the staging CA...
sudo certbot delete --cert-name api.paccradle.com  # ...then delete each staging certificate
sudo ./issue-certificates.sh                     # real certificates (ECDSA P-256)
sudo ./install.sh                                # every site switches to HTTPS
```

Running `issue-certificates.sh` accepts the Let's Encrypt Subscriber Agreement
for `CERTBOT_EMAIL`. Renewal is automatic (`certbot.timer`); the hook reloads
Nginx so the new certificate is served. Check with:

```bash
sudo certbot certificates
sudo certbot renew --dry-run
curl -sI http://api.paccradle.com | head -n 3          # 301 to https://
curl -sI https://api.paccradle.com/healthz       # 502/503 until the API is deployed
```

What the TLS setup gives you (expect an **A+** at https://www.ssllabs.com/ssltest/):

- TLS 1.3, and TLS 1.2 with ECDHE key exchange and AEAD ciphers only (Mozilla
  "intermediate"); session tickets off; ECDSA certificates.
- HTTP answers only Let's Encrypt challenges and `301`s everything else to HTTPS.
- HSTS for two years with `includeSubDomains` (and `preload`, as the landing
  page already declares): browsers refuse plain HTTP for every subdomain.
- Unknown host names and bare-IP requests get nothing: port 80 closes the
  connection, port 443 rejects the TLS handshake, so no certificate leaks.
- No OCSP stapling: Let's Encrypt no longer runs OCSP.

## 6. Deploy keys for GitHub Actions

One key per repository and environment. Each key can only run
`paccradle-remote`, and only for its own app and environment: the landing
page's key can't touch the API, and the development key can't touch
production. Create them on your computer:

```bash
for target in api-production api-development app-production landing-production; do
  ssh-keygen -t ed25519 -N "" -C "gha-paccradle-$target" -f ~/.ssh/gha-paccradle-$target
done
```

Authorize them on the VPS, each pinned to its target:

```bash
for target in api:production api:development app:production landing:production; do
  printf 'restrict,command="/usr/local/bin/paccradle-remote %s" %s\n' \
    "$target" "$(cat ~/.ssh/gha-paccradle-${target/:/-}.pub)"
done | ssh admin@VPS_IP 'sudo tee -a /home/deploy/.ssh/authorized_keys > /dev/null'
```

Pin the VPS's host key for the workflows (secret `VPS_SSH_KNOWN_HOSTS`).
Compare the fingerprint with the one the VPS reports before you trust it:

```bash
ssh-keyscan -t ed25519 VPS_IP > vps_known_hosts && ssh-keygen -lf vps_known_hosts
ssh admin@VPS_IP 'ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub'
```

Check a key: `status` works, anything else is refused.

```bash
ssh -i ~/.ssh/gha-paccradle-api-production deploy@VPS_IP status
ssh -i ~/.ssh/gha-paccradle-api-production deploy@VPS_IP bash   # ERROR: unsupported request
```

The private key files go into GitHub (each repository's `DEPLOYMENT.md` lists
where); delete them from your computer afterwards if you like, new ones are
easy to make.

## 7. API settings (production and development)

The API reads its settings from one env file per environment, passed to
`docker run --env-file`. They must differ: separate database, bucket, secrets,
VAPID keys and Paddle account (sandbox for development).

```bash
# on your computer, from paccradle-backend
scp deploy/env/api.production.env.example deploy/env/api.development.env.example admin@VPS_IP:/tmp/
# on the VPS
sudo install -m 0640 -o root -g deploy /tmp/api.production.env.example /etc/paccradle/env/api.production.env
sudo install -m 0640 -o root -g deploy /tmp/api.development.env.example /etc/paccradle/env/api.development.env
rm /tmp/api.*.env.example
sudoedit /etc/paccradle/env/api.production.env
sudoedit /etc/paccradle/env/api.development.env
```

- No quotes around values: `docker --env-file` would keep them (`paccradle-deploy`
  refuses such a file).
- `APP_ENV` must match the file (`production` / `development`).
- `RATE_LIMIT_CLIENT_IP_HEADER=x-real-ip` must stay: Nginx overwrites that
  header with the real client address.
- Secrets: `openssl rand -hex 32` for `JWT_SECRET` and `FREE_COACHING_HASH_KEY`,
  `cargo run --bin generate_vapid_keys` for a VAPID pair.
- A Postgres on the VPS itself is reachable from the container as
  `host.docker.internal`.

A changed env file takes effect when the container is recreated: redeploy the
running version (step 8) or run the Deploy workflow again.

## 8. Day to day

Run the commands as the `deploy` user, so files under `/var/lib/paccradle` keep
the right owner.

```bash
sudo -u deploy paccradle-deploy status                       # every paccradle container, version, ports
sudo -u deploy paccradle-deploy history api production       # last 20 deploys and rollbacks
sudo -u deploy paccradle-deploy rollback api production      # back to the previous version
sudo -u deploy paccradle-deploy rollback api production 1.3.0
sudo -u deploy paccradle-deploy deploy api production 1.4.0  # recreate (e.g. after an env change)
docker image ls --filter label=com.paccradle.app \
  --format 'table {{.Repository}}:{{.Tag}}\t{{.Label "com.paccradle.environment"}}\t{{.CreatedAt}}'
```

How a deploy works: the running container stops (the API gets 35 s to finish
requests and jobs), the new one starts on the same port, and `paccradle-deploy`
waits up to `HEALTH_TIMEOUT_SECONDS` for `/readyz` (API) or `/healthz`
(static sites). If it doesn't pass, the previous container starts again and
the deploy fails, so Telegram gets an alert. The swap takes a few seconds;
meanwhile the API answers a JSON `503` with `Retry-After: 5`. The last
`KEEP_IMAGES` images per app and environment stay on the server for rollbacks.

Every container runs as a non-root user with a read-only filesystem, no Linux
capabilities, `no-new-privileges`, and memory, CPU and process limits.

### Logs

Nginx writes one JSON line per request to
`/var/log/nginx/paccradle/<domain>.access.log` (errors to `<domain>.error.log`),
without query strings, cookies or `Authorization` headers. Each line carries
the API's `request_id`, the ID the app shows next to an error, so a user's
report leads straight to its log line.

```bash
sudo paccradle-logs sites                     # sizes and the oldest day kept
sudo paccradle-logs tail api                  # live, coloured by status
sudo paccradle-logs tail app errors           # only 4xx and 5xx (also: 5xx, slow)
sudo paccradle-logs find 01a16c90-5750-7077-9fed-d7fbe738152f   # one request, rotated logs too
sudo paccradle-logs slow api 0.5 7            # slowest requests over 0.5 s, last 7 days
sudo paccradle-logs top landing 30            # interactive GoAccess dashboard, last 30 days (q quits)
sudo paccradle-logs report api /tmp/api.html 7   # the same as an HTML file (scp it to open)
sudo paccradle-logs errors api                # Nginx error log, live
sudo -u deploy paccradle-logs app api production # the API's own logs (JSON, pretty-printed)
```

Plain `jq` works on the files too:
`sudo jq 'select(.status >= 500)' /var/log/nginx/paccradle/api.paccradle.com.access.log`.

Retention:

- **Nginx logs:** rotated daily, compressed, deleted after
  `NGINX_LOG_RETENTION_DAYS` (183 days, about six months). Change the value
  and rerun `install.sh` to adjust.
- **Container logs:** Docker's `local` driver (compressed), at most
  `CONTAINER_LOG_MAX_SIZE` × `CONTAINER_LOG_MAX_FILES` per container
  (20 MB × 10 by default).
- Check space with `sudo du -sh /var/log/nginx/paccradle` and `docker system df`.

### Changing the Nginx setup

Edit the templates in `deploy/vps/nginx/` (or `/etc/paccradle/vps.env`), copy
the folder over again (the `rsync` in step 4) and rerun `sudo ./install.sh`. It tests the result and
restores the previous files if the test fails. Domains or ports changed in
`vps.env` need a redeploy of the affected app too.

## Security checklist

- [ ] SSH: keys only, no root login; `fail2ban` running.
- [ ] UFW: only 22, 80 and 443 open.
- [ ] Every container on `127.0.0.1` (`sudo -u deploy paccradle-deploy status` shows `127.0.0.1:` ports).
- [ ] `/etc/paccradle/env/*.env`: mode `0640`, owner `root`, group `deploy`; production and development secrets differ.
- [ ] CI keys only in `~deploy/.ssh/authorized_keys`, each with `restrict,command=...`. The `deploy` account is in the `docker` group (root-equivalent): never give it a password or an unrestricted key.
- [ ] SSL Labs: A+ for every domain; `certbot renew --dry-run` passes.
- [ ] Optional: submit `paccradle.com` to https://hstspreload.org once every subdomain is permanently HTTPS.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| `install.sh`: Nginx is too old | install nginx.org's package (step 3) |
| `nginx -t`: `bind() to [::]:80 failed (97: Address family not supported)` | the VPS has no IPv6: set `NGINX_IPV6=false`, rerun `install.sh` |
| certbot: challenge failed | DNS not pointing here yet, port 80 blocked, or an `AAAA` record pointing elsewhere |
| deploy: `image ... is not on this server` | the upload step failed; read its log in the workflow run |
| deploy: `... has quoted values` | remove the quotes in `/etc/paccradle/env/api.<env>.env` |
| deploy: `... failed its health check; rolled back` | `sudo -u deploy paccradle-logs app api production`; for the API, `/readyz` fails when PostgreSQL is unreachable (`DATABASE_URL`) |
| `502`/`503` from a site | `sudo -u deploy paccradle-deploy status`; then the site's error log (`sudo paccradle-logs errors api`) |
| SSH from CI: `Host key verification failed` | the VPS host key changed (reinstall?): check it, then update `VPS_SSH_KNOWN_HOSTS` |
