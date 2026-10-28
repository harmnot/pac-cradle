# shellcheck shell=bash
# The paccradle sites, shared by install.sh and issue-certificates.sh.
# Needs /etc/paccradle/vps.env loaded first.

# Prints the Let's Encrypt certificate name of a site (its main domain).
site_certificate() {
  case "$1" in
    landing) printf '%s' "$LANDING_DOMAIN" ;;
    app) printf '%s' "$APP_DOMAIN" ;;
    api) printf '%s' "$API_DOMAIN" ;;
    api-dev) printf '%s' "$API_DEV_DOMAIN" ;;
    *)
      printf 'ERROR: unknown site "%s" in NGINX_SITES (landing, app, api, api-dev)\n' "$1" >&2
      return 1
      ;;
  esac
}

# Prints every host name a site answers, which its certificate covers too.
site_server_names() {
  case "$1" in
    landing) printf '%s %s' "$LANDING_DOMAIN" "$LANDING_WWW_DOMAIN" ;;
    *) site_certificate "$1" ;;
  esac
}

# True when the site's certificate has been issued.
site_has_certificate() {
  local name
  name="$(site_certificate "$1")" || return 1
  [[ -f /etc/letsencrypt/live/$name/fullchain.pem && -f /etc/letsencrypt/live/$name/privkey.pem ]]
}
