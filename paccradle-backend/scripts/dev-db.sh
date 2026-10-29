#!/usr/bin/env bash
# Local PostgreSQL for `cargo run` and the database tests, in Docker (same image as CI).
#
#   scripts/dev-db.sh          start it (or check it's running) and print DATABASE_URL
#   scripts/dev-db.sh stop     stop it; the data stays in the "paccradle-pgdata" volume
#   scripts/dev-db.sh reset    delete it and its data
#
# It listens on 127.0.0.1:54329 only, so it never clashes with another local Postgres.
# The API's development default DATABASE_URL points here. Tests:
#   DATABASE_URL=$(scripts/dev-db.sh) cargo test
set -euo pipefail

name=paccradle-postgres
volume=paccradle-pgdata
url=postgres://paccradle:paccradle@127.0.0.1:54329/paccradle

case "${1:-start}" in
  start)
    if [[ -z "$(docker ps --quiet --filter "name=^${name}$")" ]]; then
      if [[ -n "$(docker ps --all --quiet --filter "name=^${name}$")" ]]; then
        docker start "$name" > /dev/null
      else
        docker run --detach --name "$name" \
          --publish 127.0.0.1:54329:5432 \
          --env POSTGRES_USER=paccradle \
          --env POSTGRES_PASSWORD=paccradle \
          --env POSTGRES_DB=paccradle \
          --volume "$volume:/var/lib/postgresql" \
          postgres:18-alpine > /dev/null
      fi
    fi
    for _ in $(seq 1 30); do
      if docker exec "$name" pg_isready --quiet --username paccradle --dbname paccradle; then
        echo "$url"
        exit 0
      fi
      sleep 1
    done
    echo "dev-db: $name did not become ready" >&2
    exit 1
    ;;
  stop)
    docker stop "$name" > /dev/null
    ;;
  reset)
    docker rm --force "$name" > /dev/null 2>&1 || true
    docker volume rm "$volume" > /dev/null 2>&1 || true
    ;;
  *)
    echo "usage: $0 [start|stop|reset]" >&2
    exit 2
    ;;
esac
