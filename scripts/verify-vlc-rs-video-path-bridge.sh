#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
KYBER_DESKTOP="$REPO_ROOT/upstream/kyber-desktop"
KYSDK="$KYBER_DESKTOP/kysdk"
KYMEDIA="$KYSDK/kymedia"
TXPROTO="$KYMEDIA/subprojects/txproto"
VLC="$KYMEDIA/subprojects/vlc"
VLC_RS="$KYMEDIA/subprojects/vlc-rs"
LINUX_BUILD="$KYMEDIA/builddir-linux"
VLC_LIB_DIR="$LINUX_BUILD/subprojects/vlc/lib"
VLC_CORE_DIR="$LINUX_BUILD/subprojects/vlc/src"
VLC_LIBRARY="$VLC_LIB_DIR/libvlc.so"
VLC_UNINSTALLED_PC="$LINUX_BUILD/meson-uninstalled/libvlc-uninstalled.pc"

PATCH_0014="$REPO_ROOT/patches/kyber/0014-vlc-rs-video-path-bridge.patch"
MAC_VERIFIER="$SCRIPT_DIR/verify-macos-native-telemetry.sh"
HOST_VERIFIER="$SCRIPT_DIR/verify-host-evidence-sequence.sh"

EXPECTED_DESKTOP=6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f
EXPECTED_KYSDK=5836202aa4654cf64b5ca9fd204005fe4916be99
EXPECTED_KYMEDIA=e80eb6bb347ed0e378ae46a86f67ada2aa6079da
EXPECTED_TXPROTO=82694c38fb7d662ad364071382166edf8731db93
EXPECTED_VLC=dd2db54794384591684c1c86ce70eb64eb9eab15
EXPECTED_VLC_RS=7cbfc51313b4bb3ab07be51505b7054e4e2c366b
EXPECTED_LIBC=0.2.180
EXPECTED_PKG_CONFIG=0.3.30
SCRATCH_PREFIX=replaydesktop-vlc-rs-video-path-bridge

EXPECTED_0014_PATHS='src/media_player.rs
src/metrics.rs
src/sys.rs'

usage() {
    printf '%s\n' \
        "Usage: $0 --source|--tests|--replay" \
        "" \
        "  --source  verify pins, patch scope, owned contract, safety fences, and documentation" \
        "  --tests   run offline locked Rust/native ABI, link-origin, lint-delta, and stress gates" \
        "  --replay  replay all prerequisites, apply 0014 at its exact pin, and reverse cleanly"
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

assert_increasing_lines() {
    previous=0
    for current in "$@"; do
        [ -n "$current" ] || fail "required source-order marker is absent"
        [ "$current" -gt "$previous" ] ||
            fail "source-order marker is out of order at line $current"
        previous=$current
    done
}

patch_paths() {
    sed -n 's|^diff --git a/\([^ ]*\) b/.*$|\1|p' "$1" | sort
}

assert_patch_paths() {
    allowlist_patch=$1
    allowlist_expected=$2
    allowlist_actual=$(patch_paths "$allowlist_patch")
    [ "$allowlist_actual" = "$allowlist_expected" ] ||
        fail "patch path allowlist mismatch in $allowlist_patch"
}

added_patch_lines() {
    awk '
        /^\+\+\+ / { next }
        /^\+/ {
            sub(/^\+/, "")
            print
        }
    ' "$@"
}

assert_gitlink() {
    link_repo=$1
    link_tree=$2
    link_path=$3
    link_expected=$4
    link_actual=$(git -C "$link_repo" ls-tree "$link_tree" "$link_path" |
        awk '{print $3}')
    [ "$link_actual" = "$link_expected" ] ||
        fail "gitlink drifted: $link_repo $link_path"
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
    [ "$(git -C "$VLC" rev-parse HEAD)" = "$EXPECTED_VLC" ] ||
        fail "VLC working HEAD drifted"
    [ "$(git -C "$VLC_RS" rev-parse HEAD)" = "$EXPECTED_VLC_RS" ] ||
        fail "vlc-rs working HEAD drifted"

    for nested_repo in \
        "$KYBER_DESKTOP" \
        "$KYSDK" \
        "$KYMEDIA" \
        "$TXPROTO" \
        "$VLC" \
        "$VLC_RS"
    do
        git -C "$nested_repo" diff --cached --quiet ||
            fail "nested index is staged: $nested_repo"
    done

    assert_gitlink "$REPO_ROOT" HEAD upstream/kyber-desktop "$EXPECTED_DESKTOP"
    assert_gitlink "$KYBER_DESKTOP" "$EXPECTED_DESKTOP" kysdk "$EXPECTED_KYSDK"
    assert_gitlink "$KYSDK" "$EXPECTED_KYSDK" kymedia "$EXPECTED_KYMEDIA"
    assert_gitlink "$KYMEDIA" "$EXPECTED_KYMEDIA" subprojects/txproto "$EXPECTED_TXPROTO"
    assert_gitlink "$KYMEDIA" "$EXPECTED_KYMEDIA" subprojects/vlc "$EXPECTED_VLC"
    assert_gitlink "$KYMEDIA" "$EXPECTED_KYMEDIA" subprojects/vlc-rs "$EXPECTED_VLC_RS"
}

assert_parent_staging_safe() {
    staged_paths=$(git -C "$REPO_ROOT" diff --cached --name-only)
    if [ -n "$staged_paths" ]; then
        while IFS= read -r staged_path; do
            case "$staged_path" in
                README.md|patches/kyber/0014-vlc-rs-video-path-bridge.patch|scripts/verify-vlc-rs-video-path-bridge.sh)
                    ;;
                *)
                    fail "parent staged path is outside the 0014 allowlist: $staged_path"
                    ;;
            esac
        done <<<"$staged_paths"
    fi

    if printf '%s\n' "$staged_paths" |
        rg -q '^(upstream/kyber-desktop|patches/kyber/0003-linux-zlib-pic\.patch|log/|\.planning/phases/10-)'
    then
        fail "protected parent work is staged"
    fi
}

extract_legacy_setter() {
    awk '
        /    pub fn set_metrics_callback/ { inside = 1 }
        inside { print }
        inside && /^    }$/ { exit }
    '
}

verify_source() {
    require_command git
    require_command rg
    require_file "$PATCH_0014"
    require_file "$MAC_VERIFIER"
    require_file "$HOST_VERIFIER"
    require_file "$VLC_RS/src/sys.rs"
    require_file "$VLC_RS/src/metrics.rs"
    require_file "$VLC_RS/src/media_player.rs"

    "$MAC_VERIFIER" --source
    "$HOST_VERIFIER" --source
    assert_nested_heads
    assert_parent_staging_safe
    assert_patch_paths "$PATCH_0014" "$EXPECTED_0014_PATHS"

    git -C "$VLC_RS" apply --reverse --check "$PATCH_0014" ||
        fail "working vlc-rs source does not contain patch 0014"
    cmp "$PATCH_0014" <(
        git -C "$VLC_RS" diff --full-index --binary -- \
            src/media_player.rs src/metrics.rs src/sys.rs
    ) || fail "patch 0014 differs from the exact live three-file vlc-rs diff"
    git -C "$VLC_RS" diff --check -- \
        src/media_player.rs src/metrics.rs src/sys.rs

    sys_source="$VLC_RS/src/sys.rs"
    metrics_source="$VLC_RS/src/metrics.rs"
    player_source="$VLC_RS/src/media_player.rs"

    assert_contains "$sys_source" \
        'pub const LIBVLC_VIDEO_PATH_EVENT_VERSION: u16 = 1;'
    assert_contains "$sys_source" \
        'pub const LIBVLC_VIDEO_PATH_TEXT_CAPACITY: usize = 64;'
    assert_contains "$sys_source" \
        'pub struct libvlc_video_path_event'
    assert_contains "$sys_source" \
        'pub type libvlc_video_path_cb = unsafe extern "C" fn('
    assert_contains "$sys_source" \
        'const _: [(); 192] = [(); std::mem::size_of::<libvlc_video_path_event>()];'
    assert_contains "$sys_source" \
        'const _: [(); 8] = [(); std::mem::align_of::<libvlc_video_path_event>()];'
    assert_contains "$sys_source" \
        'const _: [(); 128] = [(); std::mem::offset_of!(libvlc_video_path_event, text)];'
    assert_contains "$sys_source" \
        'pub fn libvlc_media_player_set_video_path_callback('

    for public_type in \
        VideoPathEventKind \
        VideoPathCodec \
        VideoPathChroma \
        VideoPathTruth \
        VideoPathUnknownReason \
        VideoPathOutcome \
        VideoPathPolicyFlags \
        VideoPathOsFlags \
        VideoPathObservationFlags \
        VideoPathCallbackLoss \
        VideoPathEvent \
        VideoPathTerminal \
        VideoPathCallbackEvent \
        VideoPathCallbackDisposition
    do
        assert_contains "$metrics_source" "pub $(
            case "$public_type" in
                VideoPathPolicyFlags|VideoPathOsFlags|VideoPathObservationFlags|VideoPathCallbackLoss|VideoPathEvent|VideoPathTerminal)
                    printf 'struct'
                    ;;
                *)
                    printf 'enum'
                    ;;
            esac
        ) $public_type"
    done

    assert_contains "$metrics_source" \
        'pub const VIDEO_PATH_MAX_RECORDS: usize = 64;'
    assert_contains "$metrics_source" \
        'pub const VIDEO_PATH_MAX_RECORD_BYTES: usize = 1024;'
    assert_contains "$metrics_source" \
        'pub const VIDEO_PATH_MAX_CALLBACK_BYTES: usize = 64 * 1024;'
    assert_contains "$metrics_source" \
        'pub const VIDEO_PATH_V1_PREFIX_SIZE: usize = 192;'
    assert_contains "$metrics_source" 'Self::Other(other)'
    assert_contains "$metrics_source" 'pub fn from_bits_retain(bits: u32) -> Self'
    assert_contains "$metrics_source" 'events.is_null() || count == 0'
    assert_contains "$metrics_source" 'ptr::read_unaligned'
    assert_contains "$metrics_source" 'raw.pts_valid'
    assert_contains "$metrics_source" 'raw.reserved.iter().any'
    assert_contains "$metrics_source" 'str::from_utf8'
    assert_contains "$metrics_source" 'loss_values.iter().all'
    assert_contains "$metrics_source" 'raw.callback_loss_first_seq > raw.callback_loss_last_seq'

    assert_contains "$player_source" \
        'pub fn set_video_path_callback<C>('
    assert_contains "$player_source" \
        'callback: Box<VideoPathCallback>'
    assert_contains "$player_source" \
        'video_path_callback: Option<Box<VideoPathCallbackHandle>>'
    assert_contains "$player_source" \
        'static NEXT_VIDEO_PATH_CALLBACK_ID: AtomicUsize = AtomicUsize::new(1);'
    assert_contains "$player_source" \
        'static VIDEO_PATH_CALLBACK_REGISTRY: OnceLock<'
    assert_contains "$player_source" \
        'fn video_path_callback_state(data: *mut c_void) -> Option<Arc<VideoPathCallbackState>>'
    assert_contains "$player_source" \
        'let state = match video_path_callback_state(data)'
    assert_contains "$player_source" 'catch_unwind(AssertUnwindSafe'
    assert_contains "$player_source" 'VIDEO_PATH_CALLBACK_REJECTED'
    assert_contains "$player_source" 'saturating_add'
    assert_contains "$player_source" \
        'let callback_handle = self.video_path_callback.take();'
    assert_contains "$player_source" \
        'VideoPathCallbackEvent::ProducerTerminalUnconfirmed'
    assert_contains "$player_source" \
        'fn callback_triggered_owner_drop_keeps_in_flight_state_and_rejects_late_calls()'
    assert_contains "$player_source" \
        'fn release_unregisters_opaque_userdata_before_late_native_callbacks()'
    assert_contains "$player_source" \
        'fn concurrent_reject_and_panic_callbacks_keep_exact_loss_facts()'
    if added_patch_lines "$PATCH_0014" |
        rg -F -q 'data.cast::<VideoPathCallback'
    then
        fail "callback trampoline dereferences native userdata instead of resolving its opaque ID"
    fi

    release_call=$(literal_line "$player_source" '    release();')
    registry_remove=$(literal_line "$player_source" '        handle.unregister();')
    lifetime_close=$(literal_line "$player_source" \
        'state.native_lifetime_closed.store(true, Ordering::SeqCst);')
    terminal_snapshot=$(literal_line "$player_source" \
        'let terminal = state.terminal_snapshot();')
    terminal_dispatch=$(literal_line "$player_source" \
        '(state.callback)(VideoPathCallbackEvent::ProducerTerminalUnconfirmed(')
    assert_increasing_lines \
        "$release_call" \
        "$registry_remove" \
        "$lifetime_close" \
        "$terminal_snapshot" \
        "$terminal_dispatch"

    baseline_legacy=$(
        git -C "$VLC_RS" show "$EXPECTED_VLC_RS:src/media_player.rs" |
            extract_legacy_setter
    )
    current_legacy=$(extract_legacy_setter <"$player_source")
    [ "$baseline_legacy" = "$current_legacy" ] ||
        fail "legacy MediaPlayer::set_metrics_callback changed"
    baseline_metrics_entry=$(
        git -C "$VLC_RS" show "$EXPECTED_VLC_RS:src/metrics.rs" |
            sed -n '1,5p'
    )
    current_metrics_entry=$(sed -n '1,5p' "$metrics_source")
    [ "$baseline_metrics_entry" = "$current_metrics_entry" ] ||
        fail "legacy MetricsEntry changed"

    if added_patch_lines "$PATCH_0014" |
        rg -n '(println!|eprintln!|dbg!|log::|tracing::|warn!|error!|info!|debug!|trace!).*(event|record|text|payload|metric|token|password)'
    then
        fail "patch 0014 additions log callback payload or secret content"
    fi

    assert_contains "$REPO_ROOT/README.md" \
        '0014-vlc-rs-video-path-bridge.patch'
    assert_contains "$REPO_ROOT/README.md" \
        'Its callback accepts at most 64 records'
    assert_contains "$REPO_ROOT/README.md" \
        'ProducerTerminalUnconfirmed(VideoPathTerminal)'
    assert_contains "$REPO_ROOT/README.md" \
        'That event is not a native clean-completion'
    assert_contains "$REPO_ROOT/README.md" \
        'monotonic, never-reused opaque ID'
    assert_contains "$REPO_ROOT/README.md" \
        'does not treat the reference-count decrement as proof'
    assert_contains "$REPO_ROOT/README.md" \
        'Kyctl consumption,'

    printf '%s\n' \
        'PASS: exact Desktop/Kysdk/Kymedia/txproto/VLC/vlc-rs pins and 0014 path allowlist match' \
        'PASS: raw ABI constants/layout, owned public types, bounded walk, and forward-preserving values are present' \
        'PASS: panic-to-reject accounting, Arc in-flight guards, and release-before-unregister-before-terminal ordering are explicit' \
        'PASS: legacy scalar API is byte-unchanged and README freezes the terminal-unconfirmed boundary' \
        'SOURCE=PASS'
}

safe_remove_scratch() {
    scratch_root=$1
    scratch_parent=$2
    scratch_canonical=$3

    [ -e "$scratch_root" ] || return 0
    [ ! -L "$scratch_root" ] ||
        fail "refusing to clean symlink scratch root: $scratch_root"
    current_canonical=$(realpath -e "$scratch_root")
    [ "$current_canonical" = "$scratch_canonical" ] ||
        fail "scratch canonical path changed before cleanup"
    [ "$(dirname "$current_canonical")" = "$scratch_parent" ] ||
        fail "scratch parent changed before cleanup"
    case "$(basename "$current_canonical")" in
        "$SCRATCH_PREFIX".*) ;;
        *) fail "refusing to clean unexpected scratch basename" ;;
    esac
    [ "$current_canonical" != "$REPO_ROOT" ] ||
        fail "refusing to clean repository root"
    case "$REPO_ROOT/" in
        "$current_canonical"/*)
            fail "refusing to clean a repository ancestor"
            ;;
    esac
    find "$current_canonical" -depth -delete
}

create_scratch() {
    scratch_base=${TMPDIR:-/tmp}
    scratch_base=$(realpath -e "$scratch_base")
    scratch_root=$(mktemp -d "$scratch_base/$SCRATCH_PREFIX.XXXXXX")
    chmod 700 "$scratch_root"
    scratch_canonical=$(realpath -e "$scratch_root")
    [ "$(stat -c '%a' "$scratch_canonical")" = 700 ] ||
        fail "scratch root is not mode 0700"
    printf '%s\n%s\n%s\n' "$scratch_root" "$scratch_base" "$scratch_canonical"
}

clone_at_pin() {
    clone_source=$1
    clone_destination=$2
    clone_commit=$3
    git -c advice.detachedHead=false clone --quiet --no-hardlinks \
        "$clone_source" "$clone_destination"
    git -C "$clone_destination" checkout --quiet --detach "$clone_commit"
    [ "$(git -C "$clone_destination" rev-parse HEAD)" = "$clone_commit" ] ||
        fail "scratch clone did not reach exact commit: $clone_commit"
}

create_pkgconfig_view() {
    pc_view=$1
    mkdir -m 700 "$pc_view"
    cp "$VLC_UNINSTALLED_PC" "$pc_view/libvlc.pc"

    system_libidn_pc=$(
        PKG_CONFIG_PATH= \
        PKG_CONFIG_LIBDIR=/usr/lib/pkgconfig:/usr/share/pkgconfig \
            pkg-config --path libidn
    ) || fail "system libidn pkg-config metadata is unavailable"
    require_file "$system_libidn_pc"
    cp "$system_libidn_pc" "$pc_view/libidn.pc"

    PKG_CONFIG_PATH= PKG_CONFIG_LIBDIR="$pc_view" \
        pkg-config --exists libvlc ||
        fail "isolated libVLC pkg-config view is incomplete"
    [ "$(
        PKG_CONFIG_PATH= PKG_CONFIG_LIBDIR="$pc_view" \
            pkg-config --variable=pcfiledir libvlc
    )" = "$pc_view" ] ||
        fail "libVLC pkg-config metadata escaped the isolated view"
    [ "$(
        PKG_CONFIG_PATH= PKG_CONFIG_LIBDIR="$pc_view" \
            pkg-config --libs-only-L libvlc
    )" = "-L$VLC_LIB_DIR" ] ||
        fail "libVLC pkg-config library path is not the exact patched build"
}

write_layout_probe() {
    probe_path=$1
    cat >"$probe_path" <<'EOF'
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <vlc/libvlc.h>
#include <vlc/libvlc_media_player.h>
#include <vlc/libvlc_metrics.h>

static int probe_callback(void *opaque,
                          const struct libvlc_video_path_event *events,
                          size_t count)
{
    (void)opaque;
    (void)events;
    (void)count;
    return 0;
}

static libvlc_video_path_cb callback_signature = probe_callback;
static int (*setter_signature)(libvlc_media_player_t *,
                               libvlc_video_path_cb,
                               void *) =
    libvlc_media_player_set_video_path_callback;

#define PRINT_OFFSET(field) \
    printf(#field "=%zu\n", offsetof(struct libvlc_video_path_event, field))

int main(void)
{
    printf("version=%u\n", (unsigned)LIBVLC_VIDEO_PATH_EVENT_VERSION);
    printf("text_capacity=%u\n", (unsigned)LIBVLC_VIDEO_PATH_TEXT_CAPACITY);
    printf("size=%zu\n", sizeof(struct libvlc_video_path_event));
    printf("align=%zu\n", _Alignof(struct libvlc_video_path_event));
    PRINT_OFFSET(version);
    PRINT_OFFSET(struct_size);
    PRINT_OFFSET(kind);
    PRINT_OFFSET(producer_seq);
    PRINT_OFFSET(monotonic_ts);
    PRINT_OFFSET(pts);
    PRINT_OFFSET(pts_valid);
    PRINT_OFFSET(reserved);
    PRINT_OFFSET(codec);
    PRINT_OFFSET(chroma);
    PRINT_OFFSET(profile);
    PRINT_OFFSET(bit_depth);
    PRINT_OFFSET(hardware_state);
    PRINT_OFFSET(iosurface_state);
    PRINT_OFFSET(outcome);
    PRINT_OFFSET(unknown_reason);
    PRINT_OFFSET(decoded_fourcc);
    PRINT_OFFSET(policy_flags);
    PRINT_OFFSET(os_flags);
    PRINT_OFFSET(status);
    PRINT_OFFSET(observation_flags);
    PRINT_OFFSET(callback_lost_events);
    PRINT_OFFSET(callback_loss_first_seq);
    PRINT_OFFSET(callback_loss_last_seq);
    PRINT_OFFSET(callback_loss_generation);
    PRINT_OFFSET(text);
    return callback_signature == NULL || setter_signature == NULL;
}
EOF
}

write_scratch_workspace() {
    workspace_root=$1
    vlc_rs_path=$2

    mkdir -m 700 "$workspace_root/.cargo" "$workspace_root/consumer"
    mkdir -m 700 "$workspace_root/consumer/src"

    cat >"$workspace_root/Cargo.toml" <<'EOF'
[workspace]
resolver = "2"
members = ["vlc-rs", "consumer"]
EOF

    cat >"$workspace_root/.cargo/config.toml" <<EOF
[patch.crates-io]
vlc-rs = { path = "$vlc_rs_path" }

[net]
offline = true
EOF

    cat >"$workspace_root/consumer/Cargo.toml" <<'EOF'
[package]
name = "video-path-bridge-consumer"
version = "0.1.0"
edition = "2021"

[dependencies]
vlc-rs = "=0.3.0"
EOF

    cat >"$workspace_root/consumer/src/main.rs" <<'EOF'
use std::ffi::c_void;
use std::mem::{align_of, offset_of, size_of};
use std::os::raw::c_int;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use vlc::{
    sys, Instance, MediaPlayer, MetricsEntry, VideoPathCallbackDisposition,
    VideoPathCallbackEvent,
};

unsafe extern "C" fn raw_callback(
    _opaque: *mut c_void,
    _events: *const sys::libvlc_video_path_event,
    _count: usize,
) -> c_int {
    0
}

macro_rules! print_offset {
    ($field:ident) => {
        println!(
            "{}={}",
            stringify!($field),
            offset_of!(sys::libvlc_video_path_event, $field)
        );
    };
}

fn legacy_scalar_signature<F>(player: &MediaPlayer, callback: F)
where
    F: Fn(Vec<MetricsEntry>) + Send + 'static,
{
    player.set_metrics_callback(callback);
}

fn main() {
    let callback_signature: sys::libvlc_video_path_cb = raw_callback;
    let setter_signature: unsafe extern "C" fn(
        *mut sys::libvlc_media_player_t,
        sys::libvlc_video_path_cb,
        *mut c_void,
    ) -> c_int = sys::libvlc_media_player_set_video_path_callback;
    std::hint::black_box(callback_signature);
    std::hint::black_box(setter_signature);
    std::hint::black_box(
        legacy_scalar_signature::<fn(Vec<MetricsEntry>)>
            as fn(&MediaPlayer, fn(Vec<MetricsEntry>)),
    );

    println!("version={}", sys::LIBVLC_VIDEO_PATH_EVENT_VERSION);
    println!("text_capacity={}", sys::LIBVLC_VIDEO_PATH_TEXT_CAPACITY);
    println!("size={}", size_of::<sys::libvlc_video_path_event>());
    println!("align={}", align_of::<sys::libvlc_video_path_event>());
    print_offset!(version);
    print_offset!(struct_size);
    print_offset!(kind);
    print_offset!(producer_seq);
    print_offset!(monotonic_ts);
    print_offset!(pts);
    print_offset!(pts_valid);
    print_offset!(reserved);
    print_offset!(codec);
    print_offset!(chroma);
    print_offset!(profile);
    print_offset!(bit_depth);
    print_offset!(hardware_state);
    print_offset!(iosurface_state);
    print_offset!(outcome);
    print_offset!(unknown_reason);
    print_offset!(decoded_fourcc);
    print_offset!(policy_flags);
    print_offset!(os_flags);
    print_offset!(status);
    print_offset!(observation_flags);
    print_offset!(callback_lost_events);
    print_offset!(callback_loss_first_seq);
    print_offset!(callback_loss_last_seq);
    print_offset!(callback_loss_generation);
    print_offset!(text);

    let terminal_calls = Arc::new(AtomicUsize::new(0));
    let callback_terminal_calls = Arc::clone(&terminal_calls);
    let instance = Instance::new().expect("exact libVLC instance");
    let mut player = MediaPlayer::new(&instance).expect("exact libVLC media player");
    player
        .set_video_path_callback(move |event| {
            if matches!(
                event,
                VideoPathCallbackEvent::ProducerTerminalUnconfirmed(_)
            ) {
                callback_terminal_calls.fetch_add(1, Ordering::SeqCst);
            }
            VideoPathCallbackDisposition::Accept
        })
        .expect("one-time video-path callback install");
    drop(player);
    assert_eq!(terminal_calls.load(Ordering::SeqCst), 1);
}
EOF
}

pin_scratch_lock() {
    workspace_root=$1
    (
        cd "$workspace_root"
        CARGO_NET_OFFLINE=true cargo +1.89.0 generate-lockfile --offline
        CARGO_NET_OFFLINE=true cargo +1.89.0 update \
            -p libc --precise "$EXPECTED_LIBC" --offline
        CARGO_NET_OFFLINE=true cargo +1.89.0 update \
            -p pkg-config --precise "$EXPECTED_PKG_CONFIG" --offline
    )
}

assert_metadata_path_override() {
    metadata_file=$1
    expected_manifest=$2

    vlc_package_count=$(jq -r \
        '[.packages[] | select(.name == "vlc-rs" and .version == "0.3.0")] | length' \
        "$metadata_file")
    [ "$vlc_package_count" -eq 1 ] ||
        fail "Cargo metadata did not resolve exactly one vlc-rs 0.3.0"
    vlc_package_id=$(jq -r \
        '.packages[] | select(.name == "vlc-rs" and .version == "0.3.0") | .id' \
        "$metadata_file")
    vlc_source=$(jq -r \
        '.packages[] | select(.name == "vlc-rs" and .version == "0.3.0") | .source // "path"' \
        "$metadata_file")
    vlc_manifest=$(jq -r \
        '.packages[] | select(.name == "vlc-rs" and .version == "0.3.0") | .manifest_path' \
        "$metadata_file")
    [ "$vlc_source" = path ] ||
        fail "Cargo selected a registry vlc-rs instead of the path override"
    [ "$(realpath -e "$vlc_manifest")" = "$(realpath -e "$expected_manifest")" ] ||
        fail "Cargo vlc-rs manifest escaped the exact scratch checkout"

    consumer_id=$(jq -r \
        '.packages[] | select(.name == "video-path-bridge-consumer") | .id' \
        "$metadata_file")
    jq -e \
        --arg consumer "$consumer_id" \
        --arg vlc "$vlc_package_id" \
        '.resolve.nodes[] | select(.id == $consumer) | any(.deps[]; .pkg == $vlc)' \
        "$metadata_file" >/dev/null ||
        fail "scratch consumer did not resolve its registry-form dependency through 0014"
}

assert_runtime_origin() {
    binary=$1
    soname=$2
    expected_path=$3
    resolved_path=$(ldd "$binary" |
        awk -v wanted="$soname" '$1 == wanted { print $3; exit }')
    [ -n "$resolved_path" ] ||
        fail "$binary does not load $soname"
    [ "$(realpath -e "$resolved_path")" = "$(realpath -e "$expected_path")" ] ||
        fail "$binary loaded unexpected $soname: $resolved_path"
}

verify_baseline_aware_format() {
    patched_checkout=$1
    baseline_checkout=$2
    scratch_root=$3

    rustfmt +1.89.0 --edition 2018 --check \
        "$patched_checkout/src/metrics.rs"

    added_lines_file="$scratch_root/0014-added-lines.txt"
    added_patch_lines "$PATCH_0014" |
        sed '/^$/d' |
        sort -u >"$added_lines_file"

    for source_name in sys.rs media_player.rs; do
        format_copy="$scratch_root/formatted-$source_name"
        format_diff="$scratch_root/format-$source_name.diff"
        cp "$patched_checkout/src/$source_name" "$format_copy"
        rustfmt +1.89.0 --edition 2018 "$format_copy"
        diff -u "$patched_checkout/src/$source_name" "$format_copy" \
            >"$format_diff" || true

        while IFS= read -r removed_line; do
            [ -n "$removed_line" ] || continue
            if grep -Fqx -- "$removed_line" "$added_lines_file" &&
                ! grep -Fqx -- "$removed_line" "$baseline_checkout/src/$source_name"
            then
                fail "rustfmt would change a line added by 0014 in $source_name: $removed_line"
            fi
        done < <(
            awk '/^--- / { next }
                 /^-/ { sub(/^-/, ""); print }' "$format_diff"
        )
    done
}

clippy_error_headlines() {
    log_file=$1
    rg '^error:' "$log_file" |
        rg -v '^error: could not compile ' |
        sort
}

verify_baseline_aware_clippy() {
    workspace_root=$1
    baseline_checkout=$2
    scratch_root=$3
    rustflags=$4

    pin_scratch_lock "$baseline_checkout"
    baseline_log="$scratch_root/clippy-baseline.log"
    patched_log="$scratch_root/clippy-patched.log"

    set +e
    (
        cd "$baseline_checkout"
        CARGO_NET_OFFLINE=true \
        CARGO_TARGET_DIR="$scratch_root/clippy-baseline-target" \
        RUSTFLAGS="$rustflags" \
            cargo +1.89.0 clippy --locked --offline \
                --no-default-features --all-targets -- -D warnings
    ) >"$baseline_log" 2>&1
    baseline_status=$?
    (
        cd "$workspace_root"
        CARGO_NET_OFFLINE=true \
        CARGO_TARGET_DIR="$scratch_root/clippy-patched-target" \
        RUSTFLAGS="$rustflags" \
            cargo +1.89.0 clippy --locked --offline \
                --no-default-features -p vlc-rs --all-targets -- -D warnings
    ) >"$patched_log" 2>&1
    patched_status=$?
    set -e

    [ "$baseline_status" -ne 0 ] ||
        fail "pristine vlc-rs unexpectedly lost its known strict-clippy baseline"
    [ "$patched_status" -ne 0 ] ||
        fail "patched strict clippy unexpectedly passed while baseline still fails"
    baseline_headlines="$scratch_root/clippy-baseline-headlines.txt"
    patched_headlines="$scratch_root/clippy-patched-headlines.txt"
    clippy_error_headlines "$baseline_log" >"$baseline_headlines"
    clippy_error_headlines "$patched_log" >"$patched_headlines"
    [ -s "$baseline_headlines" ] ||
        fail "strict-clippy baseline failed without classifiable diagnostics"
    cmp "$baseline_headlines" "$patched_headlines" ||
        fail "patch 0014 introduced a strict-clippy diagnostic beyond the pristine baseline"
}

verify_tests() {
    for command_name in \
        cargo \
        cc \
        git \
        jq \
        ldd \
        meson \
        nm \
        pkg-config \
        realpath \
        rg \
        rustfmt \
        sha256sum
    do
        require_command "$command_name"
    done
    require_file "$VLC_UNINSTALLED_PC"
    require_file "$LINUX_BUILD/build.ninja"

    verify_source
    recursive_heads_before=$(git -C "$KYBER_DESKTOP" submodule status --recursive)

    "$MAC_VERIFIER" --tests
    "$HOST_VERIFIER" --tests
    require_file "$VLC_LIBRARY"

    legacy_symbols=$(nm -D --defined-only "$VLC_LIBRARY" |
        awk '$3 == "libvlc_media_player_set_metrics_callback" { count++ }
             END { print count + 0 }')
    video_symbols=$(nm -D --defined-only "$VLC_LIBRARY" |
        awk '$3 == "libvlc_media_player_set_video_path_callback" { count++ }
             END { print count + 0 }')
    [ "$legacy_symbols" -eq 1 ] && [ "$video_symbols" -eq 1 ] ||
        fail "exact patched libVLC must export each callback setter once"

    mapfile -t scratch_parts < <(create_scratch)
    scratch_root=${scratch_parts[0]}
    scratch_parent=${scratch_parts[1]}
    scratch_canonical=${scratch_parts[2]}
    trap 'safe_remove_scratch "$scratch_root" "$scratch_parent" "$scratch_canonical"' EXIT

    clone_at_pin "$VLC_RS" "$scratch_root/workspace/vlc-rs" "$EXPECTED_VLC_RS"
    clone_at_pin "$VLC_RS" "$scratch_root/baseline-vlc-rs" "$EXPECTED_VLC_RS"
    git -C "$scratch_root/workspace/vlc-rs" apply --check \
        --whitespace=error-all "$PATCH_0014"
    git -C "$scratch_root/workspace/vlc-rs" apply "$PATCH_0014"

    write_scratch_workspace \
        "$scratch_root/workspace" \
        "$(realpath -e "$scratch_root/workspace/vlc-rs")"
    create_pkgconfig_view "$scratch_root/pkgconfig"
    write_layout_probe "$scratch_root/layout.c"

    export PKG_CONFIG_PATH=
    export PKG_CONFIG_LIBDIR="$scratch_root/pkgconfig"
    export LD_LIBRARY_PATH="$VLC_LIB_DIR:$VLC_CORE_DIR"
    unset LD_PRELOAD

    cc -std=c11 -Wall -Wextra -Werror \
        $(pkg-config --cflags libvlc) \
        "$scratch_root/layout.c" \
        $(pkg-config --libs libvlc) \
        -o "$scratch_root/layout"
    "$scratch_root/layout" >"$scratch_root/c-layout.txt"

    pin_scratch_lock "$scratch_root/workspace"
    lock_hash_before=$(sha256sum "$scratch_root/workspace/Cargo.lock" |
        awk '{print $1}')
    (
        cd "$scratch_root/workspace"
        CARGO_NET_OFFLINE=true cargo +1.89.0 metadata \
            --locked --offline --format-version 1 \
            >"$scratch_root/metadata.json"
    )
    assert_metadata_path_override \
        "$scratch_root/metadata.json" \
        "$scratch_root/workspace/vlc-rs/Cargo.toml"

    rustflags="-Lnative=$VLC_LIB_DIR -ldylib=vlc"
    (
        cd "$scratch_root/workspace"
        CARGO_NET_OFFLINE=true \
        CARGO_TARGET_DIR="$scratch_root/no-default-target" \
        RUSTFLAGS="$rustflags" \
            cargo +1.89.0 test --locked --offline \
                --no-default-features -p vlc-rs
        CARGO_NET_OFFLINE=true \
        CARGO_TARGET_DIR="$scratch_root/no-default-target" \
        RUSTFLAGS="$rustflags" \
            cargo +1.89.0 test --locked --offline \
                --no-default-features -p vlc-rs --no-run \
                --message-format=json \
                >"$scratch_root/no-default-artifacts.jsonl"
    )

    test_binary=$(jq -r \
        'select(.reason == "compiler-artifact" and
                .target.name == "vlc" and
                .profile.test == true and
                .executable != null) | .executable' \
        "$scratch_root/no-default-artifacts.jsonl" |
        tail -1)
    require_file "$test_binary"

    stress_run=1
    while [ "$stress_run" -le 20 ]; do
        for test_name in \
            media_player::video_path_tests::contains_consumer_panic_and_returns_the_stable_rejection_code \
            media_player::video_path_tests::concurrent_callbacks_keep_exact_bounded_facts \
            media_player::video_path_tests::concurrent_reject_and_panic_callbacks_keep_exact_loss_facts \
            media_player::video_path_tests::callback_triggered_owner_drop_keeps_in_flight_state_and_rejects_late_calls \
            media_player::video_path_tests::release_unregisters_opaque_userdata_before_late_native_callbacks \
            media_player::video_path_tests::release_precedes_one_terminal_event_and_userdata_drop \
            media_player::video_path_tests::terminal_consumer_panic_is_contained_before_userdata_drop \
            media_player::video_path_tests::failed_and_duplicate_install_drop_candidate_userdata \
            media_player::video_path_tests::invalid_later_record_rejects_the_atomic_batch_without_dispatch
        do
            "$test_binary" "$test_name" --exact >/dev/null 2>&1
        done
        stress_run=$((stress_run + 1))
    done

    (
        cd "$scratch_root/workspace"
        CARGO_NET_OFFLINE=true \
        CARGO_TARGET_DIR="$scratch_root/default-target" \
            cargo +1.89.0 run --locked --offline \
                -p video-path-bridge-consumer --quiet \
                >"$scratch_root/rust-layout.txt"
    )
    cmp "$scratch_root/c-layout.txt" "$scratch_root/rust-layout.txt" ||
        fail "C and Rust video-path ABI/layout probes disagree"

    consumer_binary="$scratch_root/default-target/debug/video-path-bridge-consumer"
    require_file "$consumer_binary"
    consumer_setter_relocations=$(nm -D "$consumer_binary" |
        awk '($1 == "U" &&
              $2 ~ /^libvlc_media_player_set_video_path_callback(@|$)/) ||
             ($2 == "U" &&
              $3 ~ /^libvlc_media_player_set_video_path_callback(@|$)/) {
                 count++
             }
             END { print count + 0 }')
    [ "$consumer_setter_relocations" -eq 1 ] ||
        fail "consumer does not retain exactly one additive setter relocation"
    assert_runtime_origin "$consumer_binary" libvlc.so "$VLC_LIBRARY"
    core_soname=$(basename "$(readlink -f "$VLC_CORE_DIR/libvlccore.so")")
    core_resolved_name=$(ldd "$consumer_binary" |
        awk '$1 ~ /^libvlccore\.so/ { print $1; exit }')
    [ -n "$core_resolved_name" ] ||
        fail "consumer does not load the exact vlccore dependency"
    assert_runtime_origin \
        "$consumer_binary" \
        "$core_resolved_name" \
        "$VLC_CORE_DIR/$core_soname"

    verify_baseline_aware_format \
        "$scratch_root/workspace/vlc-rs" \
        "$scratch_root/baseline-vlc-rs" \
        "$scratch_root"
    verify_baseline_aware_clippy \
        "$scratch_root/workspace" \
        "$scratch_root/baseline-vlc-rs" \
        "$scratch_root" \
        "$rustflags"

    lock_hash_after=$(sha256sum "$scratch_root/workspace/Cargo.lock" |
        awk '{print $1}')
    [ "$lock_hash_after" = "$lock_hash_before" ] ||
        fail "Cargo.lock changed after the offline locked verification began"
    [ "$(git -C "$KYBER_DESKTOP" submodule status --recursive)" = \
        "$recursive_heads_before" ] ||
        fail "recursive nested HEADs changed during tests"

    safe_remove_scratch "$scratch_root" "$scratch_parent" "$scratch_canonical"
    trap - EXIT

    printf '%s\n' \
        'PASS: offline locked Cargo resolves registry-form vlc-rs 0.3.0 only from the exact 0014 scratch path' \
        'PASS: all 22 unit tests and 180 repeated panic/reject/concurrency/re-entrant-release stress invocations pass' \
        'PASS: C and Rust agree on every V1 offset/signature and the real safe setter/release smoke passes' \
        'PASS: ldd and symbols bind only the exact patched builddir libVLC; no registry/system/rootfs fallback is accepted' \
        'PASS: 0014 adds no rustfmt or strict-clippy diagnostic beyond the pinned crate baseline' \
        'TESTS=PASS'
}

verify_replay() {
    require_command cmp
    require_command git
    require_command realpath
    verify_source

    recursive_heads_before=$(git -C "$KYBER_DESKTOP" submodule status --recursive)
    parent_status_before=$(git -C "$REPO_ROOT" status --porcelain=v1 --untracked-files=all)

    "$MAC_VERIFIER" --replay
    "$HOST_VERIFIER" --replay

    mapfile -t scratch_parts < <(create_scratch)
    scratch_root=${scratch_parts[0]}
    scratch_parent=${scratch_parts[1]}
    scratch_canonical=${scratch_parts[2]}
    trap 'safe_remove_scratch "$scratch_root" "$scratch_parent" "$scratch_canonical"' EXIT

    clone_at_pin "$VLC_RS" "$scratch_root/vlc-rs" "$EXPECTED_VLC_RS"
    git -C "$scratch_root/vlc-rs" apply --check \
        --whitespace=error-all "$PATCH_0014"
    git -C "$scratch_root/vlc-rs" apply "$PATCH_0014"

    replay_paths=$(git -C "$scratch_root/vlc-rs" status --short |
        sed -n 's/^ M //p' |
        sort)
    [ "$replay_paths" = "$EXPECTED_0014_PATHS" ] ||
        fail "0014 replay changed paths outside its strict allowlist"
    for replay_path in $EXPECTED_0014_PATHS; do
        cmp "$scratch_root/vlc-rs/$replay_path" "$VLC_RS/$replay_path" ||
            fail "0014 replay differs from live source: $replay_path"
    done

    git -C "$scratch_root/vlc-rs" apply --reverse --check "$PATCH_0014"
    git -C "$scratch_root/vlc-rs" apply --reverse "$PATCH_0014"
    [ -z "$(git -C "$scratch_root/vlc-rs" status --short)" ] ||
        fail "vlc-rs replay did not reverse to a clean tree"
    [ "$(git -C "$scratch_root/vlc-rs" rev-parse HEAD)" = "$EXPECTED_VLC_RS" ] ||
        fail "vlc-rs replay HEAD drifted"

    safe_remove_scratch "$scratch_root" "$scratch_parent" "$scratch_canonical"
    trap - EXIT

    [ "$(git -C "$KYBER_DESKTOP" submodule status --recursive)" = \
        "$recursive_heads_before" ] ||
        fail "recursive nested HEADs changed during replay"
    [ "$(git -C "$REPO_ROOT" status --porcelain=v1 --untracked-files=all)" = \
        "$parent_status_before" ] ||
        fail "parent dirty/untracked state changed during replay"
    assert_nested_heads

    printf '%s\n' \
        'PASS: prerequisite VLC 0010->0011 replay and host 0008->0012 / 0009->0013 replay pass first' \
        'PASS: 0014 applies only to three vlc-rs files at 7cbfc513 and byte-matches the working source' \
        'PASS: reverse replay returns to the exact clean vlc-rs pin without changing recursive HEADs or parent work' \
        'REPLAY=PASS'
}

case "${1:-}" in
    --source)
        [ "$#" -eq 1 ] || {
            usage >&2
            exit 64
        }
        verify_source
        ;;
    --tests)
        [ "$#" -eq 1 ] || {
            usage >&2
            exit 64
        }
        verify_tests
        ;;
    --replay)
        [ "$#" -eq 1 ] || {
            usage >&2
            exit 64
        }
        verify_replay
        ;;
    *)
        usage >&2
        exit 64
        ;;
esac
