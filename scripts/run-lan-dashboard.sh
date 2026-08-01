#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
RUNTIME_DIR="$REPO_ROOT/.runtime"
DOWNLOAD_DIR="$RUNTIME_DIR/lan-dashboard"
BROKER_ENV_FILE="$RUNTIME_DIR/lan.env"
DASHBOARD_ENV_FILE="$RUNTIME_DIR/lan-dashboard.env"
RUNTIME_COMPOSE_FILE="$RUNTIME_DIR/lan-dashboard.compose.yaml"
COMPOSE_FILE="$REPO_ROOT/compose.lan.yaml"
DASHBOARD_COMPOSE_FILE="$REPO_ROOT/compose.lan-dashboard.yaml"
ARCHIVE_NAME=ReplayDesktop-v0.3.0-spike.6-arm64.zip
EXPECTED_SHA256=a5679ae4156752119af814b90141c173c3c382facff6a83b38405dc12242393f
CLIENT_ARCHIVE=${REPLAY_CLIENT_ARCHIVE:-$HOME/Downloads/$ARCHIVE_NAME}
DASHBOARD_BIND_IP=${REPLAY_DASHBOARD_BIND_IP:-0.0.0.0}
DASHBOARD_BIND_IPS=${REPLAY_DASHBOARD_BIND_IPS:-}
DASHBOARD_PORT=${REPLAY_DASHBOARD_PORT:-80}

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

[[ -f $BROKER_ENV_FILE ]] ||
    fail "broker environment not found; run scripts/run-lan-broker.sh first"
[[ -f $CLIENT_ARCHIVE ]] || fail "client archive not found: $CLIENT_ARCHIVE"
[[ $DASHBOARD_PORT =~ ^[0-9]+$ ]] || fail "dashboard port must be an integer"
DASHBOARD_PORT=$((10#$DASHBOARD_PORT))
(( DASHBOARD_PORT >= 1 && DASHBOARD_PORT <= 65535 )) ||
    fail "dashboard port must be between 1 and 65535"

declare -a BIND_ADDRESSES=()
if [[ $DASHBOARD_BIND_IPS == auto ]]; then
    mapfile -t BIND_ADDRESSES < <(
        ip -4 -o addr show scope global |
            awk '$2 !~ /^(lo|docker|br-|virbr|veth)/ {
                     split($4, address, "/")
                     print address[1]
                 }' |
            LC_ALL=C sort -u
    )
elif [[ -n $DASHBOARD_BIND_IPS ]]; then
    IFS=',' read -r -a BIND_ADDRESSES <<<"$DASHBOARD_BIND_IPS"
else
    BIND_ADDRESSES=("$DASHBOARD_BIND_IP")
fi
(( ${#BIND_ADDRESSES[@]} > 0 )) || fail "no client-facing IPv4 addresses found"
for bind_address in "${BIND_ADDRESSES[@]}"; do
    [[ $bind_address == 0.0.0.0 || $bind_address =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
        fail "dashboard bind address must be 0.0.0.0 or an IPv4 address: $bind_address"
done

ACTUAL_SHA256=$(sha256sum "$CLIENT_ARCHIVE" | awk '{print $1}')
[[ $ACTUAL_SHA256 == "$EXPECTED_SHA256" ]] ||
    fail "client archive checksum mismatch: $ACTUAL_SHA256"

mkdir -p "$DOWNLOAD_DIR"
chmod 0755 "$DOWNLOAD_DIR"
install -m 0644 "$CLIENT_ARCHIVE" "$DOWNLOAD_DIR/$ARCHIVE_NAME"

umask 077
printf '%s\n' \
    "REPLAY_DASHBOARD_BIND_IP=${BIND_ADDRESSES[0]}" \
    "REPLAY_DASHBOARD_PORT=$DASHBOARD_PORT" \
    >"$DASHBOARD_ENV_FILE"

if (( ${#BIND_ADDRESSES[@]} > 1 )); then
    {
        printf '%s\n' 'services:' '  dashboard:' '    ports:'
        for bind_address in "${BIND_ADDRESSES[@]}"; do
            printf '      - "%s:%s:80"\n' "$bind_address" "$DASHBOARD_PORT"
        done
    } >"$RUNTIME_COMPOSE_FILE"
    DASHBOARD_COMPOSE_FILE=$RUNTIME_COMPOSE_FILE
fi

docker compose \
    --env-file "$BROKER_ENV_FILE" \
    --env-file "$DASHBOARD_ENV_FILE" \
    -f "$COMPOSE_FILE" \
    -f "$DASHBOARD_COMPOSE_FILE" \
    --profile dashboard \
    up --detach dashboard

if [[ ${BIND_ADDRESSES[0]} == 0.0.0.0 ]]; then
    PROBE_HOST=127.0.0.1
else
    PROBE_HOST=${BIND_ADDRESSES[0]}
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

for bind_address in "${BIND_ADDRESSES[@]}"; do
    if [[ $bind_address == 0.0.0.0 ]]; then
        display_address=$PROBE_HOST
    else
        display_address=$bind_address
    fi
    printf 'ReplayDesktop client dashboard: http://%s:%s\n' "$display_address" "$DASHBOARD_PORT"
done
printf 'Direct client download:          http://%s:%s/downloads/%s\n' \
    "$PROBE_HOST" "$DASHBOARD_PORT" "$ARCHIVE_NAME"
printf 'Client archive SHA-256:          %s\n' "$ACTUAL_SHA256"
