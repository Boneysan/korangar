#!/usr/bin/env bash
# Static contract for the QW-056 quest refresh matrix.
# Each authoritative event handler must refresh tracked-objective readiness.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_file="$repo/korangar/src/lib.rs"

assert_refresh() {
    local event="$1" start end block
    start="$(rg -n "^[[:space:]]*NetworkEvent::${event}" "$source_file" | head -n 1 | cut -d: -f1)"
    if [ -z "$start" ]; then
        echo "FAIL - NetworkEvent::$event handler is missing."
        exit 1
    fi
    end="$(awk -v start="$start" 'NR > start && $0 ~ /^[[:space:]]*NetworkEvent::/ { print NR; exit }' "$source_file")"
    if [ -z "$end" ]; then
        end="$(wc -l < "$source_file")"
    fi
    block="$(sed -n "${start},$((end - 1))p" "$source_file")"
    if ! printf '%s\n' "$block" | grep -q 'update_quest_auto_tracking()'; then
        echo "FAIL - NetworkEvent::$event no longer refreshes tracked objectives."
        exit 1
    fi
}

# `ItemObtained` is deliberately absent: it is the paired combat/chat
# notification, not an inventory mutation. `IventoryItemAdded` is the
# authoritative pickup event and is the only one allowed to refresh counts.
for event in \
    QuestAdded QuestRemoved QuestList QuestObjectiveProgress \
    SetInventory TradeCompleted IventoryItemAdded InventoryItemRemoved \
    PartyList PartyMemberAdded PartyMemberRemoved ChangeMap; do
    assert_refresh "$event"
done

echo "OK - quest refresh matrix handlers retain tracked-objective refreshes."
