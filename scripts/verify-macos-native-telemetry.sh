#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
SCRIPT_PATH="$SCRIPT_DIR/verify-macos-native-telemetry.sh"
KYBER_DESKTOP="$REPO_ROOT/upstream/kyber-desktop"
KYSDK="$KYBER_DESKTOP/kysdk"
KYMEDIA="$KYSDK/kymedia"
VLC="$KYMEDIA/subprojects/vlc"
LINUX_BUILD="$KYMEDIA/builddir-linux"
PATCH_0010="$REPO_ROOT/patches/kyber/0010-vlc-video-path-telemetry-abi.patch"
PATCH_0011="$REPO_ROOT/patches/kyber/0011-vlc-macos-decoder-telemetry.patch"

EXPECTED_DESKTOP=6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f
EXPECTED_KYSDK=5836202aa4654cf64b5ca9fd204005fe4916be99
EXPECTED_KYMEDIA=e80eb6bb347ed0e378ae46a86f67ada2aa6079da
EXPECTED_VLC=dd2db54794384591684c1c86ce70eb64eb9eab15
EXPECTED_REMOTE_MESON=1.10.2
EXPECTED_REMOTE_NINJA=1.13.2

EXPECTED_0010_PATHS='include/vlc/libvlc_media_player.h
include/vlc/libvlc_metrics.h
include/vlc_metrics.h
include/vlc_player.h
lib/libvlc.sym
lib/media_player.c
modules/access/kymux.c
modules/access/kymux_video_config.c
modules/access/kymux_video_config.h
modules/access/meson.build
src/input/decoder_helpers.c
src/misc/metrics.c
src/misc/metrics.h
src/player/player.c
test/libvlc/meson.build
test/libvlc/video_path_metrics.c'

EXPECTED_0011_PATHS='modules/codec/videotoolbox/decoder.c
modules/video_output/apple/VLCSampleBufferDisplay.m
test/libvlc/meson.build
test/libvlc/video_path_metrics.c'

EXPECTED_COMBINED_PATHS='include/vlc/libvlc_media_player.h
include/vlc/libvlc_metrics.h
include/vlc_metrics.h
include/vlc_player.h
lib/libvlc.sym
lib/media_player.c
modules/access/kymux.c
modules/access/kymux_video_config.c
modules/access/kymux_video_config.h
modules/access/meson.build
modules/codec/videotoolbox/decoder.c
modules/video_output/apple/VLCSampleBufferDisplay.m
src/input/decoder_helpers.c
src/misc/metrics.c
src/misc/metrics.h
src/player/player.c
test/libvlc/meson.build
test/libvlc/video_path_metrics.c'

usage() {
    printf '%s\n' \
        "Usage: $0 --source|--tests|--replay|--mac-smoke user@host" \
        "" \
        "  --source             verify pins, patch allowlists, ABI, semantics, and log safety" \
        "  --tests              run the registered Linux native test and production link gates" \
        "  --replay             apply/reverse both patches from the exact pinned VLC commit" \
        "  --mac-smoke user@host build and test in a restrictive remote scratch directory only"
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

assert_nested_heads() {
    [ "$(git -C "$KYBER_DESKTOP" rev-parse HEAD)" = "$EXPECTED_DESKTOP" ] ||
        fail "Kyber Desktop working HEAD drifted"
    [ "$(git -C "$KYSDK" rev-parse HEAD)" = "$EXPECTED_KYSDK" ] ||
        fail "Kyber SDK working HEAD drifted"
    [ "$(git -C "$KYMEDIA" rev-parse HEAD)" = "$EXPECTED_KYMEDIA" ] ||
        fail "Kymedia working HEAD drifted"
    [ "$(git -C "$VLC" rev-parse HEAD)" = "$EXPECTED_VLC" ] ||
        fail "VLC working HEAD drifted"

    desktop_link=$(git -C "$REPO_ROOT" ls-tree HEAD upstream/kyber-desktop |
        awk '{print $3}')
    kysdk_link=$(git -C "$KYBER_DESKTOP" ls-tree "$EXPECTED_DESKTOP" kysdk |
        awk '{print $3}')
    kymedia_link=$(git -C "$KYSDK" ls-tree "$EXPECTED_KYSDK" kymedia |
        awk '{print $3}')
    vlc_link=$(git -C "$KYMEDIA" ls-tree "$EXPECTED_KYMEDIA" subprojects/vlc |
        awk '{print $3}')

    [ "$desktop_link" = "$EXPECTED_DESKTOP" ] ||
        fail "parent Kyber Desktop gitlink drifted"
    [ "$kysdk_link" = "$EXPECTED_KYSDK" ] ||
        fail "Kyber SDK gitlink drifted"
    [ "$kymedia_link" = "$EXPECTED_KYMEDIA" ] ||
        fail "Kymedia gitlink drifted"
    [ "$vlc_link" = "$EXPECTED_VLC" ] ||
        fail "VLC gitlink drifted"
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

verify_source() {
    require_command git
    require_command rg
    require_file "$PATCH_0010"
    require_file "$PATCH_0011"
    require_file "$VLC/include/vlc/libvlc_metrics.h"
    require_file "$VLC/modules/codec/videotoolbox/decoder.c"
    require_file "$VLC/modules/video_output/apple/VLCSampleBufferDisplay.m"
    require_file "$VLC/test/libvlc/video_path_metrics.c"

    assert_nested_heads
    assert_patch_paths "$PATCH_0010" "$EXPECTED_0010_PATHS"
    assert_patch_paths "$PATCH_0011" "$EXPECTED_0011_PATHS"

    public_metrics="$VLC/include/vlc/libvlc_metrics.h"
    player_header="$VLC/include/vlc/libvlc_media_player.h"
    core_metrics="$VLC/src/misc/metrics.c"
    decoder="$VLC/modules/codec/videotoolbox/decoder.c"
    renderer="$VLC/modules/video_output/apple/VLCSampleBufferDisplay.m"
    native_test="$VLC/test/libvlc/video_path_metrics.c"

    assert_contains "$public_metrics" 'typedef int (*libvlc_metrics_cb)'
    assert_contains "$public_metrics" 'const char *key;'
    assert_contains "$public_metrics" 'int64_t value;'
    assert_contains "$public_metrics" 'LIBVLC_VIDEO_PATH_EVENT_VERSION'
    assert_contains "$public_metrics" 'uint16_t struct_size;'
    assert_contains "$public_metrics" 'uint64_t producer_seq;'
    assert_contains "$public_metrics" 'callback_lost_events'
    assert_contains "$public_metrics" 'callback_loss_first_seq'
    assert_contains "$public_metrics" 'callback_loss_last_seq'
    assert_contains "$public_metrics" 'callback_loss_generation'
    assert_contains "$public_metrics" 'LIBVLC_VIDEO_PATH_TEXT_CAPACITY 64'
    assert_contains "$player_header" \
        'libvlc_media_player_set_metrics_callback'
    assert_contains "$player_header" \
        'libvlc_media_player_set_video_path_callback'

    legacy_exports=$(awk '
        $0 == "libvlc_media_player_set_metrics_callback" { count++ }
        END { print count + 0 }
    ' "$VLC/lib/libvlc.sym")
    video_exports=$(awk '
        $0 == "libvlc_media_player_set_video_path_callback" { count++ }
        END { print count + 0 }
    ' "$VLC/lib/libvlc.sym")
    [ "$legacy_exports" -eq 1 ] && [ "$video_exports" -eq 1 ] ||
        fail "legacy/additive callback exports must each appear exactly once"

    assert_contains "$VLC/modules/access/kymux_video_config.c" \
        'h264_get_chroma_luma'
    assert_contains "$VLC/modules/access/kymux_video_config.c" \
        'hevc_get_chroma_luma'
    assert_contains "$VLC/modules/access/kymux_video_config.c" \
        'AV1_get_chroma'
    assert_contains "$VLC/modules/access/kymux_video_config.c" \
        'LIBVLC_VIDEO_PATH_UNKNOWN_BITSTREAM_CONFIG_UNPARSEABLE'
    assert_contains "$VLC/modules/access/kymux.c" \
        'kymux_video_config_Inspect'
    assert_contains "$VLC/src/input/decoder_helpers.c" \
        'module_get_object'

    assert_contains "$decoder" 'VTSessionCopyProperty'
    assert_contains "$decoder" \
        'kVTDecompressionPropertyKey_UsingHardwareAcceleratedVideoDecoder'
    assert_contains "$decoder" \
        'LIBVLC_VIDEO_PATH_POLICY_HW_ENABLE_REQUESTED'
    assert_contains "$decoder" \
        'LIBVLC_VIDEO_PATH_POLICY_HW_REQUIRE_REQUESTED'
    assert_contains "$decoder" 'CVPixelBufferGetPixelFormatType'
    assert_contains "$decoder" 'CVPixelBufferGetIOSurface'
    assert_contains "$decoder" 'vlc_video_path_ClassifyVTOutcome'
    assert_contains "$renderer" 'vlc_video_path_ClassifyRendererOutcome'
    assert_contains "$renderer" 'readyForMoreMediaData'
    assert_contains "$renderer" 'enqueueSampleBuffer:sampleBuffer'
    assert_contains "$renderer" \
        'LIBVLC_VIDEO_PATH_EVENT_OBSERVATION_LIMIT'
    assert_contains "$renderer" \
        'LIBVLC_VIDEO_PATH_UNKNOWN_PHYSICAL_PRESENTATION_UNAVAILABLE'

    assert_contains "$core_metrics" \
        'LIBVLC_VIDEO_PATH_OUTCOME_RENDERER_SAMPLE_ENQUEUED'
    assert_contains "$core_metrics" \
        'LIBVLC_VIDEO_PATH_OUTCOME_VT_ATTACHMENT_FAILURE'
    assert_contains "$native_test" \
        'test_videotoolbox_outcome_classifier'
    assert_contains "$native_test" 'test_renderer_outcome_classifier'
    assert_contains "$native_test" \
        'CVPixelBufferGetIOSurface(pixel_buffer) != NULL'
    assert_contains "$VLC/test/libvlc/meson.build" \
        "'name' : 'video_path_metrics'"
    assert_contains "$VLC/test/libvlc/meson.build" \
        '? [corefoundation_dep, corevideo_dep]'

    if added_patch_lines "$PATCH_0010" "$PATCH_0011" |
        rg -n '(BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|client-token\.jwt|jwt-private|server-key|password[[:space:]]*=|token[[:space:]]*=)'
    then
        fail "patch additions contain credential or private-key material"
    fi

    if added_patch_lines "$PATCH_0010" "$PATCH_0011" |
        rg -n '(msg_(Err|Warn|Info|Dbg)|printf|fprintf|puts|NSLog).*(payload|packet|event->text|metric.*(key|value)|hostname|clipboard|password|token)'
    then
        fail "native telemetry additions log payload, metric content, or secrets"
    fi

    readme_0010=$(rg -n -F \
        '0010-vlc-video-path-telemetry-abi.patch' "$REPO_ROOT/README.md" |
        head -1 | cut -d: -f1)
    readme_0011=$(rg -n -F \
        '0011-vlc-macos-decoder-telemetry.patch' "$REPO_ROOT/README.md" |
        head -1 | cut -d: -f1)
    [ -n "$readme_0010" ] && [ -n "$readme_0011" ] &&
        [ "$readme_0010" -lt "$readme_0011" ] ||
        fail "README must apply VLC patch 0010 before 0011"

    printf '%s\n' \
        'PASS: exact nested pins and both strict patch path allowlists match' \
        'PASS: legacy scalar ABI remains beside one additive versioned callback export' \
        'PASS: compressed config, selected decoder, live VT property, decoded surface, and renderer facts are wired' \
        'PASS: physical presentation limits and callback-loss provenance are explicit' \
        'PASS: patch additions contain no payload-content or secret logging' \
        'SOURCE=PASS'
}

verify_tests() {
    require_command meson
    require_command nm
    require_command rg
    require_file "$LINUX_BUILD/build.ninja"

    meson compile -C "$LINUX_BUILD" kymux_plugin video_path_metrics
    meson test -C "$LINUX_BUILD" video_path_metrics --print-errorlogs

    test_binary="$LINUX_BUILD/subprojects/vlc/test/video_path_metrics"
    kymux_plugin="$LINUX_BUILD/subprojects/vlc/modules/libkymux_plugin.so"
    libvlc="$LINUX_BUILD/subprojects/vlc/lib/libvlc.so"
    require_file "$test_binary"
    require_file "$kymux_plugin"
    require_file "$libvlc"

    stress_run=1
    while [ "$stress_run" -le 50 ]; do
        "$test_binary" >/dev/null
        stress_run=$((stress_run + 1))
    done

    legacy_symbols=$(nm -D --defined-only "$libvlc" |
        awk '$3 == "libvlc_media_player_set_metrics_callback" { count++ }
             END { print count + 0 }')
    video_symbols=$(nm -D --defined-only "$libvlc" |
        awk '$3 == "libvlc_media_player_set_video_path_callback" { count++ }
             END { print count + 0 }')
    [ "$legacy_symbols" -eq 1 ] && [ "$video_symbols" -eq 1 ] ||
        fail "built libVLC does not export each callback setter exactly once"

    for parser_symbol in \
        AV1_OBU_parse_sequence_header \
        h264_decode_sps \
        hevc_decode_sps \
        kymux_video_config_Inspect
    do
        nm "$kymux_plugin" |
            awk -v symbol="$parser_symbol" '
                $2 ~ /^[tT]$/ && $3 == symbol { found = 1 }
                END { exit found ? 0 : 1 }
            ' ||
            fail "production Kymux plugin did not link parser: $parser_symbol"
    done

    if nm -D "$kymux_plugin" |
        awk '
            $(NF - 1) == "U" &&
            $NF ~ /^(AV1_OBU_parse_sequence_header|h264_decode_sps|hevc_decode_sps|kymux_video_config_Inspect)$/ {
                found = 1
            }
            END { exit found ? 0 : 1 }
        '
    then
        fail "production Kymux plugin retains an unresolved config parser"
    fi

    (
        cd "$VLC"
        git diff --check -- \
            include/vlc/libvlc_media_player.h \
            include/vlc/libvlc_metrics.h \
            include/vlc_metrics.h \
            include/vlc_player.h \
            lib/libvlc.sym \
            lib/media_player.c \
            modules/access/kymux.c \
            modules/access/meson.build \
            modules/codec/videotoolbox/decoder.c \
            modules/video_output/apple/VLCSampleBufferDisplay.m \
            src/input/decoder_helpers.c \
            src/misc/metrics.c \
            src/misc/metrics.h \
            src/player/player.c \
            test/libvlc/meson.build
    )

    printf '%s\n' \
        'PASS: registered cross-platform video_path_metrics test completed' \
        'PASS: 50 direct stress repetitions retained exact callback-loss assertions' \
        'PASS: production Kymux plugin owns all H.264/HEVC/AV1 config parser symbols' \
        'PASS: built libVLC exports legacy and additive callback setters exactly once' \
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

cleanup_local_scratch() {
    cleanup_path=$1
    cleanup_prefix=$2
    case "$cleanup_path" in
        "${TMPDIR:-/tmp}"/"$cleanup_prefix".*)
            [ ! -e "$cleanup_path" ] ||
                find "$cleanup_path" -depth -delete
            ;;
        *)
            printf 'REFUSING_UNSAFE_CLEANUP=%s\n' "$cleanup_path" >&2
            ;;
    esac
}

verify_replay() {
    require_command cmp
    require_command git
    assert_nested_heads
    assert_patch_paths "$PATCH_0010" "$EXPECTED_0010_PATHS"
    assert_patch_paths "$PATCH_0011" "$EXPECTED_0011_PATHS"

    replay_scratch=$(mktemp -d \
        "${TMPDIR:-/tmp}/replaydesktop-macos-native-telemetry-replay.XXXXXX")
    cleanup_replay() {
        cleanup_local_scratch "$replay_scratch" \
            replaydesktop-macos-native-telemetry-replay
    }
    trap cleanup_replay EXIT HUP INT TERM

    replay_vlc="$replay_scratch/vlc"
    clone_at_pin "$VLC" "$replay_vlc" "$EXPECTED_VLC"

    git -C "$replay_vlc" apply --check --whitespace=error-all "$PATCH_0010"
    git -C "$replay_vlc" apply "$PATCH_0010"
    replay_0010_paths=$(git -C "$replay_vlc" status --short |
        sed 's/^...//' | sort)
    [ "$replay_0010_paths" = "$EXPECTED_0010_PATHS" ] ||
        fail "0010 replay changed paths outside its allowlist"

    git -C "$replay_vlc" apply --check --whitespace=error-all "$PATCH_0011"
    git -C "$replay_vlc" apply "$PATCH_0011"
    replay_combined_paths=$(git -C "$replay_vlc" status --short |
        sed 's/^...//' | sort)
    [ "$replay_combined_paths" = "$EXPECTED_COMBINED_PATHS" ] ||
        fail "combined replay changed paths outside its allowlist"

    git -C "$replay_vlc" diff --check
    while IFS= read -r replay_path; do
        cmp "$VLC/$replay_path" "$replay_vlc/$replay_path" ||
            fail "replayed file differs from working source: $replay_path"
    done <<<"$EXPECTED_COMBINED_PATHS"

    git -C "$replay_vlc" apply --check --reverse "$PATCH_0011"
    git -C "$replay_vlc" apply --reverse "$PATCH_0011"
    git -C "$replay_vlc" apply --check --reverse "$PATCH_0010"
    git -C "$replay_vlc" apply --reverse "$PATCH_0010"
    [ -z "$(git -C "$replay_vlc" status --porcelain)" ] ||
        fail "reverse replay did not restore the exact clean VLC pin"
    [ "$(git -C "$replay_vlc" rev-parse HEAD)" = "$EXPECTED_VLC" ] ||
        fail "patch replay moved the temporary VLC HEAD"

    cleanup_replay
    trap - EXIT HUP INT TERM

    printf '%s\n' \
        'PASS: 0010 then 0011 apply from the exact VLC pin with strict path boundaries' \
        'PASS: replayed source matches the working nested source byte-for-byte' \
        'PASS: reverse replay restores a clean exact-pin tree without moving HEAD' \
        'REPLAY=PASS'
}

remote_human_needed() {
    printf 'HUMAN_NEEDED: %s\n' "$*" >&2
    exit 3
}

remote_fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

remote_select_tool() {
    tool_name=$1
    tool_version=$2
    shift 2
    SELECTED_TOOL=
    for tool_candidate in "$@"; do
        [ -n "$tool_candidate" ] && [ -x "$tool_candidate" ] || continue
        candidate_version=$("$tool_candidate" --version 2>/dev/null |
            sed -n '1p')
        if [ "$candidate_version" = "$tool_version" ]; then
            SELECTED_TOOL=$tool_candidate
            return 0
        fi
    done
    remote_human_needed \
        "pinned $tool_name $tool_version is unavailable on the remote Mac"
}

remote_app_snapshot() {
    snapshot_found=false
    snapshot_material=
    for snapshot_app in \
        "$HOME/Applications/ReplayDesktop.app" \
        /Applications/ReplayDesktop.app
    do
        [ -d "$snapshot_app" ] || continue
        snapshot_found=true
        snapshot_material=$(
            {
                stat -f '%N:%m:%z:%Sp' "$snapshot_app"
                find "$snapshot_app" -type f -exec shasum -a 256 {} \;
            } | LC_ALL=C sort | shasum -a 256 | awk '{print $1}'
        )
        printf '%s:%s\n' "$snapshot_app" "$snapshot_material"
    done
    [ "$snapshot_found" = true ] || printf 'ABSENT\n'
}

remote_run() {
    [ "$#" -eq 1 ] || remote_fail "internal remote runner requires scratch path"
    remote_scratch=$1
    case "$remote_scratch" in
        /tmp/replaydesktop-macos-native-telemetry.*) ;;
        *) remote_fail "unsafe remote scratch path: $remote_scratch" ;;
    esac
    [ -d "$remote_scratch" ] ||
        remote_fail "remote scratch directory does not exist"
    [ "$(stat -f '%Lp' "$remote_scratch")" = 700 ] ||
        remote_fail "remote scratch directory is not mode 0700"

    remote_patch_0010="$remote_scratch/0010-vlc-video-path-telemetry-abi.patch"
    remote_patch_0011="$remote_scratch/0011-vlc-macos-decoder-telemetry.patch"
    [ -f "$remote_patch_0010" ] && [ -f "$remote_patch_0011" ] ||
        remote_fail "remote patch inputs are incomplete"

    remote_arch=$(uname -m)
    [ "$remote_arch" = arm64 ] ||
        remote_human_needed "remote architecture is $remote_arch, expected arm64"
    remote_macos=$(sw_vers -productVersion)
    remote_macos_major=${remote_macos%%.*}
    case "$remote_macos_major" in
        ''|*[!0-9]*)
            remote_human_needed "could not parse remote macOS version: $remote_macos"
            ;;
    esac
    [ "$remote_macos_major" -ge 15 ] ||
        remote_human_needed "macOS $remote_macos is older than 15"
    command -v xcodebuild >/dev/null 2>&1 ||
        remote_human_needed "xcodebuild is unavailable"
    command -v xcrun >/dev/null 2>&1 ||
        remote_human_needed "xcrun is unavailable"

    remote_xcode=$(xcodebuild -version | sed -n '1s/^Xcode //p')
    remote_xcode_build=$(xcodebuild -version |
        sed -n '2s/^Build version //p')
    remote_sdk=$(xcrun --sdk macosx --show-sdk-version)
    remote_sdk_path=$(xcrun --sdk macosx --show-sdk-path)
    [ -n "$remote_xcode" ] && [ -n "$remote_xcode_build" ] &&
        [ -n "$remote_sdk" ] && [ -d "$remote_sdk_path" ] ||
        remote_human_needed "Xcode or SDK metadata is incomplete"

    meson_on_path=$(command -v meson 2>/dev/null || true)
    ninja_on_path=$(command -v ninja 2>/dev/null || true)
    remote_select_tool Meson "$EXPECTED_REMOTE_MESON" \
        "$meson_on_path" \
        /opt/homebrew/bin/meson \
        /usr/local/bin/meson
    remote_meson_source=$SELECTED_TOOL
    remote_select_tool Ninja "$EXPECTED_REMOTE_NINJA" \
        "$ninja_on_path" \
        /opt/homebrew/bin/ninja \
        /usr/local/bin/ninja \
        "$HOME/Library/Android/sdk/cmake/3.22.1/bin/ninja"
    remote_ninja_source=$SELECTED_TOOL

    remote_tools="$remote_scratch/tools"
    mkdir -m 700 "$remote_tools"
    cp -L "$remote_meson_source" "$remote_tools/meson"
    cp -L "$remote_ninja_source" "$remote_tools/ninja"
    chmod 700 "$remote_tools/meson" "$remote_tools/ninja"
    [ "$("$remote_tools/meson" --version)" = "$EXPECTED_REMOTE_MESON" ] ||
        remote_fail "scratch Meson bootstrap changed version"
    [ "$("$remote_tools/ninja" --version)" = "$EXPECTED_REMOTE_NINJA" ] ||
        remote_fail "scratch Ninja bootstrap changed version"

    remote_kymedia_source=
    for source_candidate in \
        "$HOME/Developer/replaydesktop-kyber-0.27.0/kysdk/kymedia" \
        "$HOME/Developer/replaydesktop-clipboard-v0.3.0-spike.1-build1/kysdk/kymedia"
    do
        [ -f "$source_candidate/builddir-macos/build.ninja" ] || continue
        [ -d "$source_candidate/subprojects/vlc" ] || continue
        source_vlc_head=$(git -C "$source_candidate/subprojects/vlc" \
            rev-parse HEAD 2>/dev/null || true)
        [ "$source_vlc_head" = "$EXPECTED_VLC" ] || continue
        [ -z "$(git -C "$source_candidate/subprojects/vlc" status --porcelain)" ] ||
            continue
        if ! grep -F -q \
            "\"full\": \"$EXPECTED_REMOTE_MESON\"" \
            "$source_candidate/builddir-macos/meson-info/meson-info.json"
        then
            continue
        fi
        remote_kymedia_source=$source_candidate
        break
    done
    [ -n "$remote_kymedia_source" ] ||
        remote_human_needed \
            "no clean exact-pin Kymedia source with a Meson $EXPECTED_REMOTE_MESON macOS build tree was found"

    remote_app_before=$(remote_app_snapshot)
    remote_input="$remote_scratch/build-input"
    mkdir -m 700 "$remote_input"
    cp -R "$remote_kymedia_source" "$remote_input/kymedia"
    remote_kymedia="$remote_input/kymedia"
    remote_vlc="$remote_kymedia/subprojects/vlc"
    remote_build="$remote_kymedia/builddir-macos"

    (
        cd "$remote_vlc"
        /usr/bin/patch --dry-run -s -p1 <"$remote_patch_0010"
        /usr/bin/patch -s -p1 <"$remote_patch_0010"
        /usr/bin/patch --dry-run -s -p1 <"$remote_patch_0011"
        /usr/bin/patch -s -p1 <"$remote_patch_0011"
    )

    # The copied tree is an already generated exact-pin native build. Touching
    # build.ninja prevents regeneration through an absolute Homebrew Meson
    # path while every changed production source still rebuilds normally.
    touch "$remote_build/build.ninja"
    export MACOSX_DEPLOYMENT_TARGET=15.0
    remote_build_log="$remote_scratch/native-build.log"
    if ! "$remote_tools/ninja" -C "$remote_build" \
        subprojects/vlc/modules/libvideotoolbox_plugin.dylib \
        subprojects/vlc/modules/libsamplebufferdisplay_plugin.dylib \
        subprojects/vlc/lib/libvlc.dylib \
        >"$remote_build_log" 2>&1
    then
        tail -120 "$remote_build_log" >&2
        remote_fail "native Apple production targets did not build"
    fi

    remote_cc=$(xcrun --find clang)
    remote_compile_args=(
        -Isubprojects/vlc
        -I../subprojects/vlc
        -Isubprojects/vlc/include
        -I../subprojects/vlc/include
        -I../subprojects/vlc/compat/stdbit
        -isysroot
        "$remote_sdk_path"
        -DHAVE_CONFIG_H=1
        -UNDEBUG
        -std=gnu17
        -O2
        -Wall
        -Wextra
    )
    (
        cd "$remote_build"
        "$remote_cc" "${remote_compile_args[@]}" \
            -c ../subprojects/vlc/test/libvlc/video_path_metrics.c \
            -o "$remote_scratch/video_path_metrics.o"
        "$remote_cc" "${remote_compile_args[@]}" \
            -c ../subprojects/vlc/modules/access/kymux_video_config.c \
            -o "$remote_scratch/kymux_video_config.o"
        "$remote_cc" "${remote_compile_args[@]}" \
            -c ../subprojects/vlc/modules/packetizer/av1_obu.c \
            -o "$remote_scratch/av1_obu.o"
        "$remote_cc" \
            -isysroot "$remote_sdk_path" \
            -o "$remote_scratch/video_path_metrics" \
            "$remote_scratch/video_path_metrics.o" \
            "$remote_scratch/kymux_video_config.o" \
            "$remote_scratch/av1_obu.o" \
            subprojects/vlc/modules/libhxxxhelper.a \
            subprojects/vlc/lib/libvlc.dylib \
            subprojects/vlc/src/libvlccore.9.dylib \
            subprojects/vlc/compat/libcompat.a \
            -framework CoreFoundation \
            -framework CoreVideo \
            -lm
    )

    remote_test_run=1
    while [ "$remote_test_run" -le 10 ]; do
        DYLD_LIBRARY_PATH="$remote_build/subprojects/vlc/lib:$remote_build/subprojects/vlc/src" \
            "$remote_scratch/video_path_metrics"
        remote_test_run=$((remote_test_run + 1))
    done

    vt_plugin="$remote_build/subprojects/vlc/modules/libvideotoolbox_plugin.dylib"
    renderer_plugin="$remote_build/subprojects/vlc/modules/libsamplebufferdisplay_plugin.dylib"
    remote_libvlc="$remote_build/subprojects/vlc/lib/libvlc.dylib"
    file "$vt_plugin" | grep -q 'arm64' ||
        remote_fail "VideoToolbox plugin is not arm64"
    file "$renderer_plugin" | grep -q 'arm64' ||
        remote_fail "sample-buffer renderer plugin is not arm64"
    xcrun vtool -show-build "$vt_plugin" |
        grep -Eq 'minos[[:space:]]+15\.0' ||
        remote_fail "VideoToolbox plugin does not target macOS 15.0"

    remote_legacy_symbols=$(nm -gU "$remote_libvlc" |
        awk '$NF == "_libvlc_media_player_set_metrics_callback" { count++ }
             END { print count + 0 }')
    remote_video_symbols=$(nm -gU "$remote_libvlc" |
        awk '$NF == "_libvlc_media_player_set_video_path_callback" { count++ }
             END { print count + 0 }')
    [ "$remote_legacy_symbols" -eq 1 ] &&
        [ "$remote_video_symbols" -eq 1 ] ||
        remote_fail "remote libVLC callback exports are incomplete"

    remote_app_after=$(remote_app_snapshot)
    [ "$remote_app_after" = "$remote_app_before" ] ||
        remote_fail "installed ReplayDesktop.app state changed during smoke"

    patch_0010_sha=$(shasum -a 256 "$remote_patch_0010" |
        awk '{print $1}')
    patch_0011_sha=$(shasum -a 256 "$remote_patch_0011" |
        awk '{print $1}')
    warning_count=$(grep -c 'warning:' "$remote_build_log" || true)
    if [ "$remote_xcode" = 26.6 ]; then
        remote_qualification=RELEASE_LANE_CANDIDATE_XCODE_26_6
    else
        xcode_token=$(printf '%s' "$remote_xcode" |
            tr '.-' '__' | tr '[:lower:]' '[:upper:]')
        remote_qualification="COMPATIBILITY_ONLY_XCODE_$xcode_token"
    fi

    printf '%s\n' \
        'MANIFEST_SCHEMA=replaydesktop.macos-native-telemetry-smoke.v1' \
        "MANIFEST_ARCH=$remote_arch" \
        "MANIFEST_MACOS=$remote_macos" \
        "MANIFEST_XCODE=$remote_xcode" \
        "MANIFEST_XCODE_BUILD=$remote_xcode_build" \
        "MANIFEST_SDK=$remote_sdk" \
        'MANIFEST_DEPLOYMENT_TARGET=15.0' \
        "MANIFEST_MESON=$EXPECTED_REMOTE_MESON" \
        "MANIFEST_NINJA=$EXPECTED_REMOTE_NINJA" \
        "MANIFEST_VLC_PIN=$EXPECTED_VLC" \
        "MANIFEST_PATCH_0010_SHA256=$patch_0010_sha" \
        "MANIFEST_PATCH_0011_SHA256=$patch_0011_sha" \
        "MANIFEST_BUILD_WARNINGS=$warning_count" \
        'MANIFEST_NATIVE_TEST_RUNS=10' \
        'MANIFEST_APP_ACTION=NONE' \
        "MANIFEST_QUALIFICATION=$remote_qualification" \
        'MAC_SMOKE=PASS'
}

validate_remote_target() {
    case "$1" in
        *[!A-Za-z0-9_.@:-]*|'')
            fail "remote target contains unsupported characters: $1"
            ;;
    esac
}

verify_mac_smoke() {
    [ "$#" -eq 1 ] || fail "--mac-smoke requires exactly one user@host target"
    remote_target=$1
    validate_remote_target "$remote_target"
    require_command scp
    require_command ssh
    require_file "$SCRIPT_PATH"
    require_file "$PATCH_0010"
    require_file "$PATCH_0011"

    verify_source
    verify_replay

    remote_scratch=$(ssh -o BatchMode=yes -o ConnectTimeout=15 \
        "$remote_target" \
        'umask 077; scratch=$(mktemp -d /tmp/replaydesktop-macos-native-telemetry.XXXXXX) && chmod 700 "$scratch" && printf "%s\n" "$scratch"')
    case "$remote_scratch" in
        /tmp/replaydesktop-macos-native-telemetry.*) ;;
        *) fail "remote returned an unsafe scratch path: $remote_scratch" ;;
    esac

    cleanup_remote() {
        case "$remote_scratch" in
            /tmp/replaydesktop-macos-native-telemetry.*)
                ssh -o BatchMode=yes "$remote_target" \
                    "test -d '$remote_scratch' && find '$remote_scratch' -depth -delete || true" \
                    >/dev/null 2>&1 || true
                ;;
        esac
    }
    trap cleanup_remote EXIT HUP INT TERM

    scp -q \
        "$SCRIPT_PATH" \
        "$PATCH_0010" \
        "$PATCH_0011" \
        "$remote_target:$remote_scratch/"

    set +e
    ssh -o BatchMode=yes "$remote_target" \
        "bash '$remote_scratch/verify-macos-native-telemetry.sh' --remote-run '$remote_scratch'"
    remote_status=$?
    set -e

    cleanup_remote
    trap - EXIT HUP INT TERM
    if [ "$remote_status" -eq 3 ]; then
        printf '%s\n' \
            'HUMAN_NEEDED: remote prerequisites could not be created in scratch' >&2
        return 3
    fi
    [ "$remote_status" -eq 0 ] ||
        fail "remote native telemetry smoke failed with status $remote_status"
}

if [ "${1:-}" = --remote-run ]; then
    [ "$#" -eq 2 ] || {
        printf 'FAIL: invalid internal remote-run invocation\n' >&2
        exit 1
    }
    remote_run "$2"
    exit 0
fi

case "${1:-}" in
    --source)
        [ "$#" -eq 1 ] || fail "--source takes no additional arguments"
        verify_source
        ;;
    --tests)
        [ "$#" -eq 1 ] || fail "--tests takes no additional arguments"
        verify_tests
        ;;
    --replay)
        [ "$#" -eq 1 ] || fail "--replay takes no additional arguments"
        verify_replay
        ;;
    --mac-smoke)
        shift
        verify_mac_smoke "$@"
        ;;
    -h|--help)
        usage
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac
