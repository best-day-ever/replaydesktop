#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
ENV_FILE="$REPO_ROOT/.runtime/lan.env"

if [[ ! -f $ENV_FILE ]]; then
    echo "Run scripts/run-lan-broker.sh first" >&2
    exit 1
fi

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a
BROKER_URL="http://127.0.0.1:8090"

status=$(curl --silent --output /dev/null --write-out '%{http_code}' "$BROKER_URL/healthz")
[[ $status == 204 ]] || { echo "healthz returned HTTP $status" >&2; exit 1; }

login=$(curl --fail --silent --show-error \
    --header 'content-type: application/json' \
    --data '{"username":"finn","password":"1337"}' \
    "$BROKER_URL/v1/auth/login")
access=$(jq -er '.access_token' <<<"$login")
workstations=$(curl --fail --silent --show-error \
    --header "authorization: Bearer $access" \
    "$BROKER_URL/v1/workstations")
workstation_id=$(jq -er '.[] | select(.online == true) | .id' <<<"$workstations" | head -n1)
[[ -n $workstation_id ]] || { echo "no online workstation registered" >&2; exit 1; }

session=$(curl --fail --silent --show-error \
    --request POST \
    --header "authorization: Bearer $access" \
    "$BROKER_URL/v1/workstations/$workstation_id/lan-sessions")
jq -e \
    --arg endpoint "$REPLAY_HOST_LAN_IPV4:$REPLAY_HOST_KYMUX_PORT" \
    '.status == "ready" and .direct_endpoint == $endpoint and (.kyber_token | split(".") | length == 3) and (.workstation_certificate_sha256 | length == 64)' \
    <<<"$session" >/dev/null

kyber_token=$(jq -er '.kyber_token' <<<"$session")
cookie_jar=$(mktemp /tmp/replaydesktop-kyber-cookie.XXXXXX)
trap 'rm -f -- "$cookie_jar"' EXIT
kyber_login=$(curl --fail --insecure --silent --show-error \
    --request POST \
    --cookie-jar "$cookie_jar" \
    --header "authorization: Bearer $kyber_token" \
    "https://127.0.0.1:$REPLAY_HOST_KYMUX_PORT/session/login")
jq -e '(.uid | type == "string") and (.websocket | type == "string")' \
    <<<"$kyber_login" >/dev/null
curl --fail --insecure --silent --show-error \
    --request POST \
    --cookie "$cookie_jar" \
    "https://127.0.0.1:$REPLAY_HOST_KYMUX_PORT/session/logout" \
    --output /dev/null

session_id=$(jq -er '.session_id' <<<"$session")
curl --fail --silent --show-error \
    --request DELETE \
    --header "authorization: Bearer $access" \
    "$BROKER_URL/v1/sessions/$session_id" \
    --output /dev/null

grep -q 'REPLAY_ADMISSION_BACKEND: memory' "$REPO_ROOT/compose.lan.yaml"
if rg -q 'REPLAY_UNIFI_' "$REPO_ROOT/compose.lan.yaml"; then
    echo "LAN compose unexpectedly contains UniFi configuration" >&2
    exit 1
fi
rg -q 'v1/workstations/\{workstation_id\}/lan-sessions' "$REPO_ROOT/server/src/api.rs"
rg -q -- '--auth-token' "$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"
rg -q -- '--tls-fingerprint' "$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"
rg -q 'Settings & Statistics' "$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"
rg -q 'NSStatusBar.system.statusItem' "$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"
if rg -q 'portField\.integerValue\s*=' "$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"; then
    echo "launcher writes the port through locale-sensitive integerValue" >&2
    exit 1
fi
rg -F -q 'portField.stringValue = String(endpoint.port)' \
    "$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"
rg -q 'NSAllowsLocalNetworking' "$REPO_ROOT/scripts/package-macos-gui.sh"

echo "PASS: broker health, finn login, online host discovery, direct LAN session, and Kyber JWT login"
