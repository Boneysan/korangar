# Approved playtest progression (2026-09-15)

## Recovery

- Combat: 8s after damage dealt/taken or offensive skill. Support/heal does not start combat unless the target is already in combat.
- Standing and walking HP/SP (in or out of combat): official RO **sitting**
  cadence — same amount (`1 + VIT/5 + max_hp/200` HP, `1 + INT/6 + max_sp/100` SP)
  at **2×** standing frequency (~3s HP, ~4s SP). Walking does **not** cut regen.
- Sitting: 25% of **max** HP and 25% of **max** SP every 10s, **replaces** standing (not stacked). Clamp at max.
- Overweight ≥50%: no natural regen (including sitting 25%).
- Poison: still stops all regen.
- Respawn: 50% HP/SP immediately; remaining 50% over 10s; damage cancels remainder.

## Weight

- Max weight ×5.
- Warn 70% (yellow), soft 90% (red, attacks still allowed), hard 100% (cannot pick up).

## EXP / party

- Solo `base_exp_rate` / `job_exp_rate`: 100.
- `quest_exp_rate`: 100 (same multiplier).
- `party_even_share_bonus`: 25 (per additional even-share member).
- `party_share_level`: 15.
- Share party Zeny with even EXP (existing even-share zeny path).

## Player tools

- `@save` / `@load` and Menu **Save here**.
- Unlimited `@resetskill` and Menu **Reset skills**.

## Chests

- Hidden chest discoveries stay per-character.

## Still open

- QW-087 cosmetics slice (not chosen).
