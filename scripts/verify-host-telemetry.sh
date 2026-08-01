#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
KYBER_DESKTOP="$REPO_ROOT/upstream/kyber-desktop"
KYSDK="$KYBER_DESKTOP/kysdk"
KYMEDIA="$KYSDK/kymedia"
TXPROTO="$KYMEDIA/subprojects/txproto"
ROOTFS="$KYBER_DESKTOP/rootfs-x86_64-linux-gnu"
PATCH_0008="$REPO_ROOT/patches/kyber/0008-txproto-host-telemetry.patch"
PATCH_0009="$REPO_ROOT/patches/kyber/0009-kymedia-host-telemetry-forwarding.patch"

EXPECTED_DESKTOP=6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f
EXPECTED_KYSDK=5836202aa4654cf64b5ca9fd204005fe4916be99
EXPECTED_KYMEDIA=e80eb6bb347ed0e378ae46a86f67ada2aa6079da
EXPECTED_TXPROTO=82694c38fb7d662ad364071382166edf8731db93

EXPECTED_0008_PATHS='meson.build
src/encode.c
src/fifo_template.c
src/include/libtxproto/fifo_frame.h
src/include/libtxproto/fifo_packet.h
src/include/libtxproto/metrics.h
src/iosys_nvfbc.c
src/metrics.c
src/packet_sink.c
test/host_metrics.c'

EXPECTED_0009_PATHS='kyavservice/src/metrics.rs
txproto-rs/src/lib.rs'

usage() {
    printf '%s\n' \
        "Usage: $0 --source|--tests|--replay|--live" \
        "" \
        "  --source  verify patch paths, schema keys, ABI compatibility, and log safety" \
        "  --tests   run focused Meson/Rust tests and compile the Linux kyavserver" \
        "  --replay  apply both patches to temporary checkouts at their exact pins" \
        "  --live    explicitly restart/start the bounded host probe, then restore state"
}

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "required command is unavailable: $1"
}

require_file() {
    [ -f "$1" ] || fail "required file is missing: $1"
}

assert_contains() {
    assert_file=$1
    assert_literal=$2
    rg -F -q -- "$assert_literal" "$assert_file" ||
        fail "$assert_file is missing required text: $assert_literal"
}

patch_paths() {
    sed -n 's|^diff --git a/\([^ ]*\) b/.*$|\1|p' "$1" | sort
}

assert_patch_paths() {
    path_patch=$1
    path_expected=$2
    path_actual=$(patch_paths "$path_patch")
    [ "$path_actual" = "$path_expected" ] ||
        fail "patch path allowlist mismatch in $path_patch"
}

assert_nested_heads() {
    [ "$(git -C "$KYBER_DESKTOP" rev-parse HEAD)" = "$EXPECTED_DESKTOP" ] ||
        fail "Kyber Desktop working HEAD drifted"
    [ "$(git -C "$KYSDK" rev-parse HEAD)" = "$EXPECTED_KYSDK" ] ||
        fail "Kyber SDK working HEAD drifted"
    [ "$(git -C "$KYMEDIA" rev-parse HEAD)" = "$EXPECTED_KYMEDIA" ] ||
        fail "Kymedia working HEAD drifted"
    [ "$(git -C "$TXPROTO" rev-parse HEAD)" = "$EXPECTED_TXPROTO" ] ||
        fail "txproto working HEAD drifted"

    desktop_link=$(git -C "$REPO_ROOT" ls-tree HEAD upstream/kyber-desktop |
        awk '{print $3}')
    kysdk_link=$(git -C "$KYBER_DESKTOP" ls-tree "$EXPECTED_DESKTOP" kysdk |
        awk '{print $3}')
    kymedia_link=$(git -C "$KYSDK" ls-tree "$EXPECTED_KYSDK" kymedia |
        awk '{print $3}')
    txproto_link=$(git -C "$KYMEDIA" ls-tree "$EXPECTED_KYMEDIA" subprojects/txproto |
        awk '{print $3}')

    [ "$desktop_link" = "$EXPECTED_DESKTOP" ] ||
        fail "parent Kyber Desktop gitlink drifted"
    [ "$kysdk_link" = "$EXPECTED_KYSDK" ] ||
        fail "Kyber SDK gitlink drifted"
    [ "$kymedia_link" = "$EXPECTED_KYMEDIA" ] ||
        fail "Kymedia gitlink drifted"
    [ "$txproto_link" = "$EXPECTED_TXPROTO" ] ||
        fail "txproto gitlink drifted"
}

verify_source() {
    require_command git
    require_command rg
    require_file "$PATCH_0008"
    require_file "$PATCH_0009"
    require_file "$TXPROTO/src/include/libtxproto/metrics.h"
    require_file "$KYMEDIA/txproto-rs/src/lib.rs"
    require_file "$KYMEDIA/kyavservice/src/metrics.rs"

    assert_nested_heads
    assert_patch_paths "$PATCH_0008" "$EXPECTED_0008_PATHS"
    assert_patch_paths "$PATCH_0009" "$EXPECTED_0009_PATHS"

    metrics_header="$TXPROTO/src/include/libtxproto/metrics.h"
    rust_bridge="$KYMEDIA/txproto-rs/src/lib.rs"
    rust_forwarder="$KYMEDIA/kyavservice/src/metrics.rs"

    rg -q 'typedef struct tx_metrics_entry' "$metrics_header" ||
        fail "tx_metrics_entry ABI declaration is missing"
    rg -q 'const char \*key;' "$metrics_header" ||
        fail "tx_metrics_entry.key ABI changed"
    rg -q 'int64_t value;' "$metrics_header" ||
        fail "tx_metrics_entry.value ABI changed"
    rg -q 'typedef int \(\*tx_metrics_cb\)' "$metrics_header" ||
        fail "tx_metrics_cb integer-return ABI changed"
    rg -q 'pub fn set_metrics_cb<Cb>' "$rust_bridge" ||
        fail "legacy Rust set_metrics_cb API is missing"
    rg -q 'Cb: FnMut\(&\[Metrics\]\)' "$rust_bridge" ||
        fail "legacy Rust FnMut callback contract changed"
    rg -q 'pub fn set_metrics_result_cb<Cb>' "$rust_bridge" ||
        fail "result-aware Rust callback API is missing"
    rg -q "Cb: Fn\\(&\\[Metrics\\]\\) -> bool \\+ Send \\+ Sync \\+ 'static" \
        "$rust_bridge" ||
        fail "result callback is not immutable, Send, Sync, and static"

    assert_contains "$TXPROTO/src/iosys_nvfbc.c" '.key = "acquired"'
    assert_contains "$TXPROTO/src/encode.c" '.key = "encoding"'
    assert_contains "$TXPROTO/src/encode.c" '"encoded"'
    assert_contains "$TXPROTO/src/packet_sink.c" '"sent"'
    assert_contains "$TXPROTO/src/fifo_template.c" \
        'ctx->num_data_queued >= ctx->max_queued'
    assert_contains "$TXPROTO/src/fifo_template.c" \
        'while (in && ctx->max_queued > 0'
    assert_contains "$TXPROTO/src/metrics.c" \
        'metrics->cb(batch, batch_count, metrics->userdata) != 0'

    for schema_key in \
        encoded_payload_bytes \
        sent_payload_bytes \
        queue_stage \
        queue_depth \
        queue_limit \
        queue_owner_unavailable \
        dropped \
        drop_stage \
        drop_reason \
        drop_total \
        telemetry_callback_lost_batches_total \
        telemetry_callback_lost_entries_total \
        telemetry_forwarder_lost_batches_total \
        telemetry_forwarder_lost_entries_total \
        telemetry_forwarder_first_lost_sequence \
        telemetry_forwarder_last_lost_sequence
    do
        if ! rg -F -q -- "$schema_key" "$PATCH_0008" "$PATCH_0009"; then
            fail "telemetry schema key is absent from the patch series: $schema_key"
        fi
    done

    assert_contains "$rust_forwarder" 'try_send(metrics)'
    assert_contains "$rust_forwarder" 'set_metrics_result_cb'
    assert_contains "$rust_forwarder" 'unflushed loss'

    added_lines() {
        awk '/^\+\+\+ / { next }
             /^\+/ { sub(/^\+/, ""); print }' "$@"
    }

    if added_lines "$PATCH_0008" "$PATCH_0009" |
        rg -n '(in_pkt|out_pkt|frame)->data|metric(s)?\.(key|value).*(warn!|error!|info!|debug!|trace!|println!|sp_log)|BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|client-token\.jwt|jwt-private|server-key|password[[:space:]]*=|token[[:space:]]*='
    then
        fail "patch additions contain payload/metric-content or secret logging"
    fi

    if added_lines "$PATCH_0008" "$PATCH_0009" |
        rg -n '(warn!|error!|info!|debug!|trace!|println!|sp_log).*(payload|packet_size|frame_pts|metric\.value|metric\.key)'
    then
        fail "producer-path logging exposes telemetry values or payload content"
    fi

    line_0008=$(rg -n -F '0008-txproto-host-telemetry.patch' "$REPO_ROOT/README.md" |
        head -1 | cut -d: -f1)
    line_0009=$(rg -n -F '0009-kymedia-host-telemetry-forwarding.patch' "$REPO_ROOT/README.md" |
        head -1 | cut -d: -f1)
    [ -n "$line_0008" ] && [ -n "$line_0009" ] &&
        [ "$line_0008" -lt "$line_0009" ] ||
        fail "README must apply txproto patch 0008 before Kymedia patch 0009"

    printf '%s\n' \
        'PASS: telemetry patches use exact path allowlists and nested pins' \
        'PASS: legacy callback/stage ABI remains present beside result-aware forwarding' \
        'PASS: bounded queue/drop/payload/loss keys are present' \
        'PASS: patch additions contain no payload-content or secret logging' \
        'SOURCE=PASS'
}

create_pkgconfig_view() {
    pc_view=$1
    mkdir -p "$pc_view"

    for pc_name in \
        libavcodec \
        libavformat \
        libswresample \
        libavfilter \
        libavutil \
        libswscale \
        libavdevice
    do
        pc_source="$ROOTFS/lib/pkgconfig/$pc_name.pc"
        require_file "$pc_source"
        ln -s "$pc_source" "$pc_view/$pc_name.pc"
    done

    require_file "$ROOTFS/lib/pkgconfig/liblua.pc"
    ln -s "$ROOTFS/lib/pkgconfig/liblua.pc" "$pc_view/liblua.pc"
    ln -s "$ROOTFS/lib/pkgconfig/liblua.pc" "$pc_view/lua.pc"

    system_pc_path=/usr/lib/pkgconfig:/usr/share/pkgconfig
    system_zlib_dir=$(
        PKG_CONFIG_PATH= PKG_CONFIG_LIBDIR="$system_pc_path" \
            pkg-config --variable=pcfiledir zlib
    ) || fail "system shared zlib pkg-config metadata is unavailable"
    require_file "$system_zlib_dir/zlib.pc"
    ln -s "$system_zlib_dir/zlib.pc" "$pc_view/zlib.pc"
}

compile_production_object() {
    production_source=$1
    production_build="$KYMEDIA/builddir-linux"
    production_ninja="$production_build/build.ninja"
    require_file "$production_ninja"

    production_target="subprojects/txproto/src/libtxproto.so.0.p/${production_source}.o"
    production_args=$(
        awk -v target="build ${production_target}:" '
            index($0, target) == 1 { found = 1; next }
            found && /^ ARGS = / {
                sub(/^ ARGS = /, "")
                print
                exit
            }
        ' "$production_ninja"
    )
    [ -n "$production_args" ] ||
        fail "production compile arguments are unavailable for $production_source"

    read -r -a production_argv <<<"$production_args"
    (
        cd "$production_build"
        clang "${production_argv[@]}" \
            -o "$TEST_SCRATCH/${production_source}.o" \
            -c "../subprojects/txproto/src/$production_source"
    )
}

verify_tests() {
    require_command cargo
    require_command clang
    require_command git
    require_command meson
    require_command pkg-config
    require_command rg
    require_command rustup

    TEST_SCRATCH=$(mktemp -d "${TMPDIR:-/tmp}/replaydesktop-host-telemetry-tests.XXXXXX")
    cleanup_tests() {
        case "$TEST_SCRATCH" in
            "${TMPDIR:-/tmp}"/replaydesktop-host-telemetry-tests.*)
                find "$TEST_SCRATCH" -depth -delete
                ;;
        esac
    }
    trap cleanup_tests EXIT

    pc_view="$TEST_SCRATCH/pkgconfig"
    create_pkgconfig_view "$pc_view"

    native_path="$ROOTFS/bin:$PATH"
    native_ld="$ROOTFS/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
    native_cpath="$KYMEDIA/subprojects/lua-5.4.6/src${CPATH:+:$CPATH}"

    (
        cd "$TXPROTO"
        export PATH="$native_path"
        export LD_LIBRARY_PATH="$native_ld"
        export CPATH="$native_cpath"
        export PKG_CONFIG_PATH=
        export PKG_CONFIG_LIBDIR="$pc_view"

        meson setup build-host-metrics \
            --buildtype=debugoptimized \
            --wrap-mode=nodownload \
            -Dwayland=disabled
        meson compile -C build-host-metrics host_metrics
        meson test -C build-host-metrics host_metrics --print-errorlogs
        meson compile -C build-host-metrics
        git diff --check -- \
            meson.build \
            src/fifo_template.c \
            src/include/libtxproto/fifo_frame.h \
            src/include/libtxproto/fifo_packet.h \
            src/include/libtxproto/metrics.h \
            src/metrics.c \
            src/iosys_nvfbc.c \
            src/encode.c \
            src/packet_sink.c
    )

    compile_production_object iosys_nvfbc.c
    compile_production_object encode.c
    compile_production_object packet_sink.c

    (
        cd "$KYMEDIA"
        export PATH="$native_path"
        export LD_LIBRARY_PATH="$TXPROTO/build-host-metrics/src:$native_ld"
        export CPATH="$native_cpath"
        export PKG_CONFIG_PATH="$TXPROTO/build-host-metrics/meson-uninstalled"
        export PKG_CONFIG_LIBDIR="$pc_view"

        cargo +1.89.0 test --locked -p txproto-rs -p kyavservice
        cargo +1.89.0 fmt --all -- --check

        clippy_log="$TEST_SCRATCH/clippy.log"
        set +e
        cargo +1.89.0 clippy --locked \
            -p txproto-rs -p kyavservice --all-targets -- -D warnings \
            >"$clippy_log" 2>&1
        clippy_status=$?
        set -e
        sed -n '1,240p' "$clippy_log"

        if [ "$clippy_status" -ne 0 ]; then
            rg -q 'kyavservice/src/lib.rs:361:17' "$clippy_log" &&
                rg -q 'clippy::let-underscore-future' "$clippy_log" &&
                ! rg -q 'txproto-rs/src/lib.rs|kyavservice/src/metrics.rs' "$clippy_log" ||
                fail "strict clippy failed outside the recorded pre-existing test lint"

            printf '%s\n' \
                'NOTICE: exact clippy is blocked only by the pre-existing' \
                'kyavservice/src/lib.rs:361 let_underscore_future test lint.'
            cargo +1.89.0 clippy --locked \
                -p txproto-rs -p kyavservice --all-targets -- \
                -D warnings -A clippy::let_underscore_future
        fi

        cargo +1.89.0 build --locked -p kyavserver
    )

    if rustup toolchain list | rg -q '^nightly'; then
        printf '%s\n' \
            'NOTICE: a nightly toolchain exists, but this verifier does not' \
            'silently change the pinned Rust 1.89.0 toolchain for sanitizer execution.'
        tsan_status='AVAILABLE_NOT_PINNED'
    else
        tsan_status='UNAVAILABLE_NO_NIGHTLY'
    fi

    printf '%s\n' \
        'PASS: host_metrics and full isolated txproto builds completed' \
        'PASS: production flags compiled NvFBC, encoder, and packet-sink ownership paths' \
        'PASS: focused Rust tests/fmt and changed-path warning gates completed' \
        'PASS: Linux kyavserver linked against the changed txproto build' \
        "THREAD_SANITIZER=$tsan_status" \
        'TESTS=PASS'
}

clone_at_pin() {
    clone_source=$1
    clone_destination=$2
    clone_commit=$3
    git -c advice.detachedHead=false clone --quiet --no-hardlinks \
        "$clone_source" "$clone_destination"
    git -c advice.detachedHead=false -C "$clone_destination" \
        checkout --quiet --detach "$clone_commit"
    [ "$(git -C "$clone_destination" rev-parse HEAD)" = "$clone_commit" ] ||
        fail "temporary checkout failed to reach $clone_commit"
}

verify_replay() {
    require_command git
    require_command rg
    assert_nested_heads
    assert_patch_paths "$PATCH_0008" "$EXPECTED_0008_PATHS"
    assert_patch_paths "$PATCH_0009" "$EXPECTED_0009_PATHS"

    replay_scratch=$(mktemp -d "${TMPDIR:-/tmp}/replaydesktop-host-telemetry-replay.XXXXXX")
    cleanup_replay() {
        case "$replay_scratch" in
            "${TMPDIR:-/tmp}"/replaydesktop-host-telemetry-replay.*)
                find "$replay_scratch" -depth -delete
                ;;
        esac
    }
    trap cleanup_replay EXIT

    replay_desktop="$replay_scratch/kyber-desktop"
    replay_kysdk="$replay_desktop/kysdk"
    replay_kymedia="$replay_kysdk/kymedia"
    replay_txproto="$replay_kymedia/subprojects/txproto"

    clone_at_pin "$KYBER_DESKTOP" "$replay_desktop" "$EXPECTED_DESKTOP"
    clone_at_pin "$KYSDK" "$replay_kysdk" "$EXPECTED_KYSDK"
    clone_at_pin "$KYMEDIA" "$replay_kymedia" "$EXPECTED_KYMEDIA"
    clone_at_pin "$TXPROTO" "$replay_txproto" "$EXPECTED_TXPROTO"

    git -C "$replay_txproto" apply --check --whitespace=error-all "$PATCH_0008"
    git -C "$replay_txproto" apply "$PATCH_0008"
    git -C "$replay_kymedia" apply --check --whitespace=error-all "$PATCH_0009"
    git -C "$replay_kymedia" apply "$PATCH_0009"

    replay_txproto_paths=$(
        git -C "$replay_txproto" status --short |
            sed 's/^...//' |
            sort
    )
    replay_kymedia_paths=$(
        git -C "$replay_kymedia" diff --ignore-submodules=all --name-only |
            sort
    )
    [ "$replay_txproto_paths" = "$EXPECTED_0008_PATHS" ] ||
        fail "0008 replay changed paths outside its allowlist"
    [ "$replay_kymedia_paths" = "$EXPECTED_0009_PATHS" ] ||
        fail "0009 replay changed paths outside its allowlist"

    [ "$(git -C "$replay_desktop" rev-parse HEAD)" = "$EXPECTED_DESKTOP" ] &&
        [ "$(git -C "$replay_kysdk" rev-parse HEAD)" = "$EXPECTED_KYSDK" ] &&
        [ "$(git -C "$replay_kymedia" rev-parse HEAD)" = "$EXPECTED_KYMEDIA" ] &&
        [ "$(git -C "$replay_txproto" rev-parse HEAD)" = "$EXPECTED_TXPROTO" ] ||
        fail "patch replay moved a nested HEAD"

    printf '%s\n' \
        'PASS: temporary recursive checkout matches every recorded nested pin' \
        'PASS: 0008 applies cleanly in txproto before 0009 applies in Kymedia' \
        'PASS: replayed changes match both exact path allowlists' \
        'PASS: patch replay created no nested commit or gitlink drift' \
        'REPLAY=PASS'
}

verify_live() {
    require_command journalctl
    require_command sha256sum
    require_command strings
    require_command systemctl

    live_unit=${REPLAYDESKTOP_TELEMETRY_UNIT:-replaydesktop-kyber-spike.service}
    live_config=${REPLAYDESKTOP_TELEMETRY_CONFIG:-"$HOME/.local/share/replaydesktop/kyber-spike.toml"}
    live_seconds=${REPLAYDESKTOP_TELEMETRY_LIVE_SECONDS:-30}
    case "$live_seconds" in
        ''|*[!0-9]*) fail "live duration must be an integer" ;;
    esac
    [ "$live_seconds" -ge 5 ] && [ "$live_seconds" -le 120 ] ||
        fail "live duration must be from 5 through 120 seconds"

    require_file "$live_config"
    systemctl --user show "$live_unit" -p LoadState --value |
        rg -q '^loaded$' || fail "host service is not loaded: $live_unit"

    live_before=$(systemctl --user is-active "$live_unit" 2>/dev/null || true)
    case "$live_before" in
        active|inactive) ;;
        *) fail "host service must begin active or inactive, observed: $live_before" ;;
    esac
    config_hash_before=$(sha256sum "$live_config" | awk '{print $1}')
    config_state_before=$(stat -c '%a:%u:%g:%s' "$live_config")

    restore_live_state() {
        set +e
        live_now=$(systemctl --user is-active "$live_unit" 2>/dev/null || true)
        if [ "$live_before" = active ] && [ "$live_now" != active ]; then
            timeout 30s systemctl --user start "$live_unit" >/dev/null 2>&1
        elif [ "$live_before" = inactive ] && [ "$live_now" = active ]; then
            timeout 30s systemctl --user stop "$live_unit" >/dev/null 2>&1
        fi
    }
    trap restore_live_state EXIT INT TERM

    live_cursor=$(
        journalctl --user -u "$live_unit" -n 0 --show-cursor --no-pager |
            sed -n 's/^-- cursor: //p'
    )

    if [ "$live_before" = active ]; then
        timeout 30s systemctl --user restart "$live_unit"
    else
        timeout 30s systemctl --user start "$live_unit"
    fi

    live_elapsed=0
    while [ "$live_elapsed" -lt "$live_seconds" ]; do
        systemctl --user is-active --quiet "$live_unit" ||
            fail "host service left active state during the bounded probe"
        sleep 1
        live_elapsed=$((live_elapsed + 1))
    done

    live_log=$(mktemp "${TMPDIR:-/tmp}/replaydesktop-host-telemetry-live.XXXXXX")
    if [ -n "$live_cursor" ]; then
        journalctl --user -u "$live_unit" --after-cursor "$live_cursor" \
            --no-pager -o cat >"$live_log"
    else
        journalctl --user -u "$live_unit" -n 200 --no-pager -o cat >"$live_log"
    fi
    tr -d '\000' <"$live_log" >"${live_log}.text"

    rg -i -q 'nvfbc.*(initialized|capture|grab|session)|using.*nvfbc' \
        "${live_log}.text" ||
        fail "bounded live journal contains no NvFBC session fact"
    rg -i -q '(h264|hevc|av1)_nvenc|nvenc.*(encoder|created|initialized)' \
        "${live_log}.text" ||
        fail "bounded live journal contains no NVENC session fact"

    require_file "$ROOTFS/lib/libtxproto.so.0"
    for live_key in \
        encoded_payload_bytes \
        sent_payload_bytes \
        queue_owner_unavailable \
        telemetry_callback_lost_batches_total
    do
        strings "$ROOTFS/lib/libtxproto.so.0" | rg -F -q -- "$live_key" ||
            fail "deployed libtxproto is missing telemetry key: $live_key"
    done

    rust_metrics_found=false
    for live_binary in "$ROOTFS/bin/kycontroller" "$ROOTFS/bin/kyavserver"; do
        if [ -f "$live_binary" ] &&
            strings "$live_binary" |
                rg -F -q 'telemetry_forwarder_lost_batches_total'
        then
            rust_metrics_found=true
            break
        fi
    done
    [ "$rust_metrics_found" = true ] ||
        fail "deployed controller/server lacks Rust forwarder-loss telemetry"

    restore_live_state
    trap - EXIT INT TERM

    live_after=$(systemctl --user is-active "$live_unit" 2>/dev/null || true)
    config_hash_after=$(sha256sum "$live_config" | awk '{print $1}')
    config_state_after=$(stat -c '%a:%u:%g:%s' "$live_config")
    [ "$live_after" = "$live_before" ] ||
        fail "host service active state was not restored"
    [ "$config_hash_after" = "$config_hash_before" ] &&
        [ "$config_state_after" = "$config_state_before" ] ||
        fail "host configuration content or metadata changed"

    find "$live_log" "${live_log}.text" -maxdepth 0 -type f -delete

    printf '%s\n' \
        'PASS: bounded live session observed NvFBC and NVENC facts' \
        'PASS: deployed native/Rust artifacts contain the telemetry schema' \
        "PASS: service state restored to $live_before" \
        "PASS: config hash preserved at $config_hash_before" \
        'LIVE=PASS'
}

[ "$#" -eq 1 ] || {
    usage >&2
    exit 2
}

case "$1" in
    --source) verify_source ;;
    --tests) verify_tests ;;
    --replay) verify_replay ;;
    --live) verify_live ;;
    -h|--help) usage ;;
    *)
        usage >&2
        exit 2
        ;;
esac
