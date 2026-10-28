#!/usr/bin/env bash
# Prepares the SSH client for the VPS: the deploy key, the pinned host key and
# a `vps` host alias, so later steps run `ssh vps <request>`.
#
# Reads VPS_HOST, VPS_SSH_PORT (default 22), VPS_SSH_USER, VPS_SSH_PRIVATE_KEY
# and VPS_SSH_KNOWN_HOSTS (`ssh-keyscan` output, checked by hand once). The
# host key is pinned: a changed key fails the connection instead of being
# trusted. On the VPS the key can only run paccradle-remote (forced command).
# Same script in paccradle-backend, paccradle-frontend-app and paccradle-landing.
set -euo pipefail

: "${VPS_HOST:?the VPS_HOST secret is not set}"
: "${VPS_SSH_USER:?the VPS_SSH_USER secret is not set}"
: "${VPS_SSH_PRIVATE_KEY:?the VPS_SSH_PRIVATE_KEY secret is not set}"
: "${VPS_SSH_KNOWN_HOSTS:?the VPS_SSH_KNOWN_HOSTS secret is not set}"

ssh_dir="$HOME/.ssh"
install -d -m 700 "$ssh_dir"
printf '%s\n' "$VPS_SSH_PRIVATE_KEY" > "$ssh_dir/paccradle_deploy"
printf '%s\n' "$VPS_SSH_KNOWN_HOSTS" > "$ssh_dir/paccradle_known_hosts"
chmod 600 "$ssh_dir/paccradle_deploy" "$ssh_dir/paccradle_known_hosts"

cat > "$ssh_dir/config" <<EOF
Host vps
  HostName $VPS_HOST
  Port ${VPS_SSH_PORT:-22}
  User $VPS_SSH_USER
  IdentityFile $ssh_dir/paccradle_deploy
  IdentitiesOnly yes
  StrictHostKeyChecking yes
  UserKnownHostsFile $ssh_dir/paccradle_known_hosts
  ConnectTimeout 20
  ServerAliveInterval 20
  ServerAliveCountMax 6
EOF
chmod 600 "$ssh_dir/config"
