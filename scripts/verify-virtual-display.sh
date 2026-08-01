#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TOOL="$SCRIPT_DIR/replay-virtual-display"

fail() {
    printf 'virtual-display verification failed: %s\n' "$*" >&2
    exit 1
}

need_line() {
    local pattern=$1
    local file=$2
    grep -Fq -- "$pattern" "$file" || fail "missing '$pattern' in $file"
}

bash -n "$TOOL"
bash "$TOOL" validate-assets >/dev/null

scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
fixture="$scratch/xorg.conf"
cat > "$fixture" <<'EOF'
# fixture prefix must survive byte-for-byte
Section "Device"
    Identifier "Other GPU"
    Driver "modesetting"
EndSection

Section "Device"
    Identifier  "NVIDIA Generic"
    Driver      "nvidia"
    Option      "HardDPMS" "False"
EndSection

Section "Screen"
    Identifier "Screen Default 0"
    Device "NVIDIA Generic"
EndSection
EOF

common=(
    --xorg-conf "$fixture"
    --state-dir "$scratch/state"
    --etc-dir "$scratch/etc"
)

bash "$TOOL" render headless-one "${common[@]}" > "$scratch/headless-one.conf"
need_line 'Option      "ConnectedMonitor" "DFP-2"' "$scratch/headless-one.conf"
need_line 'Option      "CustomEDID" "DFP-2:' "$scratch/headless-one.conf"
need_line 'Option      "MetaModes" "DFP-2: nvidia-auto-select +0+0"' "$scratch/headless-one.conf"

bash "$TOOL" render headless-two "${common[@]}" > "$scratch/headless-two.conf"
need_line 'Option      "ConnectedMonitor" "DFP-2, DFP-4"' "$scratch/headless-two.conf"
need_line 'DFP-4: nvidia-auto-select +3840+0' "$scratch/headless-two.conf"
[[ $(grep -Fc '# BEGIN REPLAYDESKTOP VIRTUAL DISPLAY' "$scratch/headless-two.conf") -eq 1 ]] \
    || fail "headless-two did not render exactly one managed block"

bash "$TOOL" render add-one --physical DFP-6.3 "${common[@]}" \
    > "$scratch/add-one.conf"
need_line 'Option      "ConnectedMonitor" "DFP-6.3, DFP-2"' "$scratch/add-one.conf"
need_line 'DFP-6.3: nvidia-auto-select +0+0, DFP-2: nvidia-auto-select +3840+0' \
    "$scratch/add-one.conf"

bash "$TOOL" render add-one --physical DFP-6.3 \
    --xorg-conf "$scratch/add-one.conf" \
    --state-dir "$scratch/state" --etc-dir "$scratch/etc" \
    > "$scratch/add-one-again.conf"
cmp -s "$scratch/add-one.conf" "$scratch/add-one-again.conf" \
    || fail "managed render is not idempotent"

bash "$TOOL" disable --dry-run \
    --xorg-conf "$scratch/add-one.conf" \
    --state-dir "$scratch/state" --etc-dir "$scratch/etc" \
    > "$scratch/disabled.conf"
cmp -s "$fixture" "$scratch/disabled.conf" \
    || fail "disable dry-run did not restore the original config bytes"

grep -Fq 'Identifier "Other GPU"' "$scratch/add-one.conf" \
    || fail "unrelated Device section was not preserved"
grep -Fq '# fixture prefix must survive byte-for-byte' "$scratch/add-one.conf" \
    || fail "fixture prefix was not preserved"
if sed -n '/# BEGIN REPLAYDESKTOP VIRTUAL DISPLAY/,/# END REPLAYDESKTOP VIRTUAL DISPLAY/p' \
    "$scratch/add-one.conf" | grep -Fq ModeValidation; then
    fail "managed block bypasses mode validation"
fi

if bash "$TOOL" render add-one "${common[@]}" >/dev/null 2>&1; then
    fail "add-one accepted a missing physical output"
fi
if bash "$TOOL" render headless-two --virtual-two DFP-2 "${common[@]}" \
    >/dev/null 2>&1; then
    fail "headless-two accepted duplicate virtual outputs"
fi
if bash "$TOOL" render headless-one --virtual-one DFP-2.1 "${common[@]}" \
    >/dev/null 2>&1; then
    fail "virtual output accepted an MST/alias name"
fi
if bash "$TOOL" render add-one --physical DFP-2 "${common[@]}" \
    >/dev/null 2>&1; then
    fail "add-one accepted the same physical and virtual output"
fi

need_line 'guard result.displayValue == 2 else {' \
    "$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"
need_line 'prototype accepted more than two simultaneous displays' \
    "$REPO_ROOT/prototype/macos/ReplayDesktopLauncher.swift"

printf 'VIRTUAL-DISPLAY VERIFY PASS: EDIDs, three layouts, idempotence, rollback, bounds\n'
