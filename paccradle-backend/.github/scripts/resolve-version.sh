#!/usr/bin/env bash
# Resolves the release version of a deploy and writes it to $GITHUB_OUTPUT.
#
#   resolve-version.sh <production|development> <manifest-version>
#
# production:  the manifest version as is (1.4.0); git tag v1.4.0. A version
#              that is already tagged on another commit is refused: bump it
#              in the manifest (Cargo.toml / package.json) first.
# development: a semver pre-release, 1.4.0-dev.<run number>.
#
# MANIFEST_FILE (optional) names the manifest in error messages.
# Same script in paccradle-backend, paccradle-frontend-app and paccradle-landing.
set -euo pipefail

environment="${1:?usage: resolve-version.sh <production|development> <manifest-version>}"
base_version="${2:?usage: resolve-version.sh <production|development> <manifest-version>}"
manifest="${MANIFEST_FILE:-the manifest}"
sha="${GITHUB_SHA:?GITHUB_SHA is not set}"

if [[ ! $base_version =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
  echo "::error::Version '$base_version' in $manifest must be plain MAJOR.MINOR.PATCH."
  exit 1
fi

# The commit a release tag points to (the peeled commit for an annotated tag).
tagged_commit() {
  local lines peeled
  lines="$(git ls-remote --tags origin "refs/tags/$1" "refs/tags/$1^{}")"
  [[ -z $lines ]] && return 0
  peeled="$(awk -v ref="refs/tags/$1^{}" '$2 == ref { print $1 }' <<<"$lines")"
  if [[ -n $peeled ]]; then
    echo "$peeled"
  else
    awk '{ print $1; exit }' <<<"$lines"
  fi
}

case "$environment" in
  production)
    version="$base_version"
    git_tag="v$base_version"
    released_at="$(tagged_commit "$git_tag")"
    if [[ -n $released_at && $released_at != "$sha" ]]; then
      echo "::error::Version $base_version is already released (tag $git_tag on ${released_at:0:7}). Bump the version in $manifest to deploy this commit to production."
      exit 1
    fi
    ;;
  development)
    version="$base_version-dev.${GITHUB_RUN_NUMBER:?GITHUB_RUN_NUMBER is not set}"
    git_tag=""
    ;;
  *)
    echo "::error::Unknown environment '$environment' (production or development)."
    exit 1
    ;;
esac

{
  echo "version=$version"
  echo "git_tag=$git_tag"
  echo "short_sha=${sha:0:7}"
} >> "${GITHUB_OUTPUT:-/dev/stdout}"
echo "Release version: $version ($environment, ${sha:0:7})"
