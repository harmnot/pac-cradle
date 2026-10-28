#!/bin/sh
# certbot runs this after it renews a certificate, so Nginx serves the new one.
# Installed by deploy/vps/install.sh into /etc/letsencrypt/renewal-hooks/deploy/.
set -e
nginx -t -q
systemctl reload nginx
