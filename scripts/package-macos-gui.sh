#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
LAUNCHER_SOURCE="$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"
SOURCE_APP=""
OUTPUT_DIR="$PWD"
ARCHIVE_NAME="ReplayDesktop-arm64-gui-spike.zip"
DRY_RUN=false
PACKAGE_TIMESTAMP="${PACKAGE_TIMESTAMP:-202001010000}"

usage() {
    printf '%s\n' \
        "Usage: $0 [--dry-run] [--source-app PATH] [--output-dir PATH]" \
        "" \
        "Builds an arm64 macOS 15 AppKit launcher, injects it into a recoverable" \
        "copy of ReplayDesktop.app, ad-hoc signs the complete internal prototype," \
        "and creates ReplayDesktop-arm64-gui-spike.zip."
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --source-app)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            SOURCE_APP=$2
            shift 2
            ;;
        --output-dir)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            OUTPUT_DIR=$2
            shift 2
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

[ -f "$LAUNCHER_SOURCE" ] || {
    printf 'Launcher source not found: %s\n' "$LAUNCHER_SOURCE" >&2
    exit 1
}

for required_flag in \
    '--port=8080' \
    '--protocol=kymux' \
    '--tls-skip-verification' \
    '--video-buffer=0' \
    '--metrics=true' \
    '--auto-reconnect=false'
do
    grep -F -- "$required_flag" "$LAUNCHER_SOURCE" >/dev/null || {
        printf 'Launcher source is missing fixed baseline flag: %s\n' "$required_flag" >&2
        exit 1
    }
done

case "$PACKAGE_TIMESTAMP" in
    [0-9][0-9][0-9][0-9][0-1][0-9][0-3][0-9][0-2][0-9][0-5][0-9]) ;;
    *)
        printf 'PACKAGE_TIMESTAMP must use touch format YYYYMMDDhhmm: %s\n' "$PACKAGE_TIMESTAMP" >&2
        exit 2
        ;;
esac

if [ "$DRY_RUN" = true ]; then
    printf '%s\n' \
        "ReplayDesktop GUI package dry run" \
        "  launcher: $LAUNCHER_SOURCE" \
        "  compile: xcrun --sdk macosx swiftc -parse-as-library -target arm64-apple-macos15.0 <launcher> -framework AppKit" \
        "  bundle executable: ReplayDesktopLauncher" \
        "  retained CLI fallback: Contents/MacOS/kyclient" \
        "  minimum macOS: 15.0" \
        "  architecture: arm64 only" \
        "  signing: ad-hoc, no notarization (internal LAN/Tailscale prototype only)" \
        "  archive: $ARCHIVE_NAME" \
        "  normalized timestamp: $PACKAGE_TIMESTAMP"
    exit 0
fi

[ "$(uname -s)" = "Darwin" ] || {
    printf 'A macOS builder is required for a real package build.\n' >&2
    exit 1
}

[ -n "$SOURCE_APP" ] || {
    printf -- '--source-app is required for a real package build.\n' >&2
    exit 2
}
[ -d "$SOURCE_APP/Contents" ] || {
    printf 'Invalid source app: %s\n' "$SOURCE_APP" >&2
    exit 1
}
[ -x "$SOURCE_APP/Contents/MacOS/kyclient" ] || {
    printf 'Source app does not retain Contents/MacOS/kyclient: %s\n' "$SOURCE_APP" >&2
    exit 1
}

for tool in xcrun plutil codesign otool file zip unzip shasum ditto; do
    command -v "$tool" >/dev/null 2>&1 || {
        printf 'Required macOS packaging tool not found: %s\n' "$tool" >&2
        exit 1
    }
done

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR=$(CDPATH= cd -- "$OUTPUT_DIR" && pwd)
FINAL_APP="$OUTPUT_DIR/ReplayDesktop.app"
FINAL_ARCHIVE="$OUTPUT_DIR/$ARCHIVE_NAME"

[ ! -e "$FINAL_APP" ] || {
    printf 'Refusing to replace existing output app: %s\n' "$FINAL_APP" >&2
    exit 1
}
[ ! -e "$FINAL_ARCHIVE" ] || {
    printf 'Refusing to replace existing archive: %s\n' "$FINAL_ARCHIVE" >&2
    exit 1
}

STAGING_DIR=$(mktemp -d "$OUTPUT_DIR/.replaydesktop-gui.XXXXXX")
cleanup() {
    case "$STAGING_DIR" in
        "$OUTPUT_DIR"/.replaydesktop-gui.*)
            rm -rf -- "$STAGING_DIR"
            ;;
    esac
}
trap cleanup EXIT HUP INT TERM

STAGED_APP="$STAGING_DIR/ReplayDesktop.app"
STAGED_ARCHIVE="$STAGING_DIR/$ARCHIVE_NAME"
ditto "$SOURCE_APP" "$STAGED_APP"

xcrun --sdk macosx swiftc \
    -parse-as-library \
    -target arm64-apple-macos15.0 \
    "$LAUNCHER_SOURCE" \
    -framework AppKit \
    -o "$STAGED_APP/Contents/MacOS/ReplayDesktopLauncher"

PLIST="$STAGED_APP/Contents/Info.plist"
plutil -lint "$PLIST"
plutil -replace CFBundleExecutable -string ReplayDesktopLauncher "$PLIST"
plutil -replace CFBundleName -string ReplayDesktop "$PLIST"
plutil -replace CFBundleDisplayName -string ReplayDesktop "$PLIST" 2>/dev/null ||
    plutil -insert CFBundleDisplayName -string ReplayDesktop "$PLIST"
plutil -replace CFBundleShortVersionString -string 0.2.0 "$PLIST"
plutil -replace CFBundleVersion -string 20001 "$PLIST"
plutil -replace LSMinimumSystemVersion -string 15.0 "$PLIST"
plutil -replace NSHighResolutionCapable -bool true "$PLIST" 2>/dev/null ||
    plutil -insert NSHighResolutionCapable -bool true "$PLIST"
plutil -remove CFBundleGetInfoString "$PLIST" 2>/dev/null || true
plutil -insert CFBundleGetInfoString \
    -string 'ReplayDesktop v0.2.0-spike.1 internal prototype; ad-hoc signed and not notarized' \
    "$PLIST"
plutil -lint "$PLIST"

"$STAGED_APP/Contents/MacOS/ReplayDesktopLauncher" --self-test

find "$STAGED_APP" -type f -print | LC_ALL=C sort | while IFS= read -r candidate; do
    if file "$candidate" | grep -q 'Mach-O'; then
        codesign --force --sign - --timestamp=none "$candidate"
    fi
done
codesign --force --sign - --timestamp=none "$STAGED_APP"
codesign --verify --deep --strict "$STAGED_APP"

find "$STAGED_APP" -exec touch -h -t "$PACKAGE_TIMESTAMP" {} +
(
    cd "$STAGING_DIR"
    find ReplayDesktop.app -print | LC_ALL=C sort |
        zip -X -y -q "$STAGED_ARCHIVE" -@
)
unzip -tq "$STAGED_ARCHIVE"

mv "$STAGED_APP" "$FINAL_APP"
mv "$STAGED_ARCHIVE" "$FINAL_ARCHIVE"

printf 'APP=%s\n' "$FINAL_APP"
printf 'ARCHIVE=%s\n' "$FINAL_ARCHIVE"
printf 'SHA256='
shasum -a 256 "$FINAL_ARCHIVE" | awk '{print $1}'
