# GUI session runsheet

**What this is.** The ordered list to work through at the screen, with what to
type and what to look for. The findings archive is
[gui-verification-pass.md](gui-verification-pass.md) — this is the thing you
follow; that is the thing you read afterwards.

**Run this first**, so the queue starts from what is actually unverified:

```sh
python3 tools/audits/gui-pass-staleness.py
```

---

## Bring-up

```sh
brew services run mariadb          # `run`, never `start` — start re-registers autostart

cd Hercules
./dev.sh start                     # fresh timestamped log, returns immediately
./dev.sh wait                      # blocks until the map-server is really ready (minutes)

cd ../korangar/korangar
cargo run --release --bin korangar
```

Second seat: run the client a second time and log in as the other character.

### Pre-flight — check this before you start, the seats drift

They had drifted by **2026-08-12**: `test` was a **Whitesmith in geffen**, and
**no party existed at all**. Party 280 is gone. Task 1 needs a Clown and a
Gypsy in one party, so it would have stalled on the first row — which is part of
why Hermode keeps not getting reached.

Check without starting the client (`<pw>` from `Hercules/conf/import/sql_connection.conf`):

```sh
mysql --protocol=tcp -h127.0.0.1 -uragnarok -p<pw> ragnarok -e \
 "SELECT name, class, last_map, party_id FROM \`char\` WHERE name IN ('test','HeadlessTwo');"
```

| Seat | Character | Wanted job | Weapon | Why the weapon matters |
|---|---|---|---|---|
| A | `test` | Clown **4020** | Violin (**1901**) | `CG_HERMODE`'s `WeaponTypes` is `Instruments` or `Whips` — bare-handed it refuses |
| B | `HeadlessTwo` | Gypsy **4021** | Rope (**1950**) | same |

Restore, in the client, if the query disagrees:

```
A:  @jobchange 4020        B:  @jobchange 4021
A:  @allskill              B:  @allskill
A:  @item 1901 1           B:  @item 1950 1
    equip it from inventory    equip it from inventory
A:  @warp prt_fild08 320 185
                           B:  @jumpto test
A:  @killmonster
```

Then **make a party** — A creates one from the party window and invites B. Don't
look for 280; the party table is empty and a new id will be issued.

To see the skill-unit instrumentation, launch with `KORANGAR_PACKET_LOG=1` — it
prints one `[skill-unit] spawn` line per cell with the resolved colour, the
texture, and whether that texture reports transparent. It is the thing to reach
for whenever a ground field looks wrong or does not appear.

---

## The order is not arbitrary — do not reorder these

Three of the tasks destroy the setup a later one needs, and one can end the
session outright.

1. **§1 must be first.** `@jobchange` destroys the Clown/Gypsy pair that the
   ensemble needs, and rebuilding it costs more than doing the row.
2. **§9 must be last.** The instance attempt has previously put *both* clients
   into a black screen with the interface still drawn and **no input accepted**,
   so the only way out was killing the process. Anything after it is lost.
3. **§5 Gospel clears the caster's own buffs** when it starts. Do not set
   anything up expecting to keep it.

---

## What has already been ruled out (2026-08-12, statically)

Every row was checked as far as reading code and probing the GRF can go, so a
live failure means something these could not see. **If one of these turns out to
be the cause anyway, that is itself the finding** — it means the static check was
wrong, which is worth more than the row.

| Row | Ruled out |
|---|---|
| 1 Hermode | The wav **exists** — `data\wav\effect\헤르모드의 지팡이.wav` in `data.grf` — and the path resolves: the recipe stores `effect\…` and `spawn_async_load` prepends `data\wav`. `UnitId::Hermode` is mapped, and a unit test already pins that it draws nothing. **Silence will not be a missing asset.** |
| 5–7 ground fields | All three hover textures exist: `cross_old.bmp`, `lens_w.bmp`, `curse.bmp`. **An invisible field will not be a missing texture** — which is what makes Gospel's α 0.05 the live question. |
| 3 · P5 | The offline roster row is implemented (`state/party.rs:114`). |
| 4 Auto Spell | Packet → `NetworkEvent::AutoSpellList` → `AutoSpellWindow` is wired, and **names are resolved from the `Library` at the event site**, not passed through as ids. Blank rows would mean the lookup missed, not that the feature is absent. |
| 8 · M1-009 | The `— vs equipped —` comparison block exists in `item_stats.rs`. |
| 8 · M1-014 | The two-step `Delete {name}…` → `Really delete {name}?` exists in `character_selection.rs`. |

What none of this can tell you is whether any of it **draws**, which is the
entire reason the pass exists.

---

## 1. Hermode — the oldest open row

**Never reached.** Sound-only by design.

- Both seats within **4 cells** of each other, both in the party.
- Seat A: open the skill window and cast **Hermode** (`CG_HERMODE`, **488**).
- **The window says "Hermode", not "Wand of Hermode".** The client synthesises
  the name from the skill identifier because our GRF is a Korean build.

| Look for | Verdict |
|---|---|
| You **hear** `헤르모드의 지팡이.wav` and **see nothing** | **PASS** — invisible is correct here |
| Sound but also a visual | FAIL — note what drew |
| No sound at all | FAIL — say whether the cast was refused (chat line) or accepted silently |

**Retry discipline:** a successful ensemble cast starts `SC_ENSEMBLEFATIGUE`
for **10 s on both partners**, and neither seat can use *any* skill during it.
`CG_MOONLIT` also has a 20 s reuse. **Wait ~30 s between attempts**, or clear it
with `@die` or a relog. A refusal here is usually the fatigue, not a bug.

Optional while the seats exist: re-cast **Moonlit** (`CG_MOONLIT`, **395**) as a
calibration reference for §5–§7. It should be a soft salmon field, 9×9, bobbing,
**not** a flat slab and not a red square. Both of those were fixed on 08-08.

---

## 2. Party roster + trade — re-walk, pixels only

The state layer for both is already covered by tests, so **only the screen half
is open**. Do not re-derive the data.

**2a. Party roster.** With both seats in the party, seat A leaves (party window
→ Leave).

| Look for | Verdict |
|---|---|
| Seat A's own roster empties **completely** | PASS |
| A's window still lists members after A left | FAIL |

**2b. Trade.** Seat A trades an item to seat B and both accept.

| Look for | Verdict |
|---|---|
| The item leaves **A's own inventory window** on screen | PASS |
| The item stays visible in A's inventory until relog | FAIL |

---

## 3. The four refuse/remove paths

Placed here deliberately: it needs both seats and a live party, which §4–§6's
job changes break up. Recovered by the 2026-08-12 reconciliation: Block A walked
every *accept* path and never a *refuse* one.

| # | Do this | PASS looks like |
|---|---|---|
| P1 | B **rejects** A's party invite | A gets a rejection line; B's status line returns to "Not in a party" |
| P5 | B logs out while partied | A's roster line for B flips to `(offline)`; the bars over B stop drawing |
| F3 | B **rejects** A's friend request | Request window closes on B; A gets the rejection line |
| F4 | A presses **Remove** on a friend | The row empties on **both** sides |

---

## 4. N20 — Auto Spell window

`@jobchange 16` (Sage) on seat A, then `@allskill`.

- Cast **`SA_AUTOSPELL`, skill id 279**.
- **Look for "Hindsight" in the skill window, not "Auto Spell".** The lua name
  differs from the code name — this is a recorded name trap and has cost time.

| Look for | Verdict |
|---|---|
| A window lists the offered spells **by name**; picking one closes it and the server accepts | PASS |
| Window lists ids, or blank rows | FAIL — say what the rows read |
| No window at all | FAIL — note any chat line |

---

## 5. Gospel — expected to be invisible

`@jobchange 4015` (Paladin), `@allskill`, `@heal` (needs 80–100 SP).

**Self-cast toggle**, `PA_GOSPEL` **369**, 60 s. Re-cast to cancel.
**It strips your own buffs when it starts.**

**We expect you to see nothing, and that is the point of this row.** Gospel is
coded at **α 0.05** — the exact value we measured as *not* porting to this
renderer, after Moonlit needed **0.6** to read as anything. If it is invisible,
the answer is the alpha, not the artwork.

The field is **two layers**, so the useful report distinguishes them:

| What you see | Means |
|---|---|
| Nothing at all | Both layers missing — check the `[skill-unit] spawn` log for whether cells spawned |
| A cross texture, no tint | Tile alpha too low (the predicted result) |
| A tint, no cross | Hover texture failed to load |
| Both | Better than predicted — say how it reads |

---

## 6. Fog Wall

`@jobchange 4017` (Professor), `@allskill`.

- `PF_FOGWALL` **404**, ground target, range 9, 25 SP, 20 s.
- **No gemstone needed on this server** — official RO wants a Blue Gemstone and
  this build does not. Do not go shopping.
- Coded at **α 0.6**, the calibrated magnitude, so this one *should* read.

Same two-layer question as §5.

---

## 7. Evil Land

No job learns it — reach it directly:

```
@useskill 670 10 <your character name>
```

`NPC_EVILLAND` **670**, 30 s, tile α 0.2, hover `curse.bmp` at a **half** cell —
one of only two entries the original table gives an explicit size for, so the
size is the interesting part here.

---

## 8. Two confirms that were never done

Both shipped 2026-07-22 as "code complete — live GUI confirm recommended", and
the confirm never happened. Single seat, any job.

| # | Do this | PASS looks like |
|---|---|---|
| M1-009 | Hover an inventory/equipment/storage item while wearing gear in that slot | Tooltip shows ATK/MATK/DEF/slots/req Lv/weight **and a vs-equipped delta** |
| M1-014 | Right-click a character at character select | Tooltip documents left-click play / right-click delete; delete is **two-step** (`Delete {name}…` → `Really delete {name}?`) |

## 9. N24 — Instance window — DO THIS LAST

**This has hard-locked both clients before.** Expect to kill the process.

Previously failed on Hercules truncating instanced map names past seven
characters, which is not a client bug. So use a **short** map name to get a
window-only check:

```
@dminstance start izlude 156 191
@dminstance end
```

| Look for | Verdict |
|---|---|
| Instance window opens naming it; `end` closes it | PASS |
| Black screen, interface drawn, no input | The known upstream failure — stop here, kill the client |

---

## Reporting back

Per task, this is all I need:

```
§1 Hermode        PASS/FAIL — what you saw, heard, and any chat line
§2a Party roster  PASS/FAIL — ...
```

Worth including whenever something looks wrong:

- **The exact chat line**, verbatim. A refusal that names its reason is usually
  the whole diagnosis.
- **Which of the two layers** you saw, for §4–§6.
- The `[skill-unit] spawn` lines if a ground field was wrong or missing.
- Anything you noticed **off the checklist**. Three of Block C's four bugs and
  six of the 08-08 batch were found that way, not by the rows.

A row failing is a result, not a problem — say what you saw rather than what you
think it means, and leave the diagnosis to the fix.
