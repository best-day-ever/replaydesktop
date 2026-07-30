#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
APP_PATH="$PWD/ReplayDesktop.app"
ARCHIVE_PATH="$PWD/ReplayDesktop-arm64-gui-spike.zip"
SOURCE_BOUNDARY=false

usage() {
    printf '%s\n' \
        "Usage:" \
        "  $0 [--app PATH] [--archive PATH]" \
        "  $0 --source-boundary" \
        "" \
        "The default mode verifies the signed macOS GUI bundle and archive." \
        "--source-boundary verifies staged parent-repository source and the" \
        "reproducible kynput patch without accepting nested submodule changes."
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --app)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            APP_PATH=$2
            shift 2
            ;;
        --archive)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            ARCHIVE_PATH=$2
            shift 2
            ;;
        --source-boundary)
            SOURCE_BOUNDARY=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            printf 'Unknown argument: %s\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

verify_source_boundary() {
    cd "$REPO_ROOT"
    git rev-parse --show-toplevel >/dev/null 2>&1 ||
        fail 'source-boundary verification requires the parent git repository'

    expected_desktop=6c75cc276e40ed9cca4e9dcabe3f7e4e1d83c44f
    expected_kysdk=5836202aa4654cf64b5ca9fd204005fe4916be99
    expected_kynput=5595478f3606c5624197360e2dcbf31bd60e8163

    desktop_gitlink=$(git ls-tree HEAD upstream/kyber-desktop | awk '{print $3}')
    [ "$desktop_gitlink" = "$expected_desktop" ] ||
        fail "parent Kyber gitlink drifted: $desktop_gitlink"
    [ "$(git -C upstream/kyber-desktop rev-parse HEAD)" = "$expected_desktop" ] ||
        fail 'working Kyber checkout is not at the pinned detached commit'

    kysdk_gitlink=$(git -C upstream/kyber-desktop ls-tree HEAD kysdk | awk '{print $3}')
    [ "$kysdk_gitlink" = "$expected_kysdk" ] ||
        fail "Kyber SDK gitlink drifted: $kysdk_gitlink"
    [ "$(git -C upstream/kyber-desktop/kysdk rev-parse HEAD)" = "$expected_kysdk" ] ||
        fail 'working Kyber SDK checkout is not at the pinned detached commit'

    kynput_gitlink=$(git -C upstream/kyber-desktop/kysdk ls-tree HEAD kynput | awk '{print $3}')
    [ "$kynput_gitlink" = "$expected_kynput" ] ||
        fail "kynput gitlink drifted: $kynput_gitlink"
    [ "$(git -C upstream/kyber-desktop/kysdk/kynput rev-parse HEAD)" = "$expected_kynput" ] ||
        fail 'working kynput checkout is not at the pinned detached commit'

    staged_paths=$(git diff --cached --name-only)
    [ -n "$staged_paths" ] || fail 'no parent-repository source is staged'

    while IFS= read -r path; do
        staged_status=$(git diff --cached --name-status -- "$path" | awk 'NR == 1 {print $1}')
        case "$staged_status" in
            D*) fail "source-boundary commit must not delete files: $path" ;;
        esac

        case "$path" in
            README.md|\
            prototype/macos/*|\
            scripts/package-macos-gui.sh|\
            scripts/verify-gui-spike.sh|\
            patches/kyber/0004-linux-hires-wheel.patch|\
            .planning/quick/260730-j91-build-and-publish-a-macos-technical-gui-/*.md)
                ;;
            upstream/kyber-desktop|upstream/kyber-desktop/*)
                fail "nested Kyber source must never be staged in the parent commit: $path"
                ;;
            *)
                fail "unexpected staged path: $path"
                ;;
        esac

        case "$path" in
            *.pem|*.key|*.p12|*.pfx|*.jwt|*.log|log/*|*/log/*|*/logs/*|\
            *private-config*|*identity-material*)
                fail "private, credential, or runtime material is staged: $path"
                ;;
        esac
    done <<EOF
$staged_paths
EOF

    private_marker='BEGIN PRIVATE'" KEY"
    certificate_marker='BEGIN CERT'"IFICATE"
    test_identity='kyber'"test_"
    jwt_prefix='e''yJ'
    while IFS= read -r path; do
        if git grep --cached -I -E -i -q \
            "$private_marker|$certificate_marker|$test_identity|$jwt_prefix[A-Za-z0-9_-]{10,}\\." \
            -- "$path"
        then
            fail "credential or test identity marker found in staged content: $path"
        fi
    done <<EOF
$staged_paths
EOF

    patch_path="$REPO_ROOT/patches/kyber/0004-linux-hires-wheel.patch"
    [ -f "$patch_path" ] || fail "missing reproducible scroll patch: $patch_path"
    temp_root=${TMPDIR:-/tmp}
    scratch=$(mktemp -d "$temp_root/replaydesktop-source.XXXXXX")
    cleanup_source_boundary() {
        case "$scratch" in
            "$temp_root"/replaydesktop-source.*) rm -rf -- "$scratch" ;;
        esac
    }
    trap cleanup_source_boundary RETURN

    git clone --quiet --no-hardlinks \
        "$REPO_ROOT/upstream/kyber-desktop/kysdk/kynput" \
        "$scratch/kynput"
    git -C "$scratch/kynput" checkout --quiet --detach "$expected_kynput"
    git -C "$scratch/kynput" apply --check "$patch_path"

    printf '%s\n' \
        'PASS: staged paths stay inside the GUI-spike parent source boundary' \
        'PASS: Kyber, Kyber SDK, and kynput gitlinks retain their exact pins' \
        'PASS: no private/test identity material is staged' \
        'PASS: 0004-linux-hires-wheel.patch applies to clean pinned kynput'
}

verify_bundle() {
    [ "$(uname -s)" = "Darwin" ] ||
        fail 'bundle verification requires macOS tools'

    for tool in plutil file otool codesign unzip zipinfo shasum; do
        command -v "$tool" >/dev/null 2>&1 ||
            fail "required macOS verification tool not found: $tool"
    done

    [ -d "$APP_PATH/Contents" ] || fail "app bundle not found: $APP_PATH"
    [ -f "$ARCHIVE_PATH" ] || fail "archive not found: $ARCHIVE_PATH"

    plist="$APP_PATH/Contents/Info.plist"
    launcher="$APP_PATH/Contents/MacOS/ReplayDesktopLauncher"
    raw_client="$APP_PATH/Contents/MacOS/kyclient"
    [ -x "$launcher" ] || fail 'ReplayDesktopLauncher is not executable'
    [ -x "$raw_client" ] || fail 'raw Contents/MacOS/kyclient fallback is missing'

    plutil -lint "$plist"
    executable=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$plist")
    [ "$executable" = ReplayDesktopLauncher ] ||
        fail "CFBundleExecutable is $executable"
    minimum_os=$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$plist")
    [ "$minimum_os" = 15.0 ] || fail "LSMinimumSystemVersion is $minimum_os"

    temp_root=${TMPDIR:-/tmp}
    inventory=$(mktemp "$temp_root/replaydesktop-inventory.XXXXXX")
    runtime_probe=$(mktemp -d "$temp_root/replaydesktop-runtime.XXXXXX")
    archive_probe=$(mktemp -d "$temp_root/replaydesktop-archive.XXXXXX")
    cleanup_bundle() {
        rm -f -- "$inventory"
        case "$runtime_probe" in
            "$temp_root"/replaydesktop-runtime.*) rm -rf -- "$runtime_probe" ;;
        esac
        case "$archive_probe" in
            "$temp_root"/replaydesktop-archive.*) rm -rf -- "$archive_probe" ;;
        esac
    }
    trap cleanup_bundle EXIT HUP INT TERM

    find "$APP_PATH" -type f -print | LC_ALL=C sort >"$inventory"
    macho_count=0
    while IFS= read -r candidate; do
        description=$(file "$candidate")
        case "$description" in
            *Mach-O*)
                macho_count=$((macho_count + 1))
                printf '%s\n' "$description"
                case "$description" in
                    *arm64*) ;;
                    *) fail "non-arm64 Mach-O found: $candidate" ;;
                esac
                case "$description" in
                    *x86_64*|*universal*) fail "non-arm64-only Mach-O found: $candidate" ;;
                esac

                minos=$(otool -l "$candidate" |
                    awk '$1 == "cmd" && $2 == "LC_BUILD_VERSION" { build = 1; next }
                         build && $1 == "minos" { print $2; exit }')
                if [ -n "$minos" ]; then
                    printf '  minos %s: %s\n' "$minos" "$candidate"
                fi
                ;;
        esac
    done <"$inventory"
    [ "$macho_count" -gt 1 ] || fail 'Mach-O inventory did not include launcher and raw client'

    launcher_minos=$(otool -l "$launcher" |
        awk '$1 == "cmd" && $2 == "LC_BUILD_VERSION" { build = 1; next }
             build && $1 == "minos" { print $2; exit }')
    [ "$launcher_minos" = 15.0 ] ||
        fail "launcher deployment target is $launcher_minos, expected 15.0"

    codesign --verify --deep --strict "$APP_PATH"
    codesign -d --verbose=4 "$APP_PATH" 2>&1 |
        grep -F 'Signature=adhoc' >/dev/null ||
        fail 'bundle is not ad-hoc signed'

    "$launcher" --self-test
    runtime_path=$("$launcher" --print-runtime-directory)
    [ -d "$runtime_path" ] || fail "launcher did not create runtime directory: $runtime_path"
    [ -w "$runtime_path" ] || fail "runtime directory is not writable: $runtime_path"
    case "$runtime_path" in
        "$APP_PATH"|"$APP_PATH"/*) fail 'runtime directory resolves inside the app bundle' ;;
    esac

    (
        cd "$runtime_probe"
        "$raw_client" --help >kyclient-help.txt 2>&1
        grep -E 'Streaming client|USAGE|Usage' kyclient-help.txt >/dev/null
    ) || fail 'raw kyclient --help fallback failed'

    forbidden_name=$(find "$APP_PATH" \
        \( -iname '*kybertest*' -o -iname '*.pem' -o -iname '*.key' \
        -o -iname '*.p12' -o -iname '*.pfx' -o -iname '*.jwt' \) \
        -print -quit)
    [ -z "$forbidden_name" ] ||
        fail "test identity or credential-shaped file is bundled: $forbidden_name"

    private_marker='BEGIN PRIVATE'" KEY"
    certificate_marker='BEGIN CERT'"IFICATE"
    pem_match=$(
        grep -IRIl -E "$private_marker|$certificate_marker" "$APP_PATH" |
            sed -n '1p' || true
    )
    [ -z "$pem_match" ] ||
        fail "PEM identity material is embedded in the app bundle: $pem_match"

    unzip -tq "$ARCHIVE_PATH"
    archive_entries=$(zipinfo -1 "$ARCHIVE_PATH")
    [ -n "$archive_entries" ] || fail 'archive is empty'
    while IFS= read -r entry; do
        case "$entry" in
            ReplayDesktop.app|ReplayDesktop.app/*) ;;
            *) fail "archive contains an unexpected path: $entry" ;;
        esac
        case "$entry" in
            /*|../*|*/../*) fail "archive contains path traversal: $entry" ;;
        esac
    done <<EOF
$archive_entries
EOF

    unzip -q "$ARCHIVE_PATH" -d "$archive_probe"
    codesign --verify --deep --strict "$archive_probe/ReplayDesktop.app"
    archived_executable=$(/usr/libexec/PlistBuddy \
        -c 'Print :CFBundleExecutable' \
        "$archive_probe/ReplayDesktop.app/Contents/Info.plist")
    [ "$archived_executable" = ReplayDesktopLauncher ] ||
        fail 'archived bundle metadata does not select ReplayDesktopLauncher'

    checksum=$(shasum -a 256 "$ARCHIVE_PATH" | awk '{print $1}')
    printf '%s\n' \
        "PASS: bundle=$APP_PATH" \
        "PASS: CFBundleExecutable=$executable" \
        "PASS: LSMinimumSystemVersion=$minimum_os" \
        "PASS: arm64 Mach-O count=$macho_count" \
        "PASS: launcher minos=$launcher_minos" \
        'PASS: ad-hoc signature, raw CLI fallback, runtime directory, and archive integrity' \
        "SHA256=$checksum"
}

if [ "$SOURCE_BOUNDARY" = true ]; then
    verify_source_boundary
else
    verify_bundle
fi
