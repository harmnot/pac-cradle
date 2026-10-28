#!/usr/bin/env bash
# Reports a deploy or rollback to Telegram, with link buttons.
#
#   success: a full summary from the main bot to TELEGRAM_CHAT_IDS
#            (silent for development, so only production pings phones);
#   failure: a short alert from the alert bot to TELEGRAM_ALERT_CHAT_IDS with
#            only what matters: what failed, at which job and step, and why.
#
# Inputs (environment):
#   TG_STATUS             success | failure
#   TG_ACTION             deploy (default) | rollback
#   TG_APP_NAME           e.g. "paccradle API"
#   TG_ENVIRONMENT        production | development
#   TG_VERSION            the release version
#   TG_SITE_URL           public URL of what was deployed (optional)
#   TG_COMMIT_TITLE       first line of the commit message (optional)
#   TG_PREVIOUS_VERSION   version that ran before (optional)
#   TG_PREVIOUS_REVISION  commit that ran before (optional; adds "Changes")
#   TG_REVISION           commit now running (optional, default GITHUB_SHA;
#                         a rollback passes the older commit it went back to)
#   TG_ERROR_SUMMARY      one-line failure reason (optional)
#   TG_NOTE               one extra line, e.g. a warning (optional)
#   TELEGRAM_BOT_TOKEN, TELEGRAM_CHAT_IDS              success channel(s), comma-separated
#   TELEGRAM_ALERT_BOT_TOKEN, TELEGRAM_ALERT_CHAT_IDS  failure channel(s); default to the above
#   GITHUB_TOKEN          with actions: read, to name the failed step and the run time
#   TELEGRAM_API_URL      optional, default https://api.telegram.org
#
# Telegram problems never fail the workflow; they only log a warning.
# Same script in paccradle-backend, paccradle-frontend-app and paccradle-landing.
set -euo pipefail

status="${TG_STATUS:?TG_STATUS is required (success or failure)}"
action="${TG_ACTION:-deploy}"
app_name="${TG_APP_NAME:?TG_APP_NAME is required}"
environment="${TG_ENVIRONMENT:-unknown}"
version="${TG_VERSION:-unknown}"
site_url="${TG_SITE_URL:-}"
commit_title="${TG_COMMIT_TITLE:-}"
previous_version="${TG_PREVIOUS_VERSION:-}"
previous_revision="${TG_PREVIOUS_REVISION:-}"
revision="${TG_REVISION:-$GITHUB_SHA}"
error_summary="${TG_ERROR_SUMMARY:-}"
note="${TG_NOTE:-}"

if [[ $status == success ]]; then
  bot_token="${TELEGRAM_BOT_TOKEN:-}"
  chat_ids="${TELEGRAM_CHAT_IDS:-}"
else
  bot_token="${TELEGRAM_ALERT_BOT_TOKEN:-${TELEGRAM_BOT_TOKEN:-}}"
  chat_ids="${TELEGRAM_ALERT_CHAT_IDS:-${TELEGRAM_CHAT_IDS:-}}"
fi
if [[ -z $bot_token || -z $chat_ids ]]; then
  echo "::warning::Telegram secrets are not set; no $status message sent."
  exit 0
fi

repo_url="$GITHUB_SERVER_URL/$GITHUB_REPOSITORY"
run_url="$repo_url/actions/runs/$GITHUB_RUN_ID"
commit_url="$repo_url/commit/$revision"
short_sha="${revision:0:7}"

# Telegram HTML needs &, < and > escaped in text.
escape() {
  printf '%s' "${1-}" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
}

github_api() {
  [[ -n ${GITHUB_TOKEN:-} ]] || return 1
  curl -fsS --max-time 10 \
    -H "Authorization: Bearer $GITHUB_TOKEN" \
    -H "Accept: application/vnd.github+json" \
    "${GITHUB_API_URL:-https://api.github.com}/repos/$GITHUB_REPOSITORY/$1"
}

# "job → step" for every failed job of this run attempt (at most 5).
failed_steps() {
  github_api "actions/runs/$GITHUB_RUN_ID/attempts/${GITHUB_RUN_ATTEMPT:-1}/jobs?per_page=100" 2>/dev/null \
    | jq -r '.jobs[]
        | select(.conclusion == "failure")
        | .name + ([.steps[]? | select(.conclusion == "failure") | " → " + .name][0] // "")' \
    | head -n 5 || true
}

run_duration() {
  local started seconds
  started="$(github_api "actions/runs/$GITHUB_RUN_ID" 2>/dev/null | jq -r '.run_started_at // empty')" || return 0
  [[ -n $started ]] || return 0
  seconds=$(( $(date -u +%s) - $(date -u -d "$started" +%s) ))
  printf '%dm %02ds' $(( seconds / 60 )) $(( seconds % 60 ))
}

# Deploy: what this release adds. Rollback: what going back undoes.
changes_url=""
changes_label="🧾 Changes"
if [[ $previous_revision =~ ^[0-9a-f]{7,40}$ && $previous_revision != "$revision" ]]; then
  if [[ $action == rollback ]]; then
    changes_url="$repo_url/compare/$revision...$previous_revision"
    changes_label="↩️ Undone"
  else
    changes_url="$repo_url/compare/$previous_revision...$revision"
  fi
fi

commit_line="<b>Commit:</b> <a href=\"$commit_url\">$short_sha</a>"
[[ -n $commit_title ]] && commit_line+=" $(escape "${commit_title:0:120}")"

lines=()
if [[ $status == success ]]; then
  [[ $action == rollback ]] && verb="rolled back on" || verb="deployed to"
  version_line="<b>Version:</b> <code>$(escape "$version")</code>"
  if [[ -n $previous_version && $previous_version != "$version" ]]; then
    version_line+=" (was <code>$(escape "$previous_version")</code>)"
  fi
  lines+=("✅ <b>$(escape "$app_name")</b> $verb <b>$(escape "$environment")</b>" "")
  lines+=("$version_line" "$commit_line")
  lines+=("<b>Branch:</b> $(escape "$GITHUB_REF_NAME") · <b>By:</b> $(escape "$GITHUB_ACTOR")")
  duration="$(run_duration)"
  [[ -n $duration ]] && lines+=("<b>Duration:</b> $duration")
  [[ -n $site_url ]] && lines+=("<b>URL:</b> $(escape "$site_url")")
  [[ -n $note ]] && lines+=("⚠️ $(escape "$note")")
else
  lines+=("🚨 <b>$(escape "$app_name")</b> $(escape "$action") failed · <b>$(escape "$environment")</b>" "")
  lines+=("<b>Version:</b> <code>$(escape "$version")</code>")
  failed="$(failed_steps)"
  if [[ -n $failed ]]; then
    while IFS= read -r step; do
      lines+=("<b>Failed at:</b> $(escape "$step")")
    done <<<"$failed"
  fi
  [[ -n $error_summary ]] && lines+=("<b>Reason:</b> <code>$(escape "${error_summary:0:500}")</code>")
  lines+=("$commit_line · $(escape "$GITHUB_ACTOR")")
fi
text="$(printf '%s\n' "${lines[@]}")"

keyboard="$(jq -cn \
  --arg status "$status" --arg site "$site_url" --arg run "$run_url" \
  --arg commit "$commit_url" --arg changes "$changes_url" --arg changes_label "$changes_label" '
  [
    [ (if $status == "success" and $site != "" then {text: "🌐 Open", url: $site} else empty end),
      {text: (if $status == "success" then "📋 Run log" else "📋 Open failed run" end), url: $run} ],
    [ {text: "🔍 Commit", url: $commit},
      (if $changes != "" then {text: $changes_label, url: $changes} else empty end) ]
  ]')"

silent=false
[[ $status == success && $environment != production ]] && silent=true

IFS=',' read -r -a chats <<<"$chat_ids"
for chat in "${chats[@]}"; do
  chat="${chat//[[:space:]]/}"
  [[ -n $chat ]] || continue
  payload="$(jq -cn --arg chat "$chat" --arg text "$text" \
    --argjson keyboard "$keyboard" --argjson silent "$silent" '{
      chat_id: $chat, text: $text, parse_mode: "HTML",
      link_preview_options: {is_disabled: true},
      disable_notification: $silent,
      reply_markup: {inline_keyboard: $keyboard}
    }')"
  if ! response="$(curl -sS --max-time 15 -X POST -H 'Content-Type: application/json' \
      --data-binary "$payload" "${TELEGRAM_API_URL:-https://api.telegram.org}/bot$bot_token/sendMessage" 2>&1)"; then
    echo "::warning::Could not reach Telegram for chat $chat."
    continue
  fi
  if ! jq -e '.ok == true' >/dev/null 2>&1 <<<"$response"; then
    reason="$(jq -r '.description // "unknown error"' 2>/dev/null <<<"$response" || echo "unknown error")"
    echo "::warning::Telegram refused the message for chat $chat: $reason"
  fi
done
