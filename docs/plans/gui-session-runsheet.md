# GUI session runsheet

**What this is.** The ordered list to work through at the screen, with what to
type and what to look for. The findings archive is
[gui-verification-pass.md](gui-verification-pass.md) — this is the thing you
follow; that is the thing you read afterwards.

> **2026-08-17 — §1 through §6 are DONE.** Start at **§7 Evil Land**, which
> should be the cheap one: `curse.bmp` is 76.4% magenta, so it is the keyed
> family and needs nothing that §6 just built.
>
> **§6 Fog Wall passed after four live passes and found a bug in Land
> Protector**, which had already been verified back on 2026-07-24 — its texture
> is the additive family too and had been alpha-blended the whole time, laying a
> dark backing under all 121 cells. **Re-walk Land Protector**
> (`@useskill 288 5 <your name>`) before closing the ground-field family.
>
> **2026-08-16 — §1 through §5.** §1 Hermode passed for the first time ever; §2,
> §3, §4 and §5 all passed, and §3 and §5 each turned up a real bug that is now
> fixed. Detail in [gui-verification-pass.md](gui-verification-pass.md).

**Run this first**, so the queue starts from what is actually unverified:

```sh
python3 tools/audits/gui-pass-staleness.py
tools/testing/preflight-seats.sh     # seats AND which database the servers use
```

**A punching bag, for any row about proc rates or animation:** `@monster 2410`
spawns "Lv 100", one of RO's own training dummies — immobile, never attacks,
99,999,999 HP. Ids 2408/2409/2410/2411 are the Lv 10/50/100/150 set.

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

Run this **before logging in**, and read its first three lines before its table:

```sh
tools/testing/preflight-seats.sh      # exits non-zero if anything is wrong
```

It reports the job, position, party and instrument for both seats — and, first,
**which database it read them from**, resolved from what the running servers are
actually connected to rather than from a config file.

**Why it leads with the database.** On **2026-08-16** a session was brought up
against a leftover `korangar_integration_*` database. A killed integration run
had left its overrides installed in `Hercules/conf/import/`, so every server
silently pointed at a disposable database while the seats were being read out of
`ragnarok`. The report looked healthy; the characters on screen were two Novices
from a test fixture, and nothing done in that session would have counted. The old
version of this check — a bare `mysql … ragnarok -e …` — could not see it,
because it named the database it wanted rather than the one in use. If the script
reports an override, stop and clear it; the runner reclaims orphans now, but only
when it runs.

The seats also **drift on their own**. They had by **2026-08-12** (`test` a
**Whitesmith in geffen**) and were still drifted on **2026-08-16**, with both
instruments sitting unequipped in the bag and **no party at all** — party 280 is
long gone. §1 needs a Clown and a Gypsy in one party, so it stalls on the first
row, which is part of why Hermode keeps not getting reached.

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

**Self-cast toggle**, `PA_GOSPEL` **369**, 60 s, 10 s interval = **6 rolls**.
Re-cast to cancel. **It strips your own buffs when it starts.**

> **DONE 2026-08-16 — PASS after three fixes.** Kept for re-walks. **Two setup
> traps:** Gospel **excludes its own caster** (`ss == bl` breaks first), so the
> Paladin gets no buffs and no messages — testing it needs a **second seat
> standing in the field**. And only **ten of thirteen** effects announce
> themselves; heal, Blessing and Increase AGI send no packet by design, so a
> silent buff is not a missing message.

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

## 6. Fog Wall — DONE 2026-08-17

`@jobchange 4017` (Professor), `@allskill`.

- `PF_FOGWALL` **404**, ground target, range 9, 25 SP, 20 s.
- **No gemstone needed on this server** — official RO wants a Blue Gemstone and
  this build does not. Do not go shopping.

> **PASS after four passes.** It drew 15 opaque **black squares** first: RO has
> two texture families and the ground-decal pass only knew one. `lens_w.bmp` is
> greyscale-on-black (0% magenta, 40% near-black) and has to be *added*, not
> alpha-blended. Then the artwork itself proved wrong for a field — a 32x128
> gradient reads as three hard stripes — so it now draws the original's own
> `fog1/2/3.tga` cycling at 4 fps, per-cell phase offset, with a light.
>
> **It also caught Land Protector**, which has the same additive texture and had
> been alpha-blended since it shipped. **That row is now stale — re-walk it.**
>
> **Two things to carry forward.** The `[skill-unit]` log no longer prints
> `transparent=`: that field is hard-coded `false` for every BMP and could never
> have answered anything. It prints `blend=` instead. And when counting cells on
> screen, **count blocks, not the seams between them** — this row was reported as
> 18 squares twice against a server that sends 15, and the screenshot showed six
> seam lines with five cells between them.

## 6b. Land Protector — re-walk, caused by §6

`@useskill 288 5 <your name>` (`SA_LANDPROTECTOR`, 288). Any job.

| Look for | Verdict |
|---|---|
| The blue magic circle reads brighter, with no dark backing under the pattern | PASS — the additive fix landed |
| The circles blow out to white where they overlap | FAIL — additive is accumulating too hard; say how many cells deep it goes |
| Unchanged from before | Worth saying — it would mean the dark backing was never visible against this terrain |

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
