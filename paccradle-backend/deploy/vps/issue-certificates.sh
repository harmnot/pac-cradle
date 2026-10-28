#!/usr/bin/env bash
# Requests free Let's Encrypt certificates (ECDSA P-256, renewed automatically
# by certbot's timer, Nginx reloaded by the hook install.sh adds) for every
# site in NGINX_SITES that has none yet. HTTP-01 through the webroot that
# install.sh serves: run install.sh first, and again afterwards to switch the
# new sites to HTTPS.
#
#   sudo ./issue-certificates.sh
#   sudo CERTBOT_STAGING=1 ./issue-certificates.sh   # rehearse on the staging CA
#
# A staging certificate counts as "present": `certbot delete --cert-name <name>`
# before requesting the real one. Running this accepts the Let's Encrypt
# Subscriber Agreement for CERTBOT_EMAIL.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
config="${paccradle_CONFIG:-/etc/paccradle/vps.env}"
readonly ACME_WEBROOT=/var/www/letsencrypt

# shellcheck source-path=SCRIPTDIR source=lib/sites.sh
source "$here/lib/sites.sh"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
info() { printf '==> %s\n' "$*"; }

[[ $EUID -eq 0 ]] || die "run as root: sudo $0"
[[ -r $config ]] || die "$config is missing: run install.sh first"
set -a
# shellcheck source=/dev/null
source "$config"
set +a
[[ -n ${CERTBOT_EMAIL:-} ]] || die "CERTBOT_EMAIL is not set in $config"
[[ -d $ACME_WEBROOT ]] || die "$ACME_WEBROOT is missing: run install.sh first"

issued=0
for site in $NGINX_SITES; do
  certificate="$(site_certificate "$site")"
  if site_has_certificate "$site"; then
    info "$site: $certificate already has a certificate"
    continue
  fi
  read -r -a names <<< "$(site_server_names "$site")"
  args=(certonly --webroot --webroot-path "$ACME_WEBROOT" --cert-name "$certificate"
    --key-type ecdsa --elliptic-curve secp256r1
    --email "$CERTBOT_EMAIL" --agree-tos --no-eff-email --non-interactive)
  for name in "${names[@]}"; do
    args+=(--domain "$name")
  done
  if [[ -n ${CERTBOT_STAGING:-} ]]; then
    args+=(--staging)
  fi
  info "$site: requesting a certificate for ${names[*]}"
  certbot "${args[@]}"
  issued=$((issued + 1))
done

if (( issued > 0 )); then
  info "now run: sudo $here/install.sh (switches those sites to HTTPS)"
fi
