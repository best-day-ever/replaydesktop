#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
LAUNCHER_SOURCE="$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"
ENGINE_LOCK="$REPO_ROOT/prototype/macos/engine-baseline.lock"
SOURCE_APP=""
ENGINE_SOURCE_ROOT=""
OUTPUT_DIR="$PWD"
ARCHIVE_NAME="ReplayDesktop-arm64-clipboard-spike.zip"
DRY_RUN=false
PACKAGE_TIMESTAMP="${PACKAGE_TIMESTAMP:-202001010000}"
PACKAGE_VERSION=""
SOURCE_BASE=""
SOURCE_COMMIT=""
SOURCE_BOUNDARY_DIGEST=""
DEPLOYMENT_TARGET=15.0

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

lock_value() {
    lock_key=$1
    lock_result=$(sed -n "s/^${lock_key}=//p" "$ENGINE_LOCK")
    [ -n "$lock_result" ] || fail "engine baseline lock is missing $lock_key"
    [ "$(printf '%s\n' "$lock_result" | wc -l | tr -d ' ')" = 1 ] ||
        fail "engine baseline lock repeats $lock_key"
    printf '%s\n' "$lock_result"
}

code_signature_offset() {
    otool -l "$1" |
        awk '$1 == "cmd" && $2 == "LC_CODE_SIGNATURE" {
                 signature = 1
                 next
             }
             signature && $1 == "dataoff" {
                 print $2
                 exit
             }'
}

usage() {
    printf '%s\n' \
        "Usage: $0 [--dry-run] --package-version VERSION" \
        "          --source-base COMMIT --source-commit COMMIT" \
        "          --source-boundary-digest SHA256" \
        "          [--source-app PATH --engine-source-root PATH] [--output-dir PATH]" \
        "" \
        "Builds an arm64 macOS 15 AppKit launcher, injects it into a recoverable" \
        "copy of ReplayDesktop.app, ad-hoc signs the complete internal prototype," \
        "and creates ReplayDesktop-arm64-clipboard-spike.zip."
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
        --engine-source-root)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            ENGINE_SOURCE_ROOT=$2
            shift 2
            ;;
        --output-dir)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            OUTPUT_DIR=$2
            shift 2
            ;;
        --package-version)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            PACKAGE_VERSION=$2
            shift 2
            ;;
        --source-base)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            SOURCE_BASE=$2
            shift 2
            ;;
        --source-commit)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            SOURCE_COMMIT=$2
            shift 2
            ;;
        --source-boundary-digest)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            SOURCE_BOUNDARY_DIGEST=$2
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
[ -f "$ENGINE_LOCK" ] || fail "engine baseline lock not found: $ENGINE_LOCK"

printf '%s\n' "$PACKAGE_VERSION" |
    grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+-spike\.[0-9]+$' ||
    fail 'explicit --package-version must use vMAJOR.MINOR.PATCH-spike.N'
printf '%s\n' "$SOURCE_BASE" | grep -Eq '^[0-9a-f]{40}$' ||
    fail 'explicit --source-base must be a full lowercase Git commit'
printf '%s\n' "$SOURCE_COMMIT" | grep -Eq '^[0-9a-f]{40}$' ||
    fail 'explicit --source-commit must be a full lowercase Git commit'
[ "$SOURCE_BASE" != "$SOURCE_COMMIT" ] ||
    fail 'source base and source commit must differ'
printf '%s\n' "$SOURCE_BOUNDARY_DIGEST" | grep -Eq '^[0-9a-f]{64}$' ||
    fail 'explicit --source-boundary-digest must be a lowercase SHA-256'

SHORT_VERSION=$(printf '%s\n' "$PACKAGE_VERSION" |
    sed -E 's/^v([0-9]+\.[0-9]+\.[0-9]+)-spike\.[0-9]+$/\1/')
SPIKE_NUMBER=$(printf '%s\n' "$PACKAGE_VERSION" |
    sed -E 's/^v[0-9]+\.[0-9]+\.[0-9]+-spike\.([0-9]+)$/\1/')
VERSION_MAJOR=$(printf '%s\n' "$SHORT_VERSION" | awk -F. '{print $1}')
VERSION_MINOR=$(printf '%s\n' "$SHORT_VERSION" | awk -F. '{print $2}')
VERSION_PATCH=$(printf '%s\n' "$SHORT_VERSION" | awk -F. '{print $3}')
BUNDLE_VERSION=$(
    printf '%s\n' "$((VERSION_MAJOR * 1000000 + VERSION_MINOR * 10000 + VERSION_PATCH * 100 + SPIKE_NUMBER))"
)

LOCK_FORMAT=$(lock_value format)
LOCK_KYBER_RELEASE=$(lock_value kyber_release)
LOCK_KYBER_COMMIT=$(lock_value kyber_commit)
LOCK_KYSDK_COMMIT=$(lock_value kysdk_commit)
LOCK_RAW_PATH=$(lock_value raw_kyclient_bundle_path)
LOCK_RAW_SHA256=$(lock_value raw_kyclient_sha256)
LOCK_RAW_PAYLOAD_SHA256=$(lock_value raw_kyclient_payload_sha256)
LOCK_RAW_CODE_SIGNATURE_OFFSET=$(lock_value raw_kyclient_code_signature_offset)
LOCK_RAW_MINIMUM_MACOS=$(lock_value raw_kyclient_minimum_macos)
[ "$LOCK_FORMAT" = 1 ] || fail "unsupported engine baseline lock format: $LOCK_FORMAT"
printf '%s\n' "$LOCK_KYBER_COMMIT" | grep -Eq '^[0-9a-f]{40}$' ||
    fail 'engine baseline lock contains an invalid Kyber commit'
printf '%s\n' "$LOCK_KYSDK_COMMIT" | grep -Eq '^[0-9a-f]{40}$' ||
    fail 'engine baseline lock contains an invalid source commit'
printf '%s\n' "$LOCK_RAW_SHA256" | grep -Eq '^[0-9a-f]{64}$' ||
    fail 'engine baseline lock contains an invalid raw kyclient digest'
printf '%s\n' "$LOCK_RAW_PAYLOAD_SHA256" | grep -Eq '^[0-9a-f]{64}$' ||
    fail 'engine baseline lock contains an invalid raw kyclient payload digest'
printf '%s\n' "$LOCK_RAW_CODE_SIGNATURE_OFFSET" | grep -Eq '^[1-9][0-9]*$' ||
    fail 'engine baseline lock contains an invalid code-signature offset'
[ "$LOCK_RAW_PATH" = Contents/MacOS/kyclient ] ||
    fail "unsupported raw engine bundle path: $LOCK_RAW_PATH"
[ "$LOCK_RAW_MINIMUM_MACOS" = "$DEPLOYMENT_TARGET" ] ||
    fail 'engine baseline and launcher deployment targets differ'

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
        "  package version: $PACKAGE_VERSION (bundle $BUNDLE_VERSION)" \
        "  source range: $SOURCE_BASE..$SOURCE_COMMIT" \
        "  source-boundary SHA256: $SOURCE_BOUNDARY_DIGEST" \
        "  engine: Kyber $LOCK_KYBER_RELEASE @ $LOCK_KYBER_COMMIT" \
        "  raw kyclient input SHA256: $LOCK_RAW_SHA256" \
        "  raw kyclient payload SHA256: $LOCK_RAW_PAYLOAD_SHA256" \
        "  launcher: $LAUNCHER_SOURCE" \
        "  compile: xcrun --sdk macosx swiftc -parse-as-library -target arm64-apple-macos${DEPLOYMENT_TARGET} <launcher> -framework AppKit" \
        "  bundle executable: ReplayDesktopLauncher" \
        "  retained CLI fallback: Contents/MacOS/kyclient" \
        "  minimum macOS: $DEPLOYMENT_TARGET" \
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
[ -n "$ENGINE_SOURCE_ROOT" ] || {
    printf -- '--engine-source-root is required for a real package build.\n' >&2
    exit 2
}
[ -d "$SOURCE_APP/Contents" ] || {
    printf 'Invalid source app: %s\n' "$SOURCE_APP" >&2
    exit 1
}
[ -x "$SOURCE_APP/$LOCK_RAW_PATH" ] || {
    printf 'Source app does not retain %s: %s\n' "$LOCK_RAW_PATH" "$SOURCE_APP" >&2
    exit 1
}
git -C "$ENGINE_SOURCE_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1 ||
    fail "engine source root is not a Git checkout: $ENGINE_SOURCE_ROOT"
git -C "$REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1 ||
    fail "packaging source is not a committed Git checkout: $REPO_ROOT"

for tool in xcrun xcodebuild plutil codesign otool file zip unzip shasum ditto git; do
    command -v "$tool" >/dev/null 2>&1 || {
        printf 'Required macOS packaging tool not found: %s\n' "$tool" >&2
        exit 1
    }
done

SOURCE_APP=$(CDPATH= cd -- "$SOURCE_APP" && pwd)
ENGINE_SOURCE_ROOT=$(CDPATH= cd -- "$ENGINE_SOURCE_ROOT" && pwd)
case "$SOURCE_APP" in
    "$ENGINE_SOURCE_ROOT"/*) ;;
    *) fail "source app is outside engine source root: $SOURCE_APP" ;;
esac

ACTUAL_KYBER_COMMIT=$(git -C "$ENGINE_SOURCE_ROOT" rev-parse HEAD)
[ "$ACTUAL_KYBER_COMMIT" = "$LOCK_KYBER_COMMIT" ] ||
    fail "engine source commit mismatch: $ACTUAL_KYBER_COMMIT"
ACTUAL_KYSDK_COMMIT=$(git -C "$ENGINE_SOURCE_ROOT" ls-tree HEAD kysdk | awk '{print $3}')
[ "$ACTUAL_KYSDK_COMMIT" = "$LOCK_KYSDK_COMMIT" ] ||
    fail "engine Kysdk gitlink mismatch: $ACTUAL_KYSDK_COMMIT"

ACTUAL_SOURCE_COMMIT=$(git -C "$REPO_ROOT" rev-parse HEAD)
[ "$ACTUAL_SOURCE_COMMIT" = "$SOURCE_COMMIT" ] ||
    fail "packaging checkout does not match source commit: $ACTUAL_SOURCE_COMMIT"
git -C "$REPO_ROOT" cat-file -e "${SOURCE_BASE}^{commit}" ||
    fail "source base is unavailable in packaging checkout: $SOURCE_BASE"
git -C "$REPO_ROOT" merge-base --is-ancestor "$SOURCE_BASE" "$SOURCE_COMMIT" ||
    fail 'source base is not an ancestor of source commit'
ACTUAL_SOURCE_BOUNDARY_DIGEST=$(
    git -C "$REPO_ROOT" diff \
        --no-ext-diff --no-color --binary --full-index \
        "$SOURCE_BASE" "$SOURCE_COMMIT" |
        shasum -a 256 |
        awk '{print $1}'
)
[ "$ACTUAL_SOURCE_BOUNDARY_DIGEST" = "$SOURCE_BOUNDARY_DIGEST" ] ||
    fail "source-boundary digest mismatch: $ACTUAL_SOURCE_BOUNDARY_DIGEST"
git -C "$REPO_ROOT" diff --quiet "$SOURCE_COMMIT" -- \
    README.md \
    prototype/macos/ReplayDesktopLauncher.swift \
    prototype/macos/engine-baseline.lock \
    scripts/package-macos-gui.sh \
    scripts/verify-gui-spike.sh ||
    fail 'packaging inputs differ from the committed source'
git -C "$REPO_ROOT" diff --cached --quiet -- \
    README.md \
    prototype/macos/ReplayDesktopLauncher.swift \
    prototype/macos/engine-baseline.lock \
    scripts/package-macos-gui.sh \
    scripts/verify-gui-spike.sh ||
    fail 'packaging inputs have staged changes outside the source commit'

SOURCE_RAW_CLIENT="$SOURCE_APP/$LOCK_RAW_PATH"
ACTUAL_RAW_CODE_SIGNATURE_OFFSET=$(code_signature_offset "$SOURCE_RAW_CLIENT")
[ "$ACTUAL_RAW_CODE_SIGNATURE_OFFSET" = "$LOCK_RAW_CODE_SIGNATURE_OFFSET" ] ||
    fail "raw kyclient code-signature boundary mismatch: $ACTUAL_RAW_CODE_SIGNATURE_OFFSET"
ACTUAL_RAW_SHA256=$(shasum -a 256 "$SOURCE_RAW_CLIENT" | awk '{print $1}')
[ "$ACTUAL_RAW_SHA256" = "$LOCK_RAW_SHA256" ] ||
    fail "raw kyclient digest mismatch: $ACTUAL_RAW_SHA256"
ACTUAL_RAW_PAYLOAD_SHA256=$(
    dd if="$SOURCE_RAW_CLIENT" bs="$LOCK_RAW_CODE_SIGNATURE_OFFSET" count=1 2>/dev/null |
        shasum -a 256 |
        awk '{print $1}'
)
[ "$ACTUAL_RAW_PAYLOAD_SHA256" = "$LOCK_RAW_PAYLOAD_SHA256" ] ||
    fail "raw kyclient executable payload mismatch: $ACTUAL_RAW_PAYLOAD_SHA256"
codesign --verify --strict "$SOURCE_RAW_CLIENT" ||
    fail 'baseline raw kyclient signature is invalid'

XCODE_VERSION=$(xcodebuild -version | sed -n '1s/^Xcode //p')
XCODE_BUILD_VERSION=$(xcodebuild -version | sed -n '2s/^Build version //p')
SWIFT_VERSION=$(xcrun swiftc --version | tr '\n' ' ' | sed 's/[[:space:]]*$//')
SDK_VERSION=$(xcrun --sdk macosx --show-sdk-version)
SDK_BUILD_VERSION=$(xcrun --sdk macosx --show-sdk-build-version)
[ -n "$XCODE_VERSION" ] && [ -n "$XCODE_BUILD_VERSION" ] &&
    [ -n "$SWIFT_VERSION" ] && [ -n "$SDK_VERSION" ] &&
    [ -n "$SDK_BUILD_VERSION" ] ||
    fail 'could not capture complete macOS build-toolchain metadata'

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
    -target "arm64-apple-macos${DEPLOYMENT_TARGET}" \
    "$LAUNCHER_SOURCE" \
    -framework AppKit \
    -o "$STAGED_APP/Contents/MacOS/ReplayDesktopLauncher"

PLIST="$STAGED_APP/Contents/Info.plist"
set_plist_string() {
    plist_key=$1
    plist_value=$2
    plutil -replace "$plist_key" -string "$plist_value" "$PLIST" 2>/dev/null ||
        plutil -insert "$plist_key" -string "$plist_value" "$PLIST"
}

plutil -lint "$PLIST"
set_plist_string CFBundleExecutable ReplayDesktopLauncher
set_plist_string CFBundleName ReplayDesktop
set_plist_string CFBundleDisplayName ReplayDesktop
set_plist_string CFBundleShortVersionString "$SHORT_VERSION"
set_plist_string CFBundleVersion "$BUNDLE_VERSION"
set_plist_string LSMinimumSystemVersion "$DEPLOYMENT_TARGET"
plutil -replace NSHighResolutionCapable -bool true "$PLIST" 2>/dev/null ||
    plutil -insert NSHighResolutionCapable -bool true "$PLIST"
# The LAN MVP deliberately permits a cleartext broker only on the local network.
# Public deployments keep ATS enabled and terminate broker TLS normally.
plutil -replace NSAppTransportSecurity -json '{"NSAllowsLocalNetworking":true}' "$PLIST" 2>/dev/null ||
    plutil -insert NSAppTransportSecurity -json '{"NSAllowsLocalNetworking":true}' "$PLIST"
set_plist_string CFBundleGetInfoString \
    "ReplayDesktop $PACKAGE_VERSION internal prototype; ad-hoc signed and not notarized"
set_plist_string ReplayDesktopPackageVersion "$PACKAGE_VERSION"
set_plist_string ReplayDesktopSourceBase "$SOURCE_BASE"
set_plist_string ReplayDesktopSourceCommit "$SOURCE_COMMIT"
set_plist_string ReplayDesktopSourceBoundarySHA256 "$SOURCE_BOUNDARY_DIGEST"
set_plist_string ReplayDesktopKyberRelease "$LOCK_KYBER_RELEASE"
set_plist_string ReplayDesktopKyberCommit "$LOCK_KYBER_COMMIT"
set_plist_string ReplayDesktopKysdkCommit "$LOCK_KYSDK_COMMIT"
set_plist_string ReplayDesktopRawKyclientSHA256 "$LOCK_RAW_SHA256"
set_plist_string ReplayDesktopRawKyclientPayloadSHA256 "$LOCK_RAW_PAYLOAD_SHA256"
set_plist_string ReplayDesktopRawKyclientCodeSignatureOffset \
    "$LOCK_RAW_CODE_SIGNATURE_OFFSET"
set_plist_string ReplayDesktopXcodeVersion "$XCODE_VERSION"
set_plist_string ReplayDesktopXcodeBuildVersion "$XCODE_BUILD_VERSION"
set_plist_string ReplayDesktopSwiftVersion "$SWIFT_VERSION"
set_plist_string ReplayDesktopSDKVersion "$SDK_VERSION"
set_plist_string ReplayDesktopSDKBuildVersion "$SDK_BUILD_VERSION"
set_plist_string ReplayDesktopDeploymentTarget "$DEPLOYMENT_TARGET"

mkdir -p "$STAGED_APP/Contents/Resources"
ditto "$ENGINE_LOCK" \
    "$STAGED_APP/Contents/Resources/ReplayDesktopEngineBaseline.lock"
plutil -lint "$PLIST"

"$STAGED_APP/Contents/MacOS/ReplayDesktopLauncher" --self-test

find "$STAGED_APP" -type f -print | LC_ALL=C sort | while IFS= read -r candidate; do
    if file "$candidate" | grep -q 'Mach-O'; then
        codesign --force --sign - --timestamp=none "$candidate"
    fi
done
codesign --force --sign - --timestamp=none "$STAGED_APP"
codesign --verify --deep --strict "$STAGED_APP"
PACKAGED_RAW_CODE_SIGNATURE_OFFSET=$(
    code_signature_offset "$STAGED_APP/$LOCK_RAW_PATH"
)
[ "$PACKAGED_RAW_CODE_SIGNATURE_OFFSET" = "$LOCK_RAW_CODE_SIGNATURE_OFFSET" ] ||
    fail "packaging moved the raw kyclient code-signature boundary: $PACKAGED_RAW_CODE_SIGNATURE_OFFSET"
PACKAGED_RAW_SHA256=$(shasum -a 256 "$STAGED_APP/$LOCK_RAW_PATH" | awk '{print $1}')
PACKAGED_RAW_PAYLOAD_SHA256=$(
    dd if="$STAGED_APP/$LOCK_RAW_PATH" \
        bs="$LOCK_RAW_CODE_SIGNATURE_OFFSET" count=1 2>/dev/null |
        shasum -a 256 |
        awk '{print $1}'
)
[ "$PACKAGED_RAW_PAYLOAD_SHA256" = "$LOCK_RAW_PAYLOAD_SHA256" ] ||
    fail "packaging mutated the raw kyclient executable payload: $PACKAGED_RAW_PAYLOAD_SHA256"
codesign --verify --strict "$STAGED_APP/$LOCK_RAW_PATH" ||
    fail 'packaged raw kyclient signature is invalid'

find "$STAGED_APP" -exec touch -h -t "$PACKAGE_TIMESTAMP" {} +
(
    cd "$STAGING_DIR"
    find ReplayDesktop.app -print | LC_ALL=C sort |
        zip -X -y -q "$STAGED_ARCHIVE" -@
)
unzip -tq "$STAGED_ARCHIVE"
"$SCRIPT_DIR/verify-gui-spike.sh" \
    --app "$STAGED_APP" \
    --archive "$STAGED_ARCHIVE" \
    --expected-package-version "$PACKAGE_VERSION" \
    --expected-source-base "$SOURCE_BASE" \
    --expected-source-commit "$SOURCE_COMMIT" \
    --expected-source-boundary-digest "$SOURCE_BOUNDARY_DIGEST" \
    --expected-xcode-version "$XCODE_VERSION" \
    --expected-xcode-build "$XCODE_BUILD_VERSION" \
    --expected-swift-version "$SWIFT_VERSION" \
    --expected-sdk-version "$SDK_VERSION" \
    --expected-sdk-build "$SDK_BUILD_VERSION" \
    --expected-deployment-target "$DEPLOYMENT_TARGET"

mv "$STAGED_APP" "$FINAL_APP"
mv "$STAGED_ARCHIVE" "$FINAL_ARCHIVE"

printf 'APP=%s\n' "$FINAL_APP"
printf 'ARCHIVE=%s\n' "$FINAL_ARCHIVE"
printf 'PACKAGE_VERSION=%s\n' "$PACKAGE_VERSION"
printf 'SOURCE_COMMIT=%s\n' "$SOURCE_COMMIT"
printf 'RAW_KYCLIENT_INPUT_SHA256=%s\n' "$LOCK_RAW_SHA256"
printf 'RAW_KYCLIENT_PAYLOAD_SHA256=%s\n' "$LOCK_RAW_PAYLOAD_SHA256"
printf 'RAW_KYCLIENT_PACKAGED_SHA256=%s\n' "$PACKAGED_RAW_SHA256"
printf 'SHA256='
shasum -a 256 "$FINAL_ARCHIVE" | awk '{print $1}'
