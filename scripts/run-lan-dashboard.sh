#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
RUNTIME_DIR="$REPO_ROOT/.runtime"
DOWNLOAD_DIR="$RUNTIME_DIR/lan-dashboard"
BROKER_ENV_FILE="$RUNTIME_DIR/lan.env"
DASHBOARD_ENV_FILE="$RUNTIME_DIR/lan-dashboard.env"
COMPOSE_FILE="$REPO_ROOT/compose.lan.yaml"
DASHBOARD_COMPOSE_FILE="$REPO_ROOT/compose.lan-dashboard.yaml"
ARCHIVE_NAME=ReplayDesktop-v0.3.0-spike.6-arm64.zip
EXPECTED_SHA256=a5679ae4156752119af814b90141c173c3c382facff6a83b38405dc12242393f
CLIENT_ARCHIVE=${REPLAY_CLIENT_ARCHIVE:-$HOME/Downloads/$ARCHIVE_NAME}
DASHBOARD_BIND_IP=${REPLAY_DASHBOARD_BIND_IP:-0.0.0.0}
DASHBOARD_PORT=${REPLAY_DASHBOARD_PORT:-80}

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

[[ -f $BROKER_ENV_FILE ]] ||
    fail "broker environment not found; run scripts/run-lan-broker.sh first"
[[ -f $CLIENT_ARCHIVE ]] || fail "client archive not found: $CLIENT_ARCHIVE"
[[ $DASHBOARD_BIND_IP == 0.0.0.0 || $DASHBOARD_BIND_IP =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
    fail "dashboard bind address must be 0.0.0.0 or an IPv4 address"
[[ $DASHBOARD_PORT =~ ^[0-9]+$ ]] || fail "dashboard port must be an integer"
DASHBOARD_PORT=$((10#$DASHBOARD_PORT))
(( DASHBOARD_PORT >= 1 && DASHBOARD_PORT <= 65535 )) ||
    fail "dashboard port must be between 1 and 65535"

ACTUAL_SHA256=$(sha256sum "$CLIENT_ARCHIVE" | awk '{print $1}')
[[ $ACTUAL_SHA256 == "$EXPECTED_SHA256" ]] ||
    fail "client archive checksum mismatch: $ACTUAL_SHA256"

mkdir -p "$DOWNLOAD_DIR"
chmod 0755 "$DOWNLOAD_DIR"
install -m 0644 "$CLIENT_ARCHIVE" "$DOWNLOAD_DIR/$ARCHIVE_NAME"

umask 077
printf '%s\n' \
    "REPLAY_DASHBOARD_BIND_IP=$DASHBOARD_BIND_IP" \
    "REPLAY_DASHBOARD_PORT=$DASHBOARD_PORT" \
    >"$DASHBOARD_ENV_FILE"

docker compose \
    --env-file "$BROKER_ENV_FILE" \
    --env-file "$DASHBOARD_ENV_FILE" \
    -f "$COMPOSE_FILE" \
    -f "$DASHBOARD_COMPOSE_FILE" \
    --profile dashboard \
    up --detach dashboard

if [[ $DASHBOARD_BIND_IP == 0.0.0.0 ]]; then
    PROBE_HOST=127.0.0.1
else
    PROBE_HOST=$DASHBOARD_BIND_IP
fi

deadline=$((SECONDS + 30))
until curl --fail --silent --output /dev/null "http://$PROBE_HOST:$DASHBOARD_PORT/healthz"; do
    if (( SECONDS >= deadline )); then
        docker compose \
            --env-file "$BROKER_ENV_FILE" \
            --env-file "$DASHBOARD_ENV_FILE" \
            -f "$COMPOSE_FILE" \
            -f "$DASHBOARD_COMPOSE_FILE" \
            --profile dashboard \
            logs --tail=40 dashboard >&2
        fail "dashboard did not become healthy within 30 seconds"
    fi
    sleep 1
done

printf 'ReplayDesktop client dashboard: http://%s:%s\n' "$PROBE_HOST" "$DASHBOARD_PORT"
printf 'Direct client download:          http://%s:%s/downloads/%s\n' \
    "$PROBE_HOST" "$DASHBOARD_PORT" "$ARCHIVE_NAME"
printf 'Client archive SHA-256:          %s\n' "$ACTUAL_SHA256"
