#!/usr/bin/env bash
# Static contract for QW-046 area-loot cancellation wiring.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_file="$repo/korangar/src/lib.rs"

require_handler_cancel() {
    local handler="$1" reason="$2" start end block
    start="$(rg -n "^[[:space:]]*${handler}" "$source_file" | head -n 1 | cut -d: -f1)"
    if [ -z "$start" ]; then
        echo "FAIL - ${handler} handler is missing."
        exit 1
    fi
    end="$(awk -v start="$start" 'NR > start && $0 ~ /^[[:space:]]*(InputEvent|NetworkEvent)::/ { print NR; exit }' "$source_file")"
    if [ -z "$end" ]; then
        end="$(wc -l < "$source_file")"
    fi
    block="$(sed -n "${start},$((end - 1))p" "$source_file")"
    if ! printf '%s\n' "$block" | grep -q "AreaLootCancel::${reason}"; then
        echo "FAIL - ${handler} does not cancel area loot with ${reason}."
        exit 1
    fi
}

require_handler_cancel 'InputEvent::PlayerMove' 'ManualAction'
require_handler_cancel 'InputEvent::PlayerInteract' 'Combat'
require_handler_cancel 'InputEvent::PickUpItem' 'PathFailure'
require_handler_cancel 'InputEvent::DropItem' 'PlayerDrop'
require_handler_cancel 'NetworkEvent::ChangeMap' 'MapChange'

if ! rg -q 'area_loot\(\)\)\.on_item_vanished\(entity_id\)' "$source_file"; then
    echo "FAIL - vanished ground items no longer remove one queued id."
    exit 1
fi
if ! rg -q 'ignore_new_ground_item' "$repo/korangar/src"; then
    echo "FAIL - new ground items are not protected from automatic queueing."
    exit 1
fi

echo "OK - area-loot cancellation and non-auto-queue wiring is present."
