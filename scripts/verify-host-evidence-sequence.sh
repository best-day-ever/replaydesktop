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
PATCH_0012="$REPO_ROOT/patches/kyber/0012-txproto-capture-begin.patch"
PATCH_0013="$REPO_ROOT/patches/kyber/0013-kymedia-host-evidence-sequence.patch"

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

EXPECTED_0012_PATHS='src/iosys_nvfbc.c
test/host_metrics.c'

EXPECTED_0013_PATHS='kyavservice/src/metrics.rs'

usage() {
    printf '%s\n' \
        "Usage: $0 --source|--tests|--replay" \
        "" \
        "  --source  verify exact pins, patch scope, source ordering, schema, and log safety" \
        "  --tests   run registered native stress and targeted Rust test/fmt/clippy gates" \
        "  --replay  apply prerequisites and new patches at exact pins, then reverse to baseline"
}

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 ||
        fail "required command is unavailable: $1"
}

require_file() {
    [ -f "$1" ] || fail "required file is missing: $1"
}

assert_contains() {
    contains_file=$1
    contains_literal=$2
    rg -F -q -- "$contains_literal" "$contains_file" ||
        fail "$contains_file is missing required text: $contains_literal"
}

literal_line() {
    line_file=$1
    line_literal=$2
    rg -n -F -m 1 -- "$line_literal" "$line_file" |
        cut -d: -f1
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

    git -C "$KYBER_DESKTOP" diff --cached --quiet ||
        fail "Kyber Desktop nested index is staged"
    git -C "$KYSDK" diff --cached --quiet ||
        fail "Kyber SDK nested index is staged"
    git -C "$KYMEDIA" diff --cached --quiet ||
        fail "Kymedia nested index is staged"
    git -C "$TXPROTO" diff --cached --quiet ||
        fail "txproto nested index is staged"

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

assert_increasing_lines() {
    previous=0
    for current in "$@"; do
        [ -n "$current" ] || fail "source-order marker is absent"
        [ "$current" -gt "$previous" ] ||
            fail "source-order marker is out of order at line $current"
        previous=$current
    done
}

added_lines() {
    awk '/^\+\+\+ / { next }
         /^\+/ { sub(/^\+/, ""); print }' "$@"
}

verify_source() {
    require_command git
    require_command rg
    require_file "$PATCH_0008"
    require_file "$PATCH_0009"
    require_file "$PATCH_0012"
    require_file "$PATCH_0013"
    require_file "$TXPROTO/src/iosys_nvfbc.c"
    require_file "$TXPROTO/test/host_metrics.c"
    require_file "$KYMEDIA/kyavservice/src/metrics.rs"

    assert_nested_heads
    assert_patch_paths "$PATCH_0008" "$EXPECTED_0008_PATHS"
    assert_patch_paths "$PATCH_0009" "$EXPECTED_0009_PATHS"
    assert_patch_paths "$PATCH_0012" "$EXPECTED_0012_PATHS"
    assert_patch_paths "$PATCH_0013" "$EXPECTED_0013_PATHS"

    git -C "$TXPROTO" apply --reverse --check "$PATCH_0012" ||
        fail "working txproto source does not contain patch 0012"
    git -C "$KYMEDIA" apply --reverse --check "$PATCH_0013" ||
        fail "working Kymedia source does not contain patch 0013"

    nvfbc_source="$TXPROTO/src/iosys_nvfbc.c"
    capture_sample=$(literal_line "$nvfbc_source" \
        'int64_t capture_begin = av_gettime_relative();')
    capture_grab=$(literal_line "$nvfbc_source" \
        'fbcStatus = nvfbc.nvFBCToCudaGrabFrame(priv->fbc_handle, &grabParams);')
    capture_copy=$(literal_line "$nvfbc_source" \
        'CUresult cuResult = ctx->cuda->cuMemcpyDtoD(')
    capture_copy_gate=$(literal_line "$nvfbc_source" \
        'if (cuResult != CUDA_SUCCESS) {')
    capture_acquired=$(literal_line "$nvfbc_source" \
        'int64_t ts = av_gettime_relative();')
    capture_key=$(literal_line "$nvfbc_source" \
        '{ .key = "capture_begin", .value = capture_begin },')
    capture_publish=$(literal_line "$nvfbc_source" \
        'tx_metrics_push(ctx->metrics, metrics, SP_ARRAY_ELEMS(metrics));')

    assert_increasing_lines \
        "$capture_sample" \
        "$capture_grab" \
        "$capture_copy" \
        "$capture_copy_gate" \
        "$capture_acquired" \
        "$capture_key" \
        "$capture_publish"
    [ "$capture_grab" -eq $((capture_sample + 1)) ] ||
        fail "capture_begin must be sampled immediately before the real NvFBC grab"
    assert_contains "$nvfbc_source" 'err = AVERROR_EXTERNAL;'
    assert_contains "$TXPROTO/test/host_metrics.c" \
        'static void test_capture_begin_batch(void)'
    assert_contains "$TXPROTO/test/host_metrics.c" \
        'assert(count_metric(&capture, "capture_begin") == 1);'
    assert_contains "$TXPROTO/meson.build" "test('host_metrics', host_metrics)"

    rust_forwarder="$KYMEDIA/kyavservice/src/metrics.rs"
    source_copy=$(literal_line "$rust_forwarder" \
        'metrics.extend(source.iter().map(|metric| Metrics {')
    sequence_key=$(literal_line "$rust_forwarder" \
        'key: "telemetry_forwarder_sequence".into(),')
    channel_send=$(literal_line "$rust_forwarder" \
        'if self.tx.try_send(metrics).is_ok() {')
    accepted_update=$(literal_line "$rust_forwarder" \
        'self.state.record_accepted(sequence);')
    drain_line=$(literal_line "$rust_forwarder" \
        'while let Some(metrics) = rx.recv().await {')
    terminal_snapshot=$(literal_line "$rust_forwarder" \
        'let terminal = state.terminal_snapshot().ok_or_else(|| {')
    terminal_send=$(literal_line "$rust_forwarder" \
        'send_metrics(&mut send, &terminal_metrics).await?;')
    terminal_mark=$(literal_line "$rust_forwarder" \
        'state.mark_terminal_reported(terminal.published_loss.publication);')

    assert_increasing_lines \
        "$source_copy" \
        "$sequence_key" \
        "$channel_send" \
        "$accepted_update"
    assert_increasing_lines \
        "$drain_line" \
        "$terminal_snapshot" \
        "$terminal_send" \
        "$terminal_mark"

    assert_contains "$rust_forwarder" \
        '.fetch_max(sequence, Ordering::SeqCst);'
    assert_contains "$rust_forwarder" \
        'let state = Arc::clone(&forwarder.state);'
    assert_contains "$rust_forwarder" \
        'forward_task(rx, uri, state).await;'
    assert_contains "$rust_forwarder" \
        'Metrics forward task ended with producer terminal unconfirmed'

    for schema_key in \
        telemetry_forwarder_sequence \
        telemetry_forwarder_terminal \
        telemetry_forwarder_final_sequence \
        telemetry_forwarder_final_accepted_sequence \
        telemetry_forwarder_lost_batches_total \
        telemetry_forwarder_lost_entries_total \
        telemetry_forwarder_first_lost_sequence \
        telemetry_forwarder_last_lost_sequence
    do
        assert_contains "$rust_forwarder" "$schema_key"
    done

    terminal_body=$(
        awk '/fn terminal_metrics\(/ { inside = 1 }
             inside { print }
             inside && /fn try_forward\(/ { exit }' "$rust_forwarder"
    )
    if printf '%s\n' "$terminal_body" |
        rg -q '"telemetry_forwarder_sequence"|"pts"'
    then
        fail "terminal record contains a normal sequence or PTS identity"
    fi

    for test_name in \
        sequential_accepts_preserve_native_prefix_and_append_identity \
        full_channel_tracks_consecutive_loss_and_piggybacks_recovery \
        simultaneous_writers_get_unique_sequences_and_nonregressing_acceptance \
        first_accepted_batch_can_follow_an_initial_rejection \
        terminal_reports_no_accepted_batches_explicitly \
        accepted_batches_drain_before_terminal_and_final_loss_is_disclosed \
        direct_terminal_send_failure_stays_unconfirmed \
        recovery_and_terminal_repeat_one_cumulative_loss_snapshot \
        concurrent_loss_snapshots_are_coherent_and_totals_exact
    do
        assert_contains "$rust_forwarder" "fn $test_name"
    done

    if added_lines "$PATCH_0012" "$PATCH_0013" |
        rg -n '(in_pkt|out_pkt|frame)->data|metric(s)?\.(key|value).*(warn!|error!|info!|debug!|trace!|println!|sp_log)|BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|client-token\.jwt|jwt-private|server-key|password[[:space:]]*=|token[[:space:]]*='
    then
        fail "patch additions contain payload/metric-content or secret logging"
    fi

    readme_0008=$(literal_line "$REPO_ROOT/README.md" \
        '0008-txproto-host-telemetry.patch')
    readme_0012=$(literal_line "$REPO_ROOT/README.md" \
        '0012-txproto-capture-begin.patch')
    readme_0009=$(literal_line "$REPO_ROOT/README.md" \
        '0009-kymedia-host-telemetry-forwarding.patch')
    readme_0013=$(literal_line "$REPO_ROOT/README.md" \
        '0013-kymedia-host-evidence-sequence.patch')
    assert_increasing_lines \
        "$readme_0008" \
        "$readme_0012" \
        "$readme_0009" \
        "$readme_0013"
    assert_contains "$REPO_ROOT/README.md" \
        'host_capture_to_cuda_surface_ready'
    assert_contains "$REPO_ROOT/README.md" \
        'producer_terminal_unconfirmed'

    printf '%s\n' \
        'PASS: 0012/0013 use strict path allowlists at exact nested pins' \
        'PASS: capture_begin precedes NvFBC and publishes only after checked CUDA copy' \
        'PASS: accepted sequence, cumulative loss, drain, and direct terminal order are fixed' \
        'PASS: terminal records have no normal sequence/PTS and patch logs expose no payloads' \
        'SOURCE=PASS'
}

safe_remove_scratch() {
    scratch_root=$1
    scratch_prefix=$2
    case "$scratch_root" in
        "${TMPDIR:-/tmp}"/"$scratch_prefix".*)
            [ -d "$scratch_root" ] || return 0
            [ ! -L "$scratch_root" ] ||
                fail "refusing to clean symlink scratch root: $scratch_root"
            find "$scratch_root" -depth -delete
            ;;
        *)
            fail "refusing to clean unvalidated scratch root: $scratch_root"
            ;;
    esac
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

compile_production_nvfbc() {
    production_build="$KYMEDIA/builddir-linux"
    production_ninja="$production_build/build.ninja"
    require_file "$production_ninja"

    production_target='subprojects/txproto/src/libtxproto.so.0.p/iosys_nvfbc.c.o'
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
        fail "production compile arguments are unavailable for iosys_nvfbc.c"

    read -r -a production_argv <<<"$production_args"
    (
        cd "$production_build"
        clang "${production_argv[@]}" \
            -o "$TEST_SCRATCH/iosys_nvfbc.c.o" \
            -c ../subprojects/txproto/src/iosys_nvfbc.c
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
    require_file "$ROOTFS/lib/pkgconfig/libavcodec.pc"

    assert_nested_heads
    TEST_SCRATCH=$(mktemp -d \
        "${TMPDIR:-/tmp}/replaydesktop-host-evidence-tests.XXXXXX")
    chmod 700 "$TEST_SCRATCH"
    [ "$(stat -c '%a' "$TEST_SCRATCH")" = 700 ] ||
        fail "test scratch directory is not mode 0700"
    cleanup_tests() {
        safe_remove_scratch "$TEST_SCRATCH" \
            replaydesktop-host-evidence-tests
    }
    trap cleanup_tests EXIT

    pc_view="$TEST_SCRATCH/pkgconfig"
    native_build="$TEST_SCRATCH/txproto-build"
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

        meson setup "$native_build" \
            --buildtype=debugoptimized \
            --wrap-mode=nodownload \
            -Dwayland=disabled
        meson compile -C "$native_build" host_metrics
        meson test -C "$native_build" host_metrics --print-errorlogs
        for stress_run in 1 2 3 4 5; do
            LD_LIBRARY_PATH="$native_build/src:$native_ld" \
                "$native_build/host_metrics" >/dev/null
            printf 'host_metrics stress run %s: PASS\n' "$stress_run"
        done
        git diff --check -- src/iosys_nvfbc.c test/host_metrics.c
    )

    compile_production_nvfbc

    (
        cd "$KYMEDIA"
        export PATH="$native_path"
        export LD_LIBRARY_PATH="$native_build/src:$native_ld"
        export CPATH="$native_cpath"
        export PKG_CONFIG_PATH="$native_build/meson-uninstalled"
        export PKG_CONFIG_LIBDIR="$pc_view"

        cargo +1.89.0 test --locked -p kyavservice metrics::tests
        cargo +1.89.0 fmt -p kyavservice -- --check

        clippy_log="$TEST_SCRATCH/clippy.log"
        set +e
        cargo +1.89.0 clippy --locked -p kyavservice --tests -- -D warnings \
            >"$clippy_log" 2>&1
        clippy_status=$?
        set -e
        sed -n '1,240p' "$clippy_log"

        if [ "$clippy_status" -ne 0 ]; then
            rg -q 'kyavservice/src/lib.rs:361:17' "$clippy_log" &&
                rg -q 'clippy::let-underscore-future' "$clippy_log" &&
                ! rg -q 'kyavservice/src/metrics.rs' "$clippy_log" ||
                fail "strict clippy failed outside the recorded upstream-only lint"

            printf '%s\n' \
                'NOTICE: strict clippy is blocked only by the pre-existing' \
                'kyavservice/src/lib.rs:361 let_underscore_future test lint.'
            cargo +1.89.0 clippy --locked \
                -p kyavservice --tests -- \
                -D warnings -A clippy::let_underscore_future
        fi

        git diff --check -- kyavservice/src/metrics.rs
    )

    printf '%s\n' \
        'PASS: registered host_metrics test and five native stress runs completed' \
        'PASS: production flags compiled the changed NvFBC capture path' \
        'PASS: all targeted sequence/loss/terminal Rust tests completed' \
        'PASS: Rust fmt and changed-path warning gates completed' \
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

hash_paths() {
    hash_root=$1
    hash_list=$2
    while IFS= read -r hash_path; do
        require_file "$hash_root/$hash_path"
        sha256sum "$hash_root/$hash_path"
    done <<<"$hash_list" | sha256sum | awk '{print $1}'
}

verify_replay() {
    require_command git
    require_command rg
    require_command sha256sum
    assert_nested_heads
    assert_patch_paths "$PATCH_0008" "$EXPECTED_0008_PATHS"
    assert_patch_paths "$PATCH_0009" "$EXPECTED_0009_PATHS"
    assert_patch_paths "$PATCH_0012" "$EXPECTED_0012_PATHS"
    assert_patch_paths "$PATCH_0013" "$EXPECTED_0013_PATHS"

    replay_scratch=$(mktemp -d \
        "${TMPDIR:-/tmp}/replaydesktop-host-evidence-replay.XXXXXX")
    chmod 700 "$replay_scratch"
    [ "$(stat -c '%a' "$replay_scratch")" = 700 ] ||
        fail "replay scratch directory is not mode 0700"
    cleanup_replay() {
        safe_remove_scratch "$replay_scratch" \
            replaydesktop-host-evidence-replay
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
    txproto_prerequisite=$(hash_paths "$replay_txproto" "$EXPECTED_0008_PATHS")

    git -C "$replay_txproto" apply --check --whitespace=error-all "$PATCH_0012"
    git -C "$replay_txproto" apply "$PATCH_0012"
    cmp "$replay_txproto/src/iosys_nvfbc.c" "$TXPROTO/src/iosys_nvfbc.c"
    cmp "$replay_txproto/test/host_metrics.c" "$TXPROTO/test/host_metrics.c"
    git -C "$replay_txproto" apply --reverse --check "$PATCH_0012"
    git -C "$replay_txproto" apply --reverse "$PATCH_0012"
    [ "$(hash_paths "$replay_txproto" "$EXPECTED_0008_PATHS")" = \
        "$txproto_prerequisite" ] ||
        fail "0012 reverse apply did not restore the exact 0008 baseline"

    git -C "$replay_kymedia" apply --check --whitespace=error-all "$PATCH_0009"
    git -C "$replay_kymedia" apply "$PATCH_0009"
    kymedia_prerequisite=$(hash_paths "$replay_kymedia" "$EXPECTED_0009_PATHS")

    git -C "$replay_kymedia" apply --check --whitespace=error-all "$PATCH_0013"
    git -C "$replay_kymedia" apply "$PATCH_0013"
    cmp "$replay_kymedia/kyavservice/src/metrics.rs" \
        "$KYMEDIA/kyavservice/src/metrics.rs"
    git -C "$replay_kymedia" apply --reverse --check "$PATCH_0013"
    git -C "$replay_kymedia" apply --reverse "$PATCH_0013"
    [ "$(hash_paths "$replay_kymedia" "$EXPECTED_0009_PATHS")" = \
        "$kymedia_prerequisite" ] ||
        fail "0013 reverse apply did not restore the exact 0009 baseline"

    [ "$(git -C "$replay_desktop" rev-parse HEAD)" = "$EXPECTED_DESKTOP" ] &&
        [ "$(git -C "$replay_kysdk" rev-parse HEAD)" = "$EXPECTED_KYSDK" ] &&
        [ "$(git -C "$replay_kymedia" rev-parse HEAD)" = "$EXPECTED_KYMEDIA" ] &&
        [ "$(git -C "$replay_txproto" rev-parse HEAD)" = "$EXPECTED_TXPROTO" ] ||
        fail "patch replay moved a nested HEAD"

    printf '%s\n' \
        'PASS: exact txproto replay applied 0008 before 0012' \
        'PASS: exact Kymedia replay applied 0009 before 0013 with txproto at 0008' \
        'PASS: reverse checks restored each prerequisite baseline byte-for-byte' \
        'PASS: replay created no nested commit or gitlink drift' \
        'REPLAY=PASS'
}

[ "$#" -eq 1 ] || {
    usage >&2
    exit 2
}

case "$1" in
    --source) verify_source ;;
    --tests) verify_tests ;;
    --replay) verify_replay ;;
    -h|--help) usage ;;
    *)
        usage >&2
        exit 2
        ;;
esac
