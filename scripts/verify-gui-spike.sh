#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
ENGINE_LOCK="$REPO_ROOT/prototype/macos/engine-baseline.lock"
APP_PATH="$PWD/ReplayDesktop.app"
ARCHIVE_PATH="$PWD/ReplayDesktop-arm64-gui-spike.zip"
SOURCE_BOUNDARY=false
SOURCE_BASE=""
SOURCE_HEAD=""
EXPECTED_PACKAGE_VERSION=""
EXPECTED_SOURCE_BASE=""
EXPECTED_SOURCE_COMMIT=""
EXPECTED_SOURCE_BOUNDARY_DIGEST=""
EXPECTED_XCODE_VERSION=""
EXPECTED_XCODE_BUILD=""
EXPECTED_SWIFT_VERSION=""
EXPECTED_SDK_VERSION=""
EXPECTED_SDK_BUILD=""
EXPECTED_DEPLOYMENT_TARGET=""

usage() {
    printf '%s\n' \
        "Usage:" \
        "  $0 --app PATH --archive PATH --expected-package-version VERSION" \
        "     --expected-source-base COMMIT --expected-source-commit COMMIT" \
        "     --expected-source-boundary-digest SHA256" \
        "     --expected-xcode-version VERSION --expected-xcode-build BUILD" \
        "     --expected-swift-version VERSION --expected-sdk-version VERSION" \
        "     --expected-sdk-build BUILD --expected-deployment-target VERSION" \
        "  $0 --source-boundary --base COMMIT --head COMMIT" \
        "" \
        "The default mode verifies the signed macOS GUI bundle and archive." \
        "--source-boundary verifies a replayable committed parent-repository" \
        "range and the reproducible kynput patch without accepting nested changes."
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
        --base)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            SOURCE_BASE=$2
            shift 2
            ;;
        --head)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            SOURCE_HEAD=$2
            shift 2
            ;;
        --expected-package-version)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_PACKAGE_VERSION=$2
            shift 2
            ;;
        --expected-source-base)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_SOURCE_BASE=$2
            shift 2
            ;;
        --expected-source-commit)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_SOURCE_COMMIT=$2
            shift 2
            ;;
        --expected-source-boundary-digest)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_SOURCE_BOUNDARY_DIGEST=$2
            shift 2
            ;;
        --expected-xcode-version)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_XCODE_VERSION=$2
            shift 2
            ;;
        --expected-xcode-build)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_XCODE_BUILD=$2
            shift 2
            ;;
        --expected-swift-version)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_SWIFT_VERSION=$2
            shift 2
            ;;
        --expected-sdk-version)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_SDK_VERSION=$2
            shift 2
            ;;
        --expected-sdk-build)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_SDK_BUILD=$2
            shift 2
            ;;
        --expected-deployment-target)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EXPECTED_DEPLOYMENT_TARGET=$2
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

verify_source_boundary() {
    cd "$REPO_ROOT"
    git rev-parse --show-toplevel >/dev/null 2>&1 ||
        fail 'source-boundary verification requires the parent git repository'

    [ -n "$SOURCE_BASE" ] || fail '--base is required with --source-boundary'
    [ -n "$SOURCE_HEAD" ] || fail '--head is required with --source-boundary'
    source_base_commit=$(git rev-parse --verify "${SOURCE_BASE}^{commit}") ||
        fail "source base is not a commit: $SOURCE_BASE"
    source_head_commit=$(git rev-parse --verify "${SOURCE_HEAD}^{commit}") ||
        fail "source head is not a commit: $SOURCE_HEAD"
    [ "$source_base_commit" != "$source_head_commit" ] ||
        fail 'source-boundary range must contain at least one commit'
    git merge-base --is-ancestor "$source_base_commit" "$source_head_commit" ||
        fail 'source base is not an ancestor of source head'

    head_lock_value() {
        head_lock_key=$1
        head_lock_result=$(
            git show \
                "$source_head_commit:prototype/macos/engine-baseline.lock" |
                sed -n "s/^${head_lock_key}=//p"
        )
        [ -n "$head_lock_result" ] ||
            fail "committed engine baseline lock is missing $head_lock_key"
        [ "$(printf '%s\n' "$head_lock_result" | wc -l | tr -d ' ')" = 1 ] ||
            fail "committed engine baseline lock repeats $head_lock_key"
        printf '%s\n' "$head_lock_result"
    }

    expected_desktop=$(head_lock_value kyber_commit)
    expected_kysdk=$(head_lock_value kysdk_commit)
    expected_kynput=$(head_lock_value kynput_commit)

    desktop_gitlink=$(
        git ls-tree "$source_head_commit" upstream/kyber-desktop | awk '{print $3}'
    )
    [ "$desktop_gitlink" = "$expected_desktop" ] ||
        fail "parent Kyber gitlink drifted: $desktop_gitlink"
    [ "$(git -C upstream/kyber-desktop rev-parse HEAD)" = "$expected_desktop" ] ||
        fail 'working Kyber checkout is not at the pinned detached commit'

    kysdk_gitlink=$(
        git -C upstream/kyber-desktop ls-tree "$expected_desktop" kysdk |
            awk '{print $3}'
    )
    [ "$kysdk_gitlink" = "$expected_kysdk" ] ||
        fail "Kyber SDK gitlink drifted: $kysdk_gitlink"
    [ "$(git -C upstream/kyber-desktop/kysdk rev-parse HEAD)" = "$expected_kysdk" ] ||
        fail 'working Kyber SDK checkout is not at the pinned detached commit'

    kynput_gitlink=$(
        git -C upstream/kyber-desktop/kysdk ls-tree "$expected_kysdk" kynput |
            awk '{print $3}'
    )
    [ "$kynput_gitlink" = "$expected_kynput" ] ||
        fail "kynput gitlink drifted: $kynput_gitlink"
    [ "$(git -C upstream/kyber-desktop/kysdk/kynput rev-parse HEAD)" = "$expected_kynput" ] ||
        fail 'working kynput checkout is not at the pinned detached commit'

    range_paths=$(git diff --name-only "$source_base_commit" "$source_head_commit")
    [ -n "$range_paths" ] || fail 'source-boundary commit range is empty'

    while IFS= read -r path; do
        range_status=$(
            git diff --name-status "$source_base_commit" "$source_head_commit" -- "$path" |
                awk 'NR == 1 {print $1}'
        )
        case "$range_status" in
            A|M) ;;
            D*) fail "source-boundary range must not delete files: $path" ;;
            *) fail "source-boundary range has unsupported status $range_status: $path" ;;
        esac

        case "$path" in
            README.md|\
            prototype/macos/ReplayDesktopLauncher.swift|\
            prototype/macos/engine-baseline.lock|\
            scripts/package-macos-gui.sh|\
            scripts/verify-gui-spike.sh|\
            patches/kyber/0004-linux-hires-wheel.patch)
                ;;
            upstream/kyber-desktop|upstream/kyber-desktop/*)
                fail "nested Kyber source must never be committed in the parent range: $path"
                ;;
            *)
                fail "unexpected committed source-range path: $path"
                ;;
        esac

        case "$path" in
            *.pem|*.key|*.p12|*.pfx|*.jwt|*.log|log/*|*/log/*|*/logs/*|\
            *private-config*|*identity-material*)
                fail "private, credential, or runtime material is committed: $path"
                ;;
        esac
    done <<EOF
$range_paths
EOF

    private_marker='BEGIN PRIVATE'" KEY"
    certificate_marker='BEGIN CERT'"IFICATE"
    test_identity='kyber'"test_"
    jwt_prefix='e''yJ'
    while IFS= read -r path; do
        if git grep -I -E -i -q \
            "$private_marker|$certificate_marker|$test_identity|$jwt_prefix[A-Za-z0-9_-]{10,}\\." \
            "$source_head_commit" -- "$path"
        then
            fail "credential or test identity marker found in committed content: $path"
        fi
    done <<EOF
$range_paths
EOF

    git cat-file -e \
        "$source_head_commit:patches/kyber/0004-linux-hires-wheel.patch" ||
        fail 'source head is missing the reproducible scroll patch'
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
    git -C "$scratch/kynput" apply --check \
        <(git show "$source_head_commit:patches/kyber/0004-linux-hires-wheel.patch")

    source_boundary_digest=$(
        git diff \
            --no-ext-diff --no-color --binary --full-index \
            "$source_base_commit" "$source_head_commit" |
            shasum -a 256 |
            awk '{print $1}'
    )

    printf '%s\n' \
        'PASS: committed range stays inside the GUI-spike parent source boundary' \
        'PASS: Kyber, Kyber SDK, and kynput gitlinks retain their exact pins' \
        'PASS: no private/test identity material is committed in the range' \
        'PASS: committed 0004-linux-hires-wheel.patch applies to clean pinned kynput' \
        'SOURCE_BOUNDARY=PASS' \
        "SOURCE_BASE=$source_base_commit" \
        "SOURCE_COMMIT=$source_head_commit" \
        "SOURCE_BOUNDARY_SHA256=$source_boundary_digest"
}

verify_bundle() {
    [ "$(uname -s)" = "Darwin" ] ||
        fail 'bundle verification requires macOS tools'
    [ -f "$ENGINE_LOCK" ] ||
        fail "engine baseline lock not found: $ENGINE_LOCK"

    printf '%s\n' "$EXPECTED_PACKAGE_VERSION" |
        grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+-spike\.[0-9]+$' ||
        fail '--expected-package-version is required and must be explicit'
    printf '%s\n' "$EXPECTED_SOURCE_BASE" | grep -Eq '^[0-9a-f]{40}$' ||
        fail '--expected-source-base must be a full lowercase Git commit'
    printf '%s\n' "$EXPECTED_SOURCE_COMMIT" | grep -Eq '^[0-9a-f]{40}$' ||
        fail '--expected-source-commit must be a full lowercase Git commit'
    printf '%s\n' "$EXPECTED_SOURCE_BOUNDARY_DIGEST" | grep -Eq '^[0-9a-f]{64}$' ||
        fail '--expected-source-boundary-digest must be a lowercase SHA-256'
    printf '%s\n' "$EXPECTED_XCODE_VERSION" | grep -Eq '^[0-9]+(\.[0-9]+)+$' ||
        fail '--expected-xcode-version is required'
    printf '%s\n' "$EXPECTED_XCODE_BUILD" | grep -Eq '^[A-Za-z0-9._-]+$' ||
        fail '--expected-xcode-build is required'
    case "$EXPECTED_SWIFT_VERSION" in
        *'Apple Swift version '*) ;;
        *) fail '--expected-swift-version must name the Apple Swift toolchain' ;;
    esac
    printf '%s\n' "$EXPECTED_SDK_VERSION" | grep -Eq '^[0-9]+(\.[0-9]+)+$' ||
        fail '--expected-sdk-version is required'
    printf '%s\n' "$EXPECTED_SDK_BUILD" | grep -Eq '^[A-Za-z0-9._-]+$' ||
        fail '--expected-sdk-build is required'
    printf '%s\n' "$EXPECTED_DEPLOYMENT_TARGET" | grep -Eq '^[0-9]+(\.[0-9]+)+$' ||
        fail '--expected-deployment-target is required'

    lock_format=$(lock_value format)
    lock_kyber_release=$(lock_value kyber_release)
    lock_kyber_commit=$(lock_value kyber_commit)
    lock_kysdk_commit=$(lock_value kysdk_commit)
    lock_raw_path=$(lock_value raw_kyclient_bundle_path)
    lock_raw_sha256=$(lock_value raw_kyclient_sha256)
    lock_raw_payload_sha256=$(lock_value raw_kyclient_payload_sha256)
    lock_raw_code_signature_offset=$(lock_value raw_kyclient_code_signature_offset)
    lock_raw_minimum_macos=$(lock_value raw_kyclient_minimum_macos)
    [ "$lock_format" = 1 ] ||
        fail "unsupported engine baseline lock format: $lock_format"
    [ "$lock_raw_path" = Contents/MacOS/kyclient ] ||
        fail "unsupported raw engine bundle path: $lock_raw_path"
    printf '%s\n' "$lock_kyber_commit" | grep -Eq '^[0-9a-f]{40}$' ||
        fail 'engine baseline lock contains an invalid Kyber commit'
    printf '%s\n' "$lock_kysdk_commit" | grep -Eq '^[0-9a-f]{40}$' ||
        fail 'engine baseline lock contains an invalid Kysdk commit'
    printf '%s\n' "$lock_raw_sha256" | grep -Eq '^[0-9a-f]{64}$' ||
        fail 'engine baseline lock contains an invalid raw kyclient digest'
    printf '%s\n' "$lock_raw_payload_sha256" | grep -Eq '^[0-9a-f]{64}$' ||
        fail 'engine baseline lock contains an invalid raw kyclient payload digest'
    printf '%s\n' "$lock_raw_code_signature_offset" | grep -Eq '^[1-9][0-9]*$' ||
        fail 'engine baseline lock contains an invalid code-signature offset'

    for tool in plutil file otool codesign unzip zipinfo shasum stat readlink cmp diff dd; do
        command -v "$tool" >/dev/null 2>&1 ||
            fail "required macOS verification tool not found: $tool"
    done

    [ -d "$APP_PATH/Contents" ] || fail "app bundle not found: $APP_PATH"
    [ -f "$ARCHIVE_PATH" ] || fail "archive not found: $ARCHIVE_PATH"

    plist="$APP_PATH/Contents/Info.plist"
    launcher="$APP_PATH/Contents/MacOS/ReplayDesktopLauncher"
    raw_client="$APP_PATH/$lock_raw_path"
    bundled_engine_lock="$APP_PATH/Contents/Resources/ReplayDesktopEngineBaseline.lock"
    [ -x "$launcher" ] || fail 'ReplayDesktopLauncher is not executable'
    [ -x "$raw_client" ] || fail 'raw Contents/MacOS/kyclient fallback is missing'
    [ -f "$bundled_engine_lock" ] ||
        fail 'bundled engine baseline lock is missing'
    cmp -s "$ENGINE_LOCK" "$bundled_engine_lock" ||
        fail 'bundled engine baseline lock differs from tracked source lock'

    plutil -lint "$plist"
    executable=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$plist")
    [ "$executable" = ReplayDesktopLauncher ] ||
        fail "CFBundleExecutable is $executable"
    minimum_os=$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$plist")
    [ "$minimum_os" = "$EXPECTED_DEPLOYMENT_TARGET" ] ||
        fail "LSMinimumSystemVersion is $minimum_os"

    plist_string() {
        /usr/libexec/PlistBuddy -c "Print :$1" "$plist" 2>/dev/null ||
            fail "bundle build metadata is missing $1"
    }

    package_version=$(plist_string ReplayDesktopPackageVersion)
    source_base=$(plist_string ReplayDesktopSourceBase)
    source_commit=$(plist_string ReplayDesktopSourceCommit)
    source_boundary_digest=$(plist_string ReplayDesktopSourceBoundarySHA256)
    metadata_kyber_release=$(plist_string ReplayDesktopKyberRelease)
    metadata_kyber_commit=$(plist_string ReplayDesktopKyberCommit)
    metadata_kysdk_commit=$(plist_string ReplayDesktopKysdkCommit)
    metadata_raw_sha256=$(plist_string ReplayDesktopRawKyclientSHA256)
    metadata_raw_payload_sha256=$(
        plist_string ReplayDesktopRawKyclientPayloadSHA256
    )
    metadata_raw_code_signature_offset=$(
        plist_string ReplayDesktopRawKyclientCodeSignatureOffset
    )
    metadata_xcode_version=$(plist_string ReplayDesktopXcodeVersion)
    metadata_xcode_build=$(plist_string ReplayDesktopXcodeBuildVersion)
    metadata_swift_version=$(plist_string ReplayDesktopSwiftVersion)
    metadata_sdk_version=$(plist_string ReplayDesktopSDKVersion)
    metadata_sdk_build=$(plist_string ReplayDesktopSDKBuildVersion)
    metadata_deployment_target=$(plist_string ReplayDesktopDeploymentTarget)

    [ "$package_version" = "$EXPECTED_PACKAGE_VERSION" ] ||
        fail "package version metadata mismatch: $package_version"
    expected_short_version=$(printf '%s\n' "$EXPECTED_PACKAGE_VERSION" |
        sed -E 's/^v([0-9]+\.[0-9]+\.[0-9]+)-spike\.[0-9]+$/\1/')
    expected_spike_number=$(printf '%s\n' "$EXPECTED_PACKAGE_VERSION" |
        sed -E 's/^v[0-9]+\.[0-9]+\.[0-9]+-spike\.([0-9]+)$/\1/')
    expected_major=$(printf '%s\n' "$expected_short_version" | awk -F. '{print $1}')
    expected_minor=$(printf '%s\n' "$expected_short_version" | awk -F. '{print $2}')
    expected_patch=$(printf '%s\n' "$expected_short_version" | awk -F. '{print $3}')
    expected_bundle_version=$(
        printf '%s\n' \
            "$((expected_major * 1000000 + expected_minor * 10000 + expected_patch * 100 + expected_spike_number))"
    )
    actual_short_version=$(
        /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$plist"
    )
    actual_bundle_version=$(
        /usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$plist"
    )
    bundle_info=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleGetInfoString' "$plist")
    [ "$actual_short_version" = "$expected_short_version" ] ||
        fail "CFBundleShortVersionString mismatch: $actual_short_version"
    [ "$actual_bundle_version" = "$expected_bundle_version" ] ||
        fail "CFBundleVersion mismatch: $actual_bundle_version"
    case "$bundle_info" in
        *"$EXPECTED_PACKAGE_VERSION"*) ;;
        *) fail "CFBundleGetInfoString omits explicit package version: $bundle_info" ;;
    esac
    [ "$source_base" = "$EXPECTED_SOURCE_BASE" ] ||
        fail "source base metadata mismatch: $source_base"
    [ "$source_commit" = "$EXPECTED_SOURCE_COMMIT" ] ||
        fail "source commit metadata mismatch: $source_commit"
    [ "$source_boundary_digest" = "$EXPECTED_SOURCE_BOUNDARY_DIGEST" ] ||
        fail "source-boundary metadata mismatch: $source_boundary_digest"
    [ "$metadata_kyber_release" = "$lock_kyber_release" ] ||
        fail "Kyber release metadata mismatch: $metadata_kyber_release"
    [ "$metadata_kyber_commit" = "$lock_kyber_commit" ] ||
        fail "Kyber commit metadata mismatch: $metadata_kyber_commit"
    [ "$metadata_kysdk_commit" = "$lock_kysdk_commit" ] ||
        fail "Kysdk commit metadata mismatch: $metadata_kysdk_commit"
    [ "$metadata_raw_sha256" = "$lock_raw_sha256" ] ||
        fail "raw kyclient digest metadata mismatch: $metadata_raw_sha256"
    [ "$metadata_raw_payload_sha256" = "$lock_raw_payload_sha256" ] ||
        fail "raw kyclient payload metadata mismatch: $metadata_raw_payload_sha256"
    [ "$metadata_raw_code_signature_offset" = "$lock_raw_code_signature_offset" ] ||
        fail "raw kyclient signature-offset metadata mismatch: $metadata_raw_code_signature_offset"
    [ "$metadata_deployment_target" = "$minimum_os" ] &&
        [ "$metadata_deployment_target" = "$lock_raw_minimum_macos" ] ||
        fail "deployment-target metadata mismatch: $metadata_deployment_target"

    [ "$metadata_xcode_version" = "$EXPECTED_XCODE_VERSION" ] ||
        fail "Xcode version metadata mismatch: $metadata_xcode_version"
    [ "$metadata_xcode_build" = "$EXPECTED_XCODE_BUILD" ] ||
        fail "Xcode build metadata mismatch: $metadata_xcode_build"
    [ "$metadata_swift_version" = "$EXPECTED_SWIFT_VERSION" ] ||
        fail 'Swift version metadata does not match the expected build toolchain'
    [ "$metadata_sdk_version" = "$EXPECTED_SDK_VERSION" ] ||
        fail "SDK version metadata mismatch: $metadata_sdk_version"
    [ "$metadata_sdk_build" = "$EXPECTED_SDK_BUILD" ] ||
        fail "SDK build metadata mismatch: $metadata_sdk_build"
    [ "$metadata_deployment_target" = "$EXPECTED_DEPLOYMENT_TARGET" ] ||
        fail "deployment target metadata mismatch: $metadata_deployment_target"

    raw_client_sha256=$(shasum -a 256 "$raw_client" | awk '{print $1}')
    raw_client_code_signature_offset=$(code_signature_offset "$raw_client")
    [ "$raw_client_code_signature_offset" = "$lock_raw_code_signature_offset" ] ||
        fail "raw kyclient code-signature boundary differs from baseline: $raw_client_code_signature_offset"
    raw_client_payload_sha256=$(
        dd if="$raw_client" bs="$lock_raw_code_signature_offset" count=1 2>/dev/null |
            shasum -a 256 |
            awk '{print $1}'
    )
    [ "$raw_client_payload_sha256" = "$lock_raw_payload_sha256" ] ||
        fail "raw kyclient executable payload differs from baseline: $raw_client_payload_sha256"

    temp_root=${TMPDIR:-/tmp}
    inventory=$(mktemp "$temp_root/replaydesktop-inventory.XXXXXX")
    app_manifest=$(mktemp "$temp_root/replaydesktop-app-manifest.XXXXXX")
    archive_manifest=$(mktemp "$temp_root/replaydesktop-archive-manifest.XXXXXX")
    runtime_probe=$(mktemp -d "$temp_root/replaydesktop-runtime.XXXXXX")
    archive_probe=$(mktemp -d "$temp_root/replaydesktop-archive.XXXXXX")
    cleanup_bundle() {
        rm -f -- "$inventory"
        rm -f -- "$app_manifest" "$archive_manifest"
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
                    awk '$1 == "cmd" &&
                             ($2 == "LC_BUILD_VERSION" ||
                              $2 == "LC_VERSION_MIN_MACOSX") {
                             build = 1
                             next
                         }
                         build && ($1 == "minos" || $1 == "version") {
                             print $2
                             exit
                         }')
                [ -n "$minos" ] ||
                    fail "Mach-O has no macOS minimum-version load command: $candidate"
                awk -v version="$minos" 'BEGIN {
                    split(version, component, ".")
                    major = component[1] + 0
                    minor = component[2] + 0
                    exit ((major < 15) || (major == 15 && minor <= 0)) ? 0 : 1
                }' </dev/null ||
                    fail "Mach-O requires macOS $minos, newer than supported 15.0: $candidate"
                printf '  minos %s: %s\n' "$minos" "$candidate"
                ;;
        esac
    done <"$inventory"
    [ "$macho_count" -gt 1 ] || fail 'Mach-O inventory did not include launcher and raw client'

    launcher_minos=$(otool -l "$launcher" |
        awk '$1 == "cmd" &&
                 ($2 == "LC_BUILD_VERSION" ||
                  $2 == "LC_VERSION_MIN_MACOSX") {
                 build = 1
                 next
             }
             build && ($1 == "minos" || $1 == "version") {
                 print $2
                 exit
             }')
    [ "$launcher_minos" = 15.0 ] ||
        fail "launcher deployment target is $launcher_minos, expected 15.0"
    launcher_sdk=$(otool -l "$launcher" |
        awk '$1 == "cmd" && $2 == "LC_BUILD_VERSION" {
                 build = 1
                 next
             }
             build && $1 == "sdk" {
                 print $2
                 exit
             }')
    [ "$launcher_sdk" = "$metadata_sdk_version" ] ||
        fail "launcher SDK $launcher_sdk differs from metadata $metadata_sdk_version"

    raw_description=$(file "$raw_client")
    case "$raw_description" in
        *Mach-O*arm64*) ;;
        *) fail "raw kyclient is not an arm64 Mach-O: $raw_description" ;;
    esac
    case "$raw_description" in
        *x86_64*|*universal*) fail "raw kyclient is not arm64-only: $raw_description" ;;
    esac
    raw_minos=$(otool -l "$raw_client" |
        awk '$1 == "cmd" &&
                 ($2 == "LC_BUILD_VERSION" ||
                  $2 == "LC_VERSION_MIN_MACOSX") {
                 build = 1
                 next
             }
             build && ($1 == "minos" || $1 == "version") {
                 print $2
                 exit
             }')
    [ -n "$raw_minos" ] ||
        fail 'raw kyclient has no macOS minimum-version load command'
    [ "$raw_minos" = "$lock_raw_minimum_macos" ] ||
        fail "raw kyclient minimum OS differs from baseline lock: $raw_minos"

    codesign --verify --deep --strict "$APP_PATH"
    codesign --verify --strict "$raw_client"
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

    bundle_manifest() {
        manifest_app=$1
        manifest_output=$2
        (
            cd "$manifest_app"
            find . -print | LC_ALL=C sort | while IFS= read -r entry; do
                permissions=$(stat -f '%Sp' "$entry")
                if [ -L "$entry" ]; then
                    printf 'L\t%s\t%s\t%s\n' \
                        "$permissions" "$entry" "$(readlink "$entry")"
                elif [ -f "$entry" ]; then
                    checksum=$(shasum -a 256 "$entry" | awk '{print $1}')
                    printf 'F\t%s\t%s\t%s\n' \
                        "$permissions" "$entry" "$checksum"
                elif [ -d "$entry" ]; then
                    printf 'D\t%s\t%s\n' "$permissions" "$entry"
                else
                    fail "unsupported bundle entry type: $entry"
                fi
            done
        ) >"$manifest_output"
    }

    bundle_manifest "$APP_PATH" "$app_manifest"
    bundle_manifest "$archive_probe/ReplayDesktop.app" "$archive_manifest"
    if ! cmp -s "$app_manifest" "$archive_manifest"; then
        diff -u "$app_manifest" "$archive_manifest" >&2 || true
        fail 'archived bundle differs from the fully verified source bundle'
    fi

    checksum=$(shasum -a 256 "$ARCHIVE_PATH" | awk '{print $1}')
    printf '%s\n' \
        "PASS: bundle=$APP_PATH" \
        "PASS: CFBundleExecutable=$executable" \
        "PASS: package version=$package_version (bundle $actual_bundle_version)" \
        "PASS: source=$source_base..$source_commit" \
        "PASS: source-boundary SHA256=$source_boundary_digest" \
        "PASS: Kyber=$metadata_kyber_release@$metadata_kyber_commit" \
        "PASS: LSMinimumSystemVersion=$minimum_os" \
        "PASS: arm64 Mach-O count=$macho_count" \
        "PASS: launcher minos=$launcher_minos sdk=$launcher_sdk" \
        "PASS: raw kyclient input SHA256=$lock_raw_sha256" \
        "PASS: raw kyclient payload SHA256=$raw_client_payload_sha256 packaged SHA256=$raw_client_sha256" \
        "PASS: raw kyclient code-signature offset=$raw_client_code_signature_offset" \
        "PASS: raw kyclient minos=$raw_minos and arm64-only" \
        "PASS: Xcode=$metadata_xcode_version ($metadata_xcode_build), SDK=$metadata_sdk_version ($metadata_sdk_build)" \
        "PASS: Swift=$metadata_swift_version" \
        'PASS: ad-hoc signature, raw CLI fallback, runtime directory, and exact archive manifest' \
        "SHA256=$checksum"
}

if [ "$SOURCE_BOUNDARY" = true ]; then
    verify_source_boundary
else
    verify_bundle
fi
