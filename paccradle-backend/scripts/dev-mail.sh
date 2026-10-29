#!/usr/bin/env bash
# Local email catcher for sign-in links (M1-D3): Mailpit in Docker. The API's development
# default sends to it (SMTP 127.0.0.1:1025, no TLS); read what arrives at http://localhost:8025.
#
#   scripts/dev-mail.sh          start it (or check it's running) and print the inbox URL
#   scripts/dev-mail.sh stop     stop it
#   scripts/dev-mail.sh reset    delete it and the mail it caught
#
# Both ports listen on 127.0.0.1 only.
set -euo pipefail

name=paccradle-mailpit
inbox=http://localhost:8025

case "${1:-start}" in
  start)
    if [[ -z "$(docker ps --quiet --filter "name=^${name}$")" ]]; then
      if [[ -n "$(docker ps --all --quiet --filter "name=^${name}$")" ]]; then
        docker start "$name" > /dev/null
      else
        docker run --detach --quiet --name "$name" \
          --publish 127.0.0.1:1025:1025 \
          --publish 127.0.0.1:8025:8025 \
          axllent/mailpit:v1.27 > /dev/null
      fi
    fi
    for _ in $(seq 1 30); do
      if curl --silent --fail --output /dev/null "$inbox/api/v1/info"; then
        echo "$inbox"
        exit 0
      fi
      sleep 1
    done
    echo "dev-mail: $name did not become ready" >&2
    exit 1
    ;;
  stop)
    docker stop "$name" > /dev/null
    ;;
  reset)
    docker rm --force "$name" > /dev/null 2>&1 || true
    ;;
  *)
    echo "usage: $0 [start|stop|reset]" >&2
    exit 2
    ;;
esac
