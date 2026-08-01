#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
RUNTIME_DIR="$REPO_ROOT/.runtime"
ENV_FILE="$RUNTIME_DIR/lan.env"
COMPOSE_FILE="$REPO_ROOT/compose.lan.yaml"
HOST_CERTIFICATE_PATH=${REPLAY_HOST_CERTIFICATE_PATH:-/home/finn/.local/share/replaydesktop/identity/server-cert.pem}
HOST_KYMUX_PORT=${REPLAY_HOST_KYMUX_PORT:-8080}
KYBER_CONFIG_PATH=${REPLAY_KYBER_CONFIG_PATH:-${XDG_DATA_HOME:-$HOME/.local/share}/replaydesktop/kyber-spike.toml}

HOST_LAN_IPV4=$(ip -4 route get 1.1.1.1 | awk '{for (i=1; i<=NF; i++) if ($i == "src") {print $(i+1); exit}}')
HOST_NAME=${REPLAY_HOST_NAME:-$(hostname)}
HOST_HOSTNAME=${REPLAY_HOST_HOSTNAME:-$(hostname)}

if [[ ! $HOST_LAN_IPV4 =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Could not determine the laptop LAN IPv4 address" >&2
    exit 1
fi
if [[ ! -f $HOST_CERTIFICATE_PATH ]]; then
    echo "Kyber certificate not found: $HOST_CERTIFICATE_PATH" >&2
    exit 1
fi

mkdir -p "$RUNTIME_DIR/lan-broker" "$RUNTIME_DIR/lan-secrets" "$RUNTIME_DIR/lan-registrar"
chmod 0700 "$RUNTIME_DIR/lan-broker" "$RUNTIME_DIR/lan-secrets" "$RUNTIME_DIR/lan-registrar"
umask 077
printf '%s\n' \
    "REPLAY_HOST_LAN_IPV4=$HOST_LAN_IPV4" \
    "REPLAY_HOST_NAME=$HOST_NAME" \
    "REPLAY_HOST_HOSTNAME=$HOST_HOSTNAME" \
    "REPLAY_HOST_KYMUX_PORT=$HOST_KYMUX_PORT" \
    "REPLAY_HOST_CERTIFICATE_PATH=$HOST_CERTIFICATE_PATH" \
    >"$ENV_FILE"

docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" up --detach --build

heartbeat_deadline=$((SECONDS + 30))
until docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" logs --no-color registrar 2>&1 |
    grep -q 'host heartbeat registered'; do
    if (( SECONDS >= heartbeat_deadline )); then
        echo "Registrar did not publish a host heartbeat within 30 seconds" >&2
        docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" logs --tail=40 registrar >&2
        exit 1
    fi
    sleep 1
done

echo "ReplayDesktop LAN broker: http://$HOST_LAN_IPV4:8090"
echo "Kyber host endpoint:       $HOST_LAN_IPV4:$HOST_KYMUX_PORT"
echo "Broker JWT public key:     $RUNTIME_DIR/lan-broker/kyber-jwt-public.pem"
echo "Development login:         finn / 1337 (LAN profile only)"

if [[ -f $KYBER_CONFIG_PATH ]] &&
    grep -Fq "key = { file = \"$RUNTIME_DIR/lan-broker/kyber-jwt-public.pem\" }" "$KYBER_CONFIG_PATH"; then
    echo "Kyber JWT trust:           configured in $KYBER_CONFIG_PATH"
else
    cat >&2 <<EOF

Kyber is not yet configured to trust broker-issued JWTs. Add this to:
  $KYBER_CONFIG_PATH

[kycontroller.auth.jwt]
enabled = true
algorithm = "RS256"
key = { file = "$RUNTIME_DIR/lan-broker/kyber-jwt-public.pem" }

Then restart kycontroller. The broker and registrar are running, but Connect will
fail closed until the host reloads this public key.
EOF
fi
