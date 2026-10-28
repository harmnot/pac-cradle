#!/usr/bin/env bash
# Installs the paccradle VPS tooling and Nginx configuration. Run as root from a
# copy of deploy/vps (README.md, step 5). Safe to run again after any change.
#
#   sudo ./install.sh
#
#   1. tools: paccradle-deploy, paccradle-remote, paccradle-logs → /usr/local/bin
#   2. the deploy user and the directories it uses
#   3. Nginx: shared settings, snippets and one file per site, rendered from
#      /etc/paccradle/vps.env. A site whose certificate doesn't exist yet gets an
#      HTTP-only stub for Let's Encrypt (issue-certificates.sh) and the full
#      HTTPS site on the next run
#   4. log rotation (NGINX_LOG_RETENTION_DAYS) and the certbot reload hook
#   5. `nginx -t`, then a reload; if the test fails the previous files return
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
config="${paccradle_CONFIG:-/etc/paccradle/vps.env}"
readonly NGINX_DIR=/etc/nginx
readonly LOG_DIR=/var/log/nginx/paccradle
readonly ACME_WEBROOT=/var/www/letsencrypt
readonly STATE_DIR=/var/lib/paccradle
readonly BACKUP_ROOT=/var/backups/paccradle-nginx

# Only these are substituted; Nginx's own $variables are left alone.
# shellcheck disable=SC2016
readonly TEMPLATE_VARIABLES='${LANDING_DOMAIN} ${LANDING_WWW_DOMAIN} ${APP_DOMAIN} ${API_DOMAIN} ${API_DEV_DOMAIN}
${LANDING_PRODUCTION_PORT} ${APP_PRODUCTION_PORT} ${API_PRODUCTION_PORT} ${API_DEVELOPMENT_PORT}
${NGINX_LOG_DIR} ${NGINX_USER} ${NGINX_LOG_RETENTION_DAYS} ${ACME_SERVER_NAMES} ${SITE_LOG_NAME}
${LISTEN_HTTP_IPV6} ${LISTEN_HTTPS_IPV6} ${LISTEN_HTTP_IPV6_DEFAULT} ${LISTEN_HTTPS_IPV6_DEFAULT}'

# shellcheck source-path=SCRIPTDIR source=lib/sites.sh
source "$here/lib/sites.sh"

die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
info() { printf '==> %s\n' "$*"; }

require_root_and_tools() {
  [[ $EUID -eq 0 ]] || die "run as root: sudo $0"
  local tool version
  for tool in nginx docker envsubst jq curl logrotate flock certbot; do
    command -v "$tool" > /dev/null || die "$tool is not installed (README.md, step 2)"
  done
  version="$(nginx -v 2>&1 | sed -n 's|^nginx version: nginx/\([0-9.]*\).*|\1|p')"
  printf '1.25.1\n%s\n' "$version" | sort -V -C \
    || die "Nginx $version is too old ('http2 on' needs 1.25.1+): install nginx.org's stable package (README.md, step 2)"
}

load_config() {
  if [[ ! -f $config ]]; then
    install -d -m 0755 "$(dirname "$config")"
    install -m 0644 "$here/vps.env.example" "$config"
    die "created $config from vps.env.example: check the domains and ports, then run this again"
  fi
  set -a
  # shellcheck source=/dev/null
  source "$config"
  set +a
  local name
  for name in LANDING_DOMAIN LANDING_WWW_DOMAIN APP_DOMAIN API_DOMAIN API_DEV_DOMAIN \
    LANDING_PRODUCTION_PORT APP_PRODUCTION_PORT API_PRODUCTION_PORT API_DEVELOPMENT_PORT \
    NGINX_SITES NGINX_LOG_RETENTION_DAYS DEPLOY_USER; do
    [[ -n ${!name:-} ]] || die "$name is not set in $config"
  done
}

# The worker user from nginx.conf: nginx (nginx.org packages) or www-data (Debian/Ubuntu).
detect_nginx_user() {
  local user
  user="$(sed -nE 's/^[[:space:]]*user[[:space:]]+([^[:space:];]+).*/\1/p' "$NGINX_DIR/nginx.conf" | head -n 1)"
  printf '%s' "${user:-nginx}"
}

export_template_values() {
  NGINX_USER="$(detect_nginx_user)"
  export NGINX_LOG_DIR="$LOG_DIR" NGINX_USER ACME_SERVER_NAMES="" SITE_LOG_NAME=""
  if [[ ${NGINX_IPV6:-true} == true ]]; then
    export LISTEN_HTTP_IPV6='listen [::]:80;'
    export LISTEN_HTTPS_IPV6='listen [::]:443 ssl;'
    export LISTEN_HTTP_IPV6_DEFAULT='listen [::]:80 default_server;'
    export LISTEN_HTTPS_IPV6_DEFAULT='listen [::]:443 ssl default_server;'
  else
    local off='# IPv6 is off (NGINX_IPV6=false)'
    export LISTEN_HTTP_IPV6="$off" LISTEN_HTTPS_IPV6="$off"
    export LISTEN_HTTP_IPV6_DEFAULT="$off" LISTEN_HTTPS_IPV6_DEFAULT="$off"
  fi
}

render() {
  envsubst "$TEMPLATE_VARIABLES" < "$1"
}

install_tools() {
  info "installing paccradle-deploy, paccradle-remote and paccradle-logs"
  install -m 0755 "$here/bin/paccradle-deploy" "$here/bin/paccradle-remote" "$here/bin/paccradle-logs" /usr/local/bin/
  install -d -m 0755 "$ACME_WEBROOT"
  install -d -m 0755 -o root -g adm "$LOG_DIR"
}

# The account GitHub Actions connects as. Its keys may only run paccradle-remote
# (README.md, step 4). It is in the docker group to start containers.
setup_deploy_user() {
  local home
  if ! id "$DEPLOY_USER" > /dev/null 2>&1; then
    info "creating user $DEPLOY_USER (no password: SSH keys only)"
    useradd --create-home --shell /bin/bash "$DEPLOY_USER"
  fi
  usermod --append --groups docker "$DEPLOY_USER"
  home="$(getent passwd "$DEPLOY_USER" | cut -d: -f6)"
  install -d -m 0700 -o "$DEPLOY_USER" -g "$DEPLOY_USER" "$home/.ssh"
  if [[ ! -f $home/.ssh/authorized_keys ]]; then
    install -m 0600 -o "$DEPLOY_USER" -g "$DEPLOY_USER" /dev/null "$home/.ssh/authorized_keys"
  fi
  install -d -m 0750 -o root -g "$DEPLOY_USER" /etc/paccradle/env
  install -d -m 0750 -o "$DEPLOY_USER" -g "$DEPLOY_USER" "$STATE_DIR" "$STATE_DIR/history" "$STATE_DIR/locks"
}

# Renders every Nginx file into $1, mirroring /etc/nginx.
render_nginx() {
  local staged=$1 site
  install -d "$staged/conf.d" "$staged/snippets/paccradle"
  render "$here/nginx/conf.d/00-paccradle.conf.template" > "$staged/conf.d/00-paccradle.conf"
  cp "$here"/nginx/snippets/*.conf "$staged/snippets/paccradle/"
  for site in $NGINX_SITES; do
    site_certificate "$site" > /dev/null || exit 1
    if site_has_certificate "$site"; then
      render "$here/nginx/sites/$site.conf.template" > "$staged/conf.d/paccradle-$site.conf"
      info "$site: HTTPS ($(site_server_names "$site"))"
    else
      ACME_SERVER_NAMES="$(site_server_names "$site")"
      SITE_LOG_NAME="$(site_certificate "$site")"
      render "$here/nginx/sites/acme-only.conf.template" > "$staged/conf.d/paccradle-$site.conf"
      PENDING_SITES+=("$site")
      info "$site: HTTP only until its certificate exists"
    fi
  done
}

# Moves the current paccradle files, and the distribution's default site (its
# default_server would clash with ours), into $1.
backup_current() {
  local backup=$1 file
  install -d -m 0700 "$backup/conf.d" "$backup/sites-enabled"
  for file in "$NGINX_DIR"/conf.d/00-paccradle.conf "$NGINX_DIR"/conf.d/paccradle-*.conf "$NGINX_DIR"/conf.d/default.conf; do
    [[ -e $file ]] && mv "$file" "$backup/conf.d/"
  done
  if [[ -e $NGINX_DIR/sites-enabled/default || -L $NGINX_DIR/sites-enabled/default ]]; then
    mv "$NGINX_DIR/sites-enabled/default" "$backup/sites-enabled/"
  fi
  if [[ -d $NGINX_DIR/snippets/paccradle ]]; then
    mv "$NGINX_DIR/snippets/paccradle" "$backup/snippets-paccradle"
  fi
  return 0
}

restore_backup() {
  local backup=$1
  rm -f "$NGINX_DIR"/conf.d/00-paccradle.conf "$NGINX_DIR"/conf.d/paccradle-*.conf
  rm -rf "$NGINX_DIR/snippets/paccradle"
  cp -a "$backup/conf.d/." "$NGINX_DIR/conf.d/"
  if [[ -e $backup/sites-enabled/default || -L $backup/sites-enabled/default ]]; then
    mv "$backup/sites-enabled/default" "$NGINX_DIR/sites-enabled/"
  fi
  if [[ -d $backup/snippets-paccradle ]]; then
    cp -a "$backup/snippets-paccradle" "$NGINX_DIR/snippets/paccradle"
  fi
}

apply_nginx() {
  local staged=$1 backup
  backup="$BACKUP_ROOT/$(date -u +%Y%m%dT%H%M%SZ)"
  backup_current "$backup"
  install -d "$NGINX_DIR/snippets"
  cp -a "$staged/snippets/paccradle" "$NGINX_DIR/snippets/paccradle"
  install -m 0644 "$staged"/conf.d/*.conf "$NGINX_DIR/conf.d/"
  if ! nginx -t; then
    restore_backup "$backup"
    die "nginx -t failed: the previous configuration is back (copy in $backup)"
  fi
  systemctl reload-or-restart nginx
  info "Nginx reloaded (previous files in $backup)"
}

install_log_rotation() {
  render "$here/logrotate/paccradle-nginx.template" > /etc/logrotate.d/paccradle-nginx
  chmod 0644 /etc/logrotate.d/paccradle-nginx
  logrotate --debug /etc/logrotate.d/paccradle-nginx > /dev/null 2>&1 \
    || die "logrotate rejects /etc/logrotate.d/paccradle-nginx (see: logrotate -d /etc/logrotate.d/paccradle-nginx)"
  info "log rotation: daily, kept $NGINX_LOG_RETENTION_DAYS days ($LOG_DIR)"
}

install_certbot_hook() {
  install -D -m 0755 "$here/letsencrypt/paccradle-reload-nginx.sh" \
    /etc/letsencrypt/renewal-hooks/deploy/paccradle-reload-nginx.sh
}

main() {
  require_root_and_tools
  load_config
  export_template_values
  install_tools
  setup_deploy_user

  local staged
  staged="$(mktemp -d)"
  trap 'rm -rf "$staged"' EXIT
  PENDING_SITES=()
  render_nginx "$staged"
  install_log_rotation
  install_certbot_hook
  apply_nginx "$staged"

  if (( ${#PENDING_SITES[@]} > 0 )); then
    info "waiting for certificates: ${PENDING_SITES[*]}"
    info "next: sudo $here/issue-certificates.sh && sudo $here/install.sh"
  else
    info "every site is on HTTPS"
  fi
}

main "$@"
