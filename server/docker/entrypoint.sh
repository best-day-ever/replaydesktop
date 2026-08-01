#!/bin/sh
set -eu

state_dir=${REPLAY_STATE_DIR:-/var/lib/replay-control}
secret_dir=${REPLAY_SECRET_DIR:-/run/replay-secrets}

mkdir -p "$state_dir" "$secret_dir"
chmod 0700 "$state_dir"
umask 077

mode=${1:-broker}
case "$mode" in
    broker)
        chmod 0700 "$secret_dir"
        host_token="$secret_dir/host-registration-token"
        if [ ! -s "$host_token" ]; then
            openssl rand -hex 32 >"$host_token"
        fi

        ticket_private="$state_dir/replay-ticket-private.pem"
        ticket_public="$state_dir/replay-ticket-public.pem"
        if [ ! -s "$ticket_private" ] || [ ! -s "$ticket_public" ]; then
            rm -f "$ticket_private" "$ticket_public"
            replay-control init-keys \
                --private-key "$ticket_private" \
                --public-key "$ticket_public"
        fi

        jwt_private="$state_dir/kyber-jwt-private.pem"
        jwt_public="$state_dir/kyber-jwt-public.pem"
        if [ ! -s "$jwt_private" ] || [ ! -s "$jwt_public" ]; then
            rm -f "$jwt_private" "$jwt_public"
            openssl genpkey -algorithm RSA \
                -pkeyopt rsa_keygen_bits:3072 \
                -out "$jwt_private"
            openssl pkey -in "$jwt_private" -pubout -out "$jwt_public"
        fi
        chmod 0600 "$host_token" "$ticket_private" "$jwt_private"
        chmod 0644 "$ticket_public" "$jwt_public"
        exec replay-control serve
        ;;
    registrar)
        host_token="$secret_dir/host-registration-token"
        tries=0
        while [ ! -s "$host_token" ]; do
            tries=$((tries + 1))
            if [ "$tries" -ge 60 ]; then
                echo "host registration token did not appear" >&2
                exit 1
            fi
            sleep 1
        done
        exec replay-control register-host
        ;;
    *)
        exec "$@"
        ;;
esac
