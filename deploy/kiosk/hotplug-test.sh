#!/usr/bin/env bash
# Prüft lagemonitor-kiosk-hotplug gegen ein nachgebautes /sys/class/drm. Aufruf: deploy/kiosk/hotplug-test.sh
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/drm/card0-HDMI-A-2" "$tmp/drm/card0-DP-1" "$tmp/bin"
printf '#!/bin/sh\necho "$*" >>"%s/calls"\n' "$tmp" >"$tmp/bin/systemctl"
chmod +x "$tmp/bin/systemctl"

step() { # <HDMI-A-2> <DP-1> <erwartete Neustarts insgesamt> <Beschreibung>
	echo "$1" >"$tmp/drm/card0-HDMI-A-2/status"
	echo "$2" >"$tmp/drm/card0-DP-1/status"
	PATH="$tmp/bin:$PATH" DRM_SYSFS="$tmp/drm" STATE_FILE="$tmp/state" SETTLE=0 "$here/lagemonitor-kiosk-hotplug" >/dev/null
	local got
	got=$(grep -c 'try-restart lagemonitor-kiosk.service' "$tmp/calls" 2>/dev/null || true)
	[[ ${got:-0} == "$3" ]] || { echo "FEHLER: $4 – $3 Neustarts erwartet, $got bekommen"; exit 1; }
	echo "ok: $4"
}

step disconnected disconnected 0 "Boot ohne Monitor"
step connected disconnected 1 "Monitor eingeschaltet"
step connected disconnected 1 "weiteres Ereignis ohne Änderung (z. B. Modeset)"
step disconnected disconnected 1 "Monitor ausgeschaltet"
step connected disconnected 2 "Monitor wieder eingeschaltet"
step connected connected 3 "zweiter Monitor dazu"
