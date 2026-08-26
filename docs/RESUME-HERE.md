# Resume here — live pass status

> **2026-08-25 — the campaign's hunting layer is now item turn-ins, and the
> client finally has a quest log. NOTHING BELOW IS LIVE-VERIFIED.**
>
> All 41 non-boss Seal Cascade hunts ask for drops instead of kills. Master data
> is `Hercules/db/dm_hunt_db.json`; `Hercules/tools/gen-hunts.py` derives the
> rates and turn-in counts and writes all three artifacts, including this repo's
> `korangar/src/world/library/campaign_quests.tsv`. Do not hand-edit any of the
> three — `tools/check-campaign.sh` fails on drift.
>
> **The quest log had no UI at all.** `QuestAdded` / `QuestRemoved` / `QuestList`
> were registered and then discarded with empty match arms, and
> `HuntingQuestUpdateObjectivePacket` is still a `register_noop`. So every quest
> in a 19-arc campaign was invisible outside NPC dialogue. There is now a quest
> log window (Ctrl+Q or the menu) that lists each contract's items and how many
> the player is carrying, counted live off the inventory rather than cached.
>
> **Hercules never sends item requirements** — the quest packets carry kill
> objectives only, and a converted contract has none. The requirement list is
> bundled from the server's own master file rather than added to the wire
> protocol, the same way `hercules_item_names.tsv` is.
>
> **Formatting CI was already red on this branch** before any of this: five files
> from the 08-17/08-18 security and Windows-pack work fail `cargo fmt --all
> --check` under the pinned nightly (`archive/native/mod.rs`,
> `gamefile/mod.rs`, `towninfo.rs`, `item_info.rs`, `ragnarok-packets/lib.rs`).
> `formatting.yml` checks out to the workspace root, so it *does* pick up
> `rust-toolchain.toml` — this is real drift, not the stable-rustfmt phantom.
> Fixed here.
>
> **What needs playing:** a contract offer, a hand-in, the standing ledger, and
> the quest log window against a live inventory. Also worth watching: quest
> drops roll per party member in range, so a party should see them pool.


> **2026-08-18 — remediations landed** for the four security passes. C1 is
> rotated: published admin passwords no longer work, `headless2` is group 0,
> and the tester no longer ships a default password. Live creds are in the
> gitignored `Hercules/conf/import/operator-credentials.conf`. Restart
> login/char/map/api to pick up the C and config changes. H2 (cleartext
> protocol) and M4 (optional plaintext remember-password) stay accepted.

> **2026-08-17 — §6, §6b and §7 all PASSED. The ground-field family is CLOSED**,
> after being open since 2026-08-08 — and the two rows found **three** bugs that
> none of them were aimed at.
>
> **Start here:** [plans/gui-session-runsheet.md](plans/gui-session-runsheet.md).
> **THE RUNSHEET IS COMPLETE — §1 through §9 all DONE.**
>
> **§9 N24 PASS on both seats, and it did not hard-lock** — the short map name
> (`izlude`) dodged the Hercules truncation that cost the 08-05 session. The
> window names the *label* `dm_console.txt` builds, **"Seal Cascade - izlude"**,
> not the map. Its timer had **two** bugs from one design: the label was built
> once on join and cached, so it first showed **the Unix clock** as a duration
> ("496397h 45m" — Hercules sends `now + value` and `clif_instance_join` sends it
> raw, so both timers are **absolute timestamps**), and then, once corrected,
> sat frozen. It now re-renders against the wall clock, gated so a session with
> no instance open never pays for it.
>
> **§8 both PASS** — M1-009 showed the vs-equipped comparison, and M1-014's
> two-step delete works but was **unreadable**: the confirmation is an overlay
> over the character-select art and `text!` draws glyphs with nothing behind
> them. It is the last thing between a right-click and a destroyed character, so
> it now draws its own plate (`WarningBanner`) in red. **Text on an overlay needs
> its own plate** — nothing stops the next one being written with a bare `text!`.
>
> **Moonlit is DEFERRED by decision (2026-08-17), not cleared.** Its note had
> been drawing upside down since the 08-08 pass that closed the row; today's fix
> corrects it but nobody has seen it. Re-walking needs the Clown/Gypsy pair
> rebuilt and the combo is judged unlikely to be played. `gui-pass-staleness.py`
> deliberately still reports Block E as STALE, with the reason in its comment.
>
> **§1–§7 were DONE earlier today.** What is left is **§8** (the two confirms that shipped
> 2026-07-22 and were never looked at) and **§9 the instance window**, which
> stays **last because it can hard-lock both clients**.
>
> **EVERY GROUND DECAL WAS DRAWING UPSIDE DOWN**, and only Evil Land could show
> it. The camera sits at -z looking toward +z (`DEFAULT_ANGLE` 180,
> `CAMERA_PITCH` -55), so screen-up on the ground is +z, but the UVs put the
> picture's top on the -z corners. Nothing revealed it because nothing
> asymmetric had ever been drawn flat — Gospel's cross is symmetric, Land
> Protector's circle radial, Fog Wall's puff a blob. **Moonlit's note had been
> inverted since 2026-08-08, through a live pass that closed its row.** The
> convention is now one constant, `GROUND_DECAL_TEXTURE_COORDINATES`, with the
> camera derivation pinned in a test.
>
> **All three of Gospel, Fog Wall and Evil Land shipped with no light**, and all
> three needed one. That is now a rule worth applying before walking a new
> ground unit rather than after: **check for a `light` first.** Radius 9 for a
> small field — Fog Wall's 22 lit twice the area of its own effect.
>
> **RO has two effect-texture families and the ground-decal pass knew one.** Fog
> Wall drew 15 **opaque black squares**: `lens_w.bmp` is greyscale-on-black (0%
> magenta, 40% near-black) — additive artwork, where black means *add nothing* —
> and the pass hard-coded `ALPHA_BLENDING`. There are now two pipelines and a
> `GroundDecalBlend` on every decal, with instances stable-partitioned by family.
>
> **The same defect was sitting in Land Protector, verified back on 2026-07-24.**
> `aaa copy.bmp` is 0% magenta, 45.7% near-black, alpha-blended since it shipped,
> so all 121 cells at Lv5 laid a dark backing under the magic circle — very
> likely why its light needed three passes to stop reading as dim (26 → 40 → 30).
> **Measure every sibling when you find a bug of this class, not just the one
> that failed.**
>
> **Then the artwork was wrong for a field, which no blend could fix.**
> `lens_w.bmp` is a 32×128 one-dimensional gradient, so flat on a cell it paints
> a bar and the wall read as three hard stripes. It now draws the original's own
> **`fog1/2/3.tga`** — three frames of one soft puff, real alpha channel —
> cycling at 4 fps with **each cell offset by its own phase**, so fifteen cells
> boil instead of flickering in unison. Recipes take a frame list and an fps now.
>
> **Traps this row cost time on:**
> - **The `[skill-unit]` log's `transparent=` field was a lie** for every BMP —
>   `load_texture_data` only measures TGAs and hard-codes `false` otherwise — and
>   the runsheet told people to read it. It prints `blend=` now.
> - **Count blocks, not seams.** The field was reported as 18 squares twice
>   against a server that sends 15 every time; profiling the screenshot showed
>   six seam lines with five cells between them. Two wrong explanations came from
>   trusting the count over the wire.
> - **`tools/grf_extract.py` cannot read these textures** (DES-encrypted). The
>   `grf_extract` ignored test in `lib.rs` uses the client's own reader:
>   `KORANGAR_EXTRACT='data\texture\effect\fog1.tga' KORANGAR_EXTRACT_OUT=/tmp/x
>   cargo test -p korangar --lib grf_extract -- --ignored --nocapture`.
>   Measuring the pixels is what settled every question here.

> **2026-08-16 — the GUI pass finally moved, and five rows closed in one sitting.**
>
> **Start here:** [plans/gui-session-runsheet.md](plans/gui-session-runsheet.md).
> **§1–§5 are DONE.** Next is **§6 Fog Wall**, then §7 Evil Land, §8 the two
> confirms, §9 the instance window (**last, it can hard-lock both clients**).
>
> | Row | Result |
> |---|---|
> | §1 Hermode | **PASS — the oldest open row, never once reached before today** |
> | §2 Party roster + trade | PASS both halves; Block A is no longer stale |
> | §3 P1/P5/F3/F4 | **4/4 PASS**, and found a real bug on the way |
> | §4 Auto Spell (N20) | PASS end to end, proc observed |
> | §5 Gospel | PASS after three visual fixes **plus a dropped feature recovered** |
>
> **The session did not start with testing — it started with the environment
> lying.** The servers were running against `korangar_integration_9784`, a
> disposable database left by a run killed on 08-12. The seats were read from
> `ragnarok` and looked healthy; the characters on screen were two Novices from a
> fixture. **Nothing done in that session would have counted.** `cleanup()` hangs
> off an EXIT trap and no trap survives SIGKILL — and worse, `restore_configs()`
> would back up the *previous run's artifacts* as "the developer's original" and
> faithfully restore them, so **one `kill -9` cemented the override permanently**.
> Fixed by sweeping at startup (`reclaim_orphans`), and
> **`tools/testing/preflight-seats.sh`** now resolves the database from the
> running servers' own connections instead of naming the one it hopes for.
>
> **Four product bugs, three of them invisible to any automated test:**
> 1. **`ZC_GOSPEL_INFO` (0x0215) silently dropped** for the client's whole life —
>    a party under Gospel receives a major effect every 10s and was told which by
>    nothing. The fallback consumed it *cleanly*, so it never reached the
>    unknown-packet ledger.
> 2. **Dismissing the party-invite popup sent nothing**, stranding
>    `tsd->party_invite`, after which every later invite told the inviter
>    **"<name> is already in a party"** — false, and false until relog. The
>    friend-request popup had the identical defect. Both `closable: false` now.
> 3. **Gospel had no light at all** while 17 other units do, so its greyscale
>    cross read as **grey metal**; and its α 0.05 tint was **completely
>    invisible**, confirming 0.05 is dead in this renderer.
> 4. **Hermode's warp-portal refusal sent bare cause 0** — "skill level is not
>    high enough" for standing in the wrong place — fifteen lines below a site
>    that already had a real reason.
>
> **Two deliberate fork deltas, at the user's decision** (CLAUDE.md §3b): Hermode
> is no longer banned in the Normal zone, and its warp requirement sits behind
> `hermode_requires_warp` (default **off**). It was **dead content** here — the
> only zones permitting it were towns and sieges, neither of which this campaign
> plays.
>
> **What kept being true all night: most things that looked like failures were
> correct.** An empty status window twice (`SC_HERMODE` and `SC_GOSPEL` have no
> `Icon:`, so Hercules never sends the state change), a frozen caster
> (`SC_DANCING` roots for ~31s), no damage from Gospel (it debuffs), no message
> when a friend is removed (upstream sends `0x020A` and no text, and says so in a
> comment). **Check what the server is supposed to send before calling it a bug.**
>
> **Traps that cost time tonight, so they do not again:**
> - **The ensemble partner needs the instrument equipped too**, not just the
>   caster (`skill.c:15402`). Invisible in the DB mid-session; visible in the
>   client's `[packet-log] local equipped weapon=` line.
> - **`inventory` is a save snapshot and disagreed with live state by a whole
>   trade.** Read `picklog`.
> - **RO ships training dummies**: `@monster 2410` ("Lv 100") is immobile, never
>   attacks, 99,999,999 HP. Far better than porings for proc rates and animation.
> - **Gospel excludes its own caster** (`ss == bl`), so testing its buffs needs a
>   second seat standing in the field.
>
> ---
>
> **2026-08-12 — where to go:**
>
> | Track | Status | Open first |
> |---|---|---|
> | **Headless suite** | **Acceptance closed** (147/1/0 full green; empty exemptions; golden 1–10; quest-log-multi; PR multi-scenario CI). HEAD `e4d6e6d5`+ | [../tools/testing/headless-next-steps.md](../tools/testing/headless-next-steps.md) |
> | **GUI live pass** | **In progress** — Hermode, Auto Spell (N20); open-only table at top of file | [plans/gui-verification-pass.md](plans/gui-verification-pass.md) |
> | **Meaning of green** | Wire ≠ pixels | [plans/testing-completeness.md](plans/testing-completeness.md) |
>
> Do **not** grind more headless scenarios for allowlist culls, cast-only skill
> expansion, or duration jitter. Do **not** treat headless-green as client-verified.
>
> ---
>
> **2026-08-09 — the day turned into a rebuild of what the suite MEANS.** It
> started as the one unfinished job (re-run `--scenario all`) and became: the
> suite is **136 scenarios**, all 11 fork deltas are guarded, the campaign's 103
> story beats execute for the first time, and three standing audits now enforce
> *classify, don't silence*. Plan: **[plans/testing-completeness.md](plans/testing-completeness.md)**.
>
> **THE THING TO INTERNALISE, because nine of the day's findings were bugs in the
> TESTS rather than the product:** green did not mean what it looked like.
> Measured across a full run — 36% of skill observations were the server
> *refusing* the skill, 26% were passive skills never cast, 43 of 81 allowlist
> entries were dead weight that would absorb a real regression in silence, and
> the matcher stops at the FIRST event it recognises so `cast` means "a cast bar
> started and we stopped looking". The run now prints this distribution and says
> in words that **a green sweep means the wire is alive, NOT that the skills
> work**.
>
> **VERIFIED GREEN BOTH WAYS (2026-08-10):** full run **135 passed / 0 failed /
> 1 skipped**, and shuffle seed **20260810** the same — the seed that had found
> four defects. 22 commits validated by both.
>
> **THE INTERMITTENT CLUSTER IS SOLVED, AND IT WAS TERRAIN.** Five "silent
> skills" and four "3-6x slow scenarios" were **one** root cause. `sweep_job`
> anchored each job with `warp_random("prt_fild08")` and fired fixed ground
> offsets around wherever it landed; a draw near trees or water put target cells
> on unwalkable terrain, and Hercules drops an unplaceable ground cast with a
> bare `return 0` and **no `clif->skill_fail`** — silence indistinguishable from
> a lost packet. That explains every property it had: intermittent, a different
> job each run, never reproducible in isolation, only ever ground/trap skills.
> **Four earlier theories (cell collisions, radius-3 out of range, unit
> accumulation, the ring size) were all plausible and all wrong**; the
> instrumentation settled it in one run by printing anchor (161,246),
> `HT_BLASTMINE -> (164,246)` SILENT while (161,249) and (164,249) placed fine.
> Same lesson as `observer.rs`'s CAST_VENUE one level down: **choose the ground,
> do not inherit or randomise it.**
>
> **What the shuffle found that a green full run could not** — it was 135/136
> green on the same code while the shuffle was 131/136:
> - `friend_reject` and all three trade scenarios were clean only by *position*;
>   their `*_lifecycle` siblings self-clean and they did not.
> - **Trading needs Basic Skill 1 and party creation Basic Skill 7**
>   (`clif.c:13262`, `clif.c:14633`, `basic_skill_check: true`). Hercules refuses
>   with `skill_fail(skill_id 1, cause 0)` and **returns without acting**, so no
>   `TradeRequest` and no `CreatePartyResult` are sent at all. Any job change
>   wipes skills; only the sweeps restore them, and natural order always happens
>   to run one first.
> - `create_party` waited only for `result: 0`, so a *refusal* read as silence.
>   It now reports the code.
>
> **Two wrong turns worth not repeating:** "User 'korangar' is already online -
> Rejected" in the server log looks damning and is **baseline noise** — ~135 per
> run, once per scenario, the ordinary login-retry path. And the evidence that
> killed that theory (the partner's context receiving `EntityMove` throughout)
> was already in the failure output before the server log was ever opened.
>
> **Still open:** `dm-beat-table` / `dm-story-beats` **cannot run standalone** —
> they fail at `@dm reset confirm` and only work after earlier scenarios set
> something up.
>
> **New tools, each of which caught something the day it was written:**
> `tools/audits/flaky.py` (cross-run inconsistency — found a 523s scenario nobody
> had noticed), `tools/audits/event-routing.py` (server events the client
> discards — found the quest system), `tools/audits/packetver-variants.py` (stale
> header variants), and `tools/generate_skill_expectations.py` (664 derived
> expectations, **deliberately not enforced** — validated against real runs it
> would redden 217 working skills).
>
> **The quest system is the largest dropped feature in the tree**, found twice
> independently: the campaign registers 78 quest ids, grants them via
> `setquest()`, and gates 290 dialogue branches on `questprogress()` — and
> `korangar/src/lib.rs` discards `QuestAdded`/`QuestList`/`QuestRemoved` in three
> empty match arms. A DM hands out a quest, the server records it, the gates
> work, and the player never sees any of it. **That is the GUI session's first
> job.**

<details>
<summary>2026-08-09 (earlier) — the re-run that started it</summary>

> The previous session's one unfinished job is closed: `--scenario all` is green,
> and **all 11 fork deltas now have a guard** (three had none, including the most
> valuable find of 08-08).
>
> **The re-run's question got a definite answer, and it was the opposite of the
> one expected.** The two new observer scenarios do **not** disturb the other
> 123 — those were intact even in the failing run. The suite's shared state broke
> the *new* scenarios instead, and the recorded "8/8 in `--scenario phase11`" was
> **luck of position, not a pass**:
> - `HeadlessTwo`'s **save point is `int_land`**, the renewal start point it was
>   created on. `dm-instance-lifecycle` ejects it there when the instance closes,
>   two scenarios before phase 11, and `connect_pair` then meets on the beginner
>   island for the rest of the run.
> - The six look rows do not care where they stand. The two casting rows aim
>   blindly at `x+2`, `int_land(80,101)` cannot take a field, and Hercules drops
>   an unplaceable ground cast with a bare `return 0` and **no** `clif->skill_fail`.
>   Total silence. Measured: **8/8 on `prt_fild08`, 6/8 on `int_land`.**
> - **Second instance in that one file of the class it already documents** — a
>   constant assumption resting on a position that is shared mutable state.
>   `far_map_from` fixed the meeting *map*; nothing covered the target *cell*.
>
> **`connect_pair`'s "non-GM partner" comment was FALSE** and `skills.rs`
> contradicted it in the same binary (login table: **both accounts are group
> 99**). That false premise is *why* the meeting place is inherited rather than
> chosen. **Recommended next change:** have `connect_pair` warp *both* seats to a
> chosen venue — the partner can warp itself. Do it as its own change with a full
> run plus a fresh shuffle seed, since ~16 paired scenarios rest on it.
> **Do NOT "fix" this with `@save` in `normalize()`:** `@save` pins the *current*
> position, so run after drift it would make `int_land` the **permanent** save
> point, and it adds a global shared-state write to a suite whose entire bug
> history is global shared-state writes.
>
> **Four new guards, each proved able to fail on the exact regression it
> protects** — `cast-cancel` (`CZ_CANCEL_CAST` 0x0F00, whose failure mode is
> **right-click disconnecting you to login**, and whose only previous tests used
> bytes we assembled ourselves), `item-command-multi-word` (the `@item` parser
> delta, both halves), `land-protector-status` (`SC_LANDPROTECTOR`, 5 sites), and
> `kick-explains-itself` (map-server `SC_NOTIFY_BAN` 0x0081 — verified by eye once
> on 08-08 and guarded by nothing since).
>
> **THE LESSON OF THE DAY, and it was self-inflicted: a new guard introduced the
> exact hazard this file warns about.** `land-protector-status` cast at the shared
> venue, and Land Protector at level 1 stands for **165 seconds** over a **7×7**
> area whose whole purpose is suppressing ground magic — covering cells 285-291,
> i.e. the meeting cell everything else uses. Two minutes downstream `AL_PNEUMA`
> went **SILENT** in the Acolyte sweep. It was latent from the moment it was
> written; the run before it passed only by landing just outside the window, and
> it surfaced only because a *fourth, unrelated* scenario shifted the timing.
> **A single green run does not clear a scenario that leaves state behind — and
> "walk away" is not cleanup, because the field is what harms the next scenario,
> not your position in it.** Fixed by giving it `prt_fild01`, which nothing else
> visits, and by casting on the caster's own cell (Land Protector carries only
> `UF_PATHCHECK`, **no `UF_NOFOOTSET`**, so it places under the caster — which
> also removes the guessed-neighbour-cell fragility).
>
> **Ledger:** `0x0078` is explained and can be retired from the modeling
> backlog — it is `clif_sendfakenpc`, an **invisible** placeholder (class 111) so
> a script dialog has an owner. Two new entries appeared once `@kick` was
> exercised: **`0x00CD`** (`clif_GM_kickack`, the kicker-side ack — documented,
> not a bug) and **`0x0191`**, previously only ever seen under a shuffled run.
>
> **Still unverified, and needing ears rather than tests:** the audio work below.
> Ambience and skill-sound levels both changed substantially, and no one has
> heard either.
>
> **Both test characters need explicit re-jobbing** — `test` is Sage 16 and
> `HeadlessTwo` is Priest 8 at the restored snapshot. Hermode wants Clown 4020 +
> Gypsy 4021 with the whip (1950, still in inventory).
>
> **Next, cheapest first:** the `connect_pair` venue change above; then the
> untested modelled packets (`AutoSpellList` via Sage `SA_AUTOSPELL`,
> `SpiritSpheres` via Monk `MO_CALLSPIRITS`, `EntitySnapped` via Champion
> `MO_BODYRELOCATION` are the cheap three — note the job sweeps already *cast*
> these, so what a scenario adds is asserting the specific event, not the wire
> path); then **GUI Block E**, still the largest open item.

> **2026-08-08 (evening) — a two-seat live pass. Six bugs, five checks passed,
> everything pushed.** None of the six was reachable from the headless suite,
> which links `ragnarok-packets` and `korangar-networking` and **not**
> `korangar/src`. Two of them needed a *second seat* to exist at all.
>
> **PASSED, and these close long-standing debt:**
> - **`ZC_ADD_SKILL` (0x0111)** — `@questskill 1014` put Redemptio in the skill
>   tree live. The path no test can reach.
> - **`ZC_SKILL_FAIL_REASON` (0x0EFE)** — casting Redemptio with no party named
>   the *experience* condition out of its three cause-0 paths.
> - **The reuse-delay message** — an Yggdrasil Berry used twice inside five
>   seconds printed *"%d seconds left until you can use"*. That single line
>   confirms three separate pieces of the 07-08 work at once: the `ZC_MSG_VALUE`
>   (0x07E2) channel, the generated message table, and the `messages_main.h`
>   gloss realignment. It read *"Content has been saved in [SaveData_ExMacro5]"*
>   before the fix.
> - **`@kick`** now explains itself, and the berry heal shows a number.
>
> **FIXED:**
> 1. **No Gypsy could log in.** `SkillListRequirements::get` panicked
>    (`incomplete skill tree`) *before the character reached the screen*.
>    `CG_LONGINGFREEDOM` (487) is in `skilltreeview.lub`'s Gypsy tab and absent
>    from `skillinfolist.lub`. **Two GRF tables are allowed to disagree**, and
>    the sibling built from the same file has always degraded gracefully — so the
>    skill passed the first lookup and detonated on the second.
> 2. **Moonlit's white bloom.** One point light *per cell*, 81 on a 9×9. The
>    recipe's "keep it dim" hedge could not work: **that number is a radius, not
>    a brightness.** Overlap decides the hue — a saturated colour saturates as
>    its hue, a pale one goes white, which is why even blue came back white.
> 3. **`@kick` bounced you to character select in silence** — and this one took
>    **four** rounds, each at a different stage: routed to chat (destroyed with
>    the screen), then opened in the wrong event arm (ordering not ours to
>    choose), then wiped by one of the **three** `close_all_windows` calls on the
>    way back, and finally **drawn underneath character select**, because windows
>    render in open order and the interface has no raise-to-front. See the new
>    failure mode below.
> 4. **"Rejected from Server"** had the same draw-order problem.
> 5. **No numeric HP or SP anywhere** in the client. Now in the HUD.
> 6. **Base EXP looked missing at max level** — the server sends a next-level
>    requirement of 0 and the zero branch printed a bare number.
>
> **A NEW FAILURE MODE, and the most transferable thing here.** "The data arrives
> and nothing displays it" had three recorded instances and **all three stopped
> at the handler**. The kick was a fourth stage: correct packet, correct handler,
> correct living window — **behind another window**. When a message is missing,
> the stages are wire → handler → surface lifetime → **draw order**, and only
> instrumenting each in turn found it. Three rounds of theorising lost to it.
>
> **Ordering audit (asked for after the kick).** This fork has three companion
> packets — a small packet sent immediately before the thing it explains.
> `0x0EFE` and `0x0EFF` are both **sound**: packet↔packet pairings on one
> connection are safe by construction, since TCP preserves order and the handler
> dispatches sequentially. `0x0081` broke because it pairs a packet with a
> **connection-level event**, produced by the socket layer on a different clock.
> **Packet↔lifecycle pairings are the fragile class, and their bugs surface in
> the UI rather than on the wire.**
>
> **Not a bug, now settled:** the "static / blowing on a mic" on one seat is
> `prt_fild08`'s own ambience — `se_moc_wind_little.wav` and
> `se_prtthewaterofabrook.wav` are placed in the RSW. Spatial, cyclic, and
> loudest at the source, exactly as reported. The two-client phasing theory was
> wrong; it was position, not client.
>
> **Moonlit is redesigned, deliberately.** The α 0.6 salmon tile is *faithful*
> and was confirmed correct — and 81 full-coverage tiles read as a slab whatever
> colour they are. The tint is now **off**, the colour moved into the light, and
> each cell carries a bobbing note. **`UnitBody::LayeredGroundQuad` is built**,
> which unblocks `PA_GOSPEL`, `PF_FOGWALL` and `NPC_EVILLAND` — all three are
> wired with the table's verbatim colours, **none has been seen on screen**, and
> their hover sizes and opacities are estimates, not table values.
>
> **CAUTION on the α 0.6 calibration this repo records as settled.** That reading
> was taken while the tile still borrowed Land Protector's *texture* as a
> carrier, whose artwork was cutting the coverage. The flat-colour change removed
> that filter and α was never re-derived. It has since held up on a genuinely
> flat tile — but anyone porting the song/Gospel/Fog-Wall family at
> "0.6-magnitude" should know the number was measured under conditions that no
> longer exist.
>
> **Next, cheapest first:** **Hermode** is the last open Block E row — but
> `HeadlessTwo` is now a **Priest (8)**, because the Redemptio checks jobbed it
> away from Gypsy. **Order the ensemble rows BEFORE the cause-0 checks in
> future**: the latter destroy a setup the former needs and which is far more
> expensive to rebuild (`@jobchange 4021`, `@allskill`, equip the whip — item
> 1950 is still in its inventory). Then `@jobchange 4015` + `@allskill` for
> Gospel, the first live look at the two-layer body.
>
> **Test-environment note that cost real time:** `inventory` and `char` are stale
> for a live session — `picklog` is not. It proved the berry was being consumed
> all along and that both attempts were 6 s and 15 s apart against a 5 s delay,
> i.e. the "broken" item was working and the test never triggered the condition.

<details>
<summary>2026-08-08 (earlier) — the cause-0 rebuild (superseded above)</summary>

> **2026-08-08 — the cause-0 work is finished, suite-verified, and STILL NOT ON
> SCREEN.** The row-5 failure ("skill level is not high enough" from a Clown with
> the skill at max — it meant *no ensemble partner*) turned into an audit of every
> channel by which the server reports a failure, and then into a rebuild of how
> that reporting works. See
> [protocol/server-error-channels.md](protocol/server-error-channels.md).
>
> **The finding that reframed it:** cause 0 is not laziness in Hercules, it is the
> protocol's fallback for conditions Gravity never numbered — 21 of the 33 states
> in `skill_check_condition_castbegin` report cause 0 and only **one** has a
> dedicated cause; `skill.c` alone has 174 cause-0 emissions. The official client
> says "Skill level is not high enough" to all of them too. So there is no "send
> the right cause" fix. What there is, is a split:
> - **static preconditions** (`State:` in skill_db — shield, falcon, cart, stance)
>   are on disk at both ends, so nothing needs sending;
>   `tools/generate_skill_states.py` carries them across (42 skills, 13 states);
> - **runtime outcomes** (roll missed, nobody in range, not enough experience, no
>   partner) only the server knows, and now ride **`ZC_SKILL_FAIL_REASON`
>   (fork packet 0x0EFE)** — see CLAUDE.md 3b for the five touch points.
>
> **Two bugs were found by writing the guard rather than by auditing.**
> `ZC_ADD_SKILL` (0x0111) was **never modelled**, so any skill *granted* rather
> than levelled was eaten by the length fallback — Plagiarism's copied skill, quest
> rewards, and anything a campaign script grants, all invisible until relog. And
> the reason field was first modelled as a `ByteConvertable` enum, which fails to
> deserialize on an unknown value and **discards the whole read buffer** — in a
> packet whose enum is documented *append only*, i.e. the intended way to extend it
> was the trigger. Both fixed and guarded.
>
> **Suite: 124 scenarios, green twice** — ordered, and shuffled (seed 20260808),
> 123 passed / 0 failed / 1 structural skip, 0 packet deserialization failures.
> The shuffled pass matters because all scenarios share one character and the
> double-run gate cannot see order dependence.
>
> **What the suite CANNOT tell you, and what to do first on screen.** The headless
> tester links `ragnarok-packets` and `korangar-networking` — **not** `korangar/src`.
> So `NetworkEvent::SkillAdded` → the Skill Tree window is new, unreached code.
> One action each, most valuable first:
> 1. **`@questskill 1014` as a Priest — does Redemptio appear in the skill tree?**
>    The only path no test can reach, and that window is where the M1-017 logout
>    crash lived.
> 2. **Use an Yggdrasil Berry twice inside five seconds** — the reuse-delay message
>    ("%d seconds left until you can use"), which read "Content has been saved in
>    [SaveData_ExMacro5]" before the Hercules gloss fix.
> 3. **Cast Redemptio with no party** — the 0x0EFE reason text as a chat line.
> 4. **`@kick` from a second seat** — the map-connection `SC_NOTIFY_BAN`.
>
> **Block E is still open and is still the actual next task**: Moonlit's α 0.6 is
> confirmed correct (which unblocks the whole song/Gospel/Fog-Wall family), but its
> tile then drew as a solid red square after a fix made during the walk, and **the
> instrumentation log that would settle it in one cast has never been read**. Rows
> 4b and 5 of [plans/gui-verification-pass.md](plans/gui-verification-pass.md).
> Do this in the same session — the client has to be launched for all of it.

</details>

<details>
<summary>2026-08-07 — the error-channel audit that started it (superseded above)</summary>

> **2026-08-07 — a whole batch of error-message work is pushed and NOT live-verified.**
> Blocks D and E of [plans/gui-verification-pass.md](plans/gui-verification-pass.md)
> ran, and the row-5 failure ("skill level is not high enough" from a Clown with
> the skill at max — it meant *no ensemble partner*) turned into a full audit of
> every channel by which the server reports a failure. Four channels were
> completely silent, 25 skill-fail causes printed "Skill failed (reason 73)", and
> the message table the client resolves ids through was **wrong often enough to
> invert a success into a failure**. See
> [protocol/server-error-channels.md](protocol/server-error-channels.md) for the
> channel map, the four sweeps needed to find them all, and the short list of
> live checks — `@kick` from a second seat is the most valuable single action.
>
> **Block E is still open**: Moonlit's α 0.6 is confirmed correct (which unblocks
> the whole song/Gospel/Fog-Wall family), but its tile then drew as a solid red
> square after a fix made during the walk, and **the instrumentation log that
> would settle it in one cast has never been read**.

</details>

**Session 2026-07-28/29 closed a lot.** The observer-view checklist
([plans/observer-view-verification.md](plans/observer-view-verification.md)) is
**rows 1-5, 7, 8, 9 PASS**, row 6 retired as unobservable, **rows 10-11 open**.
That pass found **six real bugs**, all invisible to single-client testing:

1. Login `LOOK_AMMO` broadcast issued before `map->addblock` — reached nobody
2. Client dropped the ammunition packet when it arrived pre-spawn
3. `AddEntity` rebuilt the entity and wiped the applied value
4. Cached ammunition survived a weapon change — arrows drawn from a revolver
5. **Every remote player rendered with hair style 1**, always, since `Common`
   passed `head: None` and `set_hair` was gated on `Entity::Player`
6. Disguise zeroed `vd->ammo` server-side and un-disguise never restored it

All six fixed, live-verified on two clients, and pushed. The bug *classes* are
generalised into runnable audits in
[plans/observer-parity-audits.md](plans/observer-parity-audits.md) — which
themselves found two latent issues (weapon/shield broadcast before `addblock`,
and a re-send guard that turned out to be harmless) and one live one (item 6).

**A programme plan now sits on top of those audits:**
[plans/observer-parity-harness.md](plans/observer-parity-harness.md) (2026-07-29).
It reframes the six bugs around the four boundaries state crosses between
`sd->vd` and a pixel, and it found three new live gaps by reading the no-op
list — **remote players' facing changes never reach observers** (`0x009C` is
`register_noop`), stop-move is ignored, and hat effects are dropped. Its headline:
the spawn packet already carries the **complete** appearance (hair colour,
clothes colour, all three headgear slots, robe, body style, emblem) and
`EntityData` drops all ten fields, which is why none of it renders. Read §4
before touching the harness — the ordering there is not optional.

**Harness phases 0 and 1 are DONE (2026-07-29), both written with the stack
down and NOT live-verified.** Phase 0 is the audit suite
(`tools/audits/observer-parity.sh` + [runbook](../tools/audits/README.md)).
Phase 1 closed the wire→event boundary: `EntityData` now carries the seven
appearance fields the spawn packet always sent, the `SpriteChangeType` match is
exhaustive behind a new `ChangeLook` event, and `ChangeDirection` / `StopMove`
produce real events instead of no-ops. Workspace tests green (253 + 21), five
new wire-level tests, audit open items 32 → 21.

**Those appearance fields are stored, not drawn** — sprite composition is still
body + head + weapon + shield. Rendering them is phase 4 and needs accessory
sprite paths and palette files this tree does not have.

**Phase 2 is DONE and LIVE-VERIFIED (2026-07-29): 6/6 PASS, double-run.**
Six scenarios assert on the *observer* across the five timings that generalise
the four `LOOK_AMMO` bugs (change while watched, while out of view, before the
observer logs in, across an entity rebuild, and a change to zero), plus a
disguise round-trip for audit A8.

```sh
cargo run --release --example headless-tester -p korangar-networking -- --scenario phase11
```

**It is `phase11`, not `--scenario observer`** — `--scenario` matches a scenario
*name* or `phaseN`, and "observer" is neither, so it silently selects nothing.

They were **proved able to fail**: deleting the `ChangeLook` tracking makes 4 of
6 fail. `observer-look-fresh-login` still passes without it, because it takes
everything from the spawn packet — which is exactly what that row is for.

## Test-suite validity work — shuffle pass DONE (2026-07-29/30)

The full suite is **114 scenarios**, not 107 or 125.

**A `--shuffle <seed>` detector is in the tester** (verified: same seed
reproduces, different seeds differ, no scenarios lost). All 114 share one
character, so order dependence is the suite's structural weak point, and the
existing double-run gate cannot see it — it runs the same order twice.

```sh
./target/release/examples/headless-tester --scenario all --shuffle 42
./target/release/examples/headless-tester --list --shuffle 42   # preview order only
```

**The shuffle pass is complete and it paid for itself: four order-dependence
bugs plus one real client bug.** Runs: seed 42 → 111/114, fixes → 112/114, fresh
seed 1337 → 113/114 (fixes generalised; one new bug), fixes → re-run.

**Correction to the old tally: the recorded "114/114 green" was never true.**
`skills-dancer` and `skills-gypsy` had been failing the whole time behind a green
count, because a skipped/mis-jobbed scenario reports as PASS (item 1 below).

The four order-dependence bugs, all fixed:

1. **`weapon-refine-missing-material`** — recorded as "root cause not yet found".
   It was never a refine bug: `skill-fail-rejection` drains SP to zero and never
   restores it, `WS_WEAPONREFINE` costs 5 SP, and `ensure_job` does not heal, so
   the drain survived the job change. Now restores SP on the way out.
2. **`observer-look-clear`** — `FAR_MAP` was the constant `"prontera"`, but
   `connect_pair` meets wherever the partner character was *last left*, and that
   position persists in the `char` table. Now `far_map_from(home_map)`. Three
   scenarios used `FAR_MAP`, so all three were latent.
3. **`skills-dancer` / `skills-gypsy`** — *not* order-dependent. The shared
   character is male, and **Hercules does not refuse a sex-mismatched job
   change**: `pc_mapid2jobid` (`pc.c:6465`) round-trips through the character's
   sex, so asking for Gypsy silently yields Clown and the server says "Your job
   has been changed." No failure message exists, so `sweep_job`'s
   gender-restriction skip guard could never fire. Both now route to the female
   partner character (`HeadlessTwo`, which also has GM rights).
4. **`incoming-damage`** — inherited whatever job the previous scenario left;
   after `skills-soul-linker` the provoked mob never retaliates. Passed in
   natural order only because `dm-warp-recall` precedes it there. Now normalises
   the job best-effort. **Two wrong hypotheses first** (a lethal provoking blow —
   the fallback never fired; then mob species — 1007 both passed and failed).
   Bisect settled it: failing state → normalise to 4008 → passes, nothing else
   changed. **Bisect, don't theorise.**

**The client bug the sweeps found once they could run:** `ZC_NOTIFY_MAPINFO`
(**0x0189**) was unmodeled, so `register_length_fallbacks` ate it and four
user-facing messages were dropped — cannot teleport here / save point cannot be
memorized / skill unusable here / item unusable here. Hercules sends this
*instead of* `clif->skill_fail` (`clif.c:6213` says so). Any skill in the map
zone's `disabled_skills` therefore did nothing at all, silently, on any non-PvP
map: `DC_UGLYDANCE`, `BD_ETERNALCHAOS`, `BD_ROKISWEIL`, `CG_HERMODE`,
`BA_DISSONANCE`, `DC_DONTFORGETME`. Fixed with a packet + handler + regression
test (`ee26fb23`). **Not yet seen in the graphical client** — the chat line is
wire-verified only.

**Also fixed later the same session** (details in
[tools/testing/headless_findings.md](../tools/testing/headless_findings.md)):

- **Skips no longer count as passes.** `Scenario` has a third outcome and the
  summary reads `N passed, N failed, N skipped, N known-fail`. `skills-novice` is
  the one legitimately permanent skip — the Novice tree is passive apart from
  quest-gated actives — which is why a skip must **not** fail the exit code.
- **The `BD_ETERNALCHAOS` / `BD_ROKISWEIL` allowlist entries are gone**; their
  stated reason was wrong (map-zone disabled, not ensemble).
- **Hercules' per-IP anti-flood was causing every "random disconnect" cluster.**
  `ip_rules` is now disabled in the Hercules tree — see CLAUDE.md §3b. **The
  diagnostic:** connection-flavoured failures *plus* `Packet coverage: … 0 failed`
  means environment, never protocol. Count `there is no char-server online`.
- **Ammunition accumulated until the character was overweight**, at which point
  `@item` fails and every item-dependent scenario breaks at once, far from the
  cause. `observer-ammo-disguise` now cleans up after itself. The existing
  backlog was purged by hand via SQL and **does not replay on a fresh database**.
- **Traps now prove they were placed** (`AddSkillUnit`), 12 assertions per run
  where there were previously zero.

**Suite hardening is COMPLETE (2026-07-31).** Four items, all validated by a
clean full run on an unseen seed:

1. **`./dev.sh snapshot` / `restore`** (Hercules tree) — rolls the database back
   after a run, so accumulation cannot build up again. Refuses while any
   character is online, and verifies the dump's completion trailer.
2. **`weapon-refine-success` uses real commands** — `@stat` is not a Hercules
   command, so two thirds of its documented flakiness fix never ran.
3. **`normalize()` at scenario start** — full HP/SP plus removal of stacking
   items, at the *start* so a failing scenario cannot skip it. It must **not**
   flush: the server delivers real state at login (`QuestList`), and flushing ate
   it.
4. **Unit-creating skills prove their unit** — 38 `ground-unit` assertions
   against zero before. `UNIT_CREATING_SKILLS` is a snapshot of `skill_db`'s
   `Unit:` blocks; **regenerate it when skill_db changes** (snippet in the const's
   doc comment).

**The standing inventory lives in
[plans/work-backlog.md](plans/work-backlog.md)** (assembled 2026-08-02) — live
verification debt, features that exist only as discarded data, suite depth, and
the server deltas a merge would silently drop. It also carries the four greps
that regenerate it, because a day of fixes came out of lists nobody was reading.

**Still open:**

- **`MG_FROSTDIVER` is a known intermittent** in `skills-super-novice`.
- **THE GUI PASS — STARTED 2026-07-31, one row closed.** Boundary 5 now has its
  first evidence. The queue, with setup, traps and a known-unrendered list (so
  phase-4 gaps are not logged as bugs), is
  **[plans/gui-verification-pass.md](plans/gui-verification-pass.md)**.
  - **Row 1 `0x0189` PASS** — `DC_UGLYDANCE` as a Dancer in Prontera printed
    *"This skill cannot be used in this area."* on screen. That closes the one
    genuinely new item from 31 July; the other three `info_type` values differ
    only by a string literal in the same `match`, so the packet, the handler and
    the chat path are all proven.
  - **Row 11's probe was invalid and has been corrected** — `AC_CONCENTRATION`
    has no opt1/opt2 state, so it has no entity visual for *anyone* and the row
    could neither pass nor fail. Use a Sage field (`SA_VOLCANO` 285 /
    `SA_DELUGE` 286 / `SA_VIOLENTGALE` 287). Full reasoning in both plan docs.
    **Generalise this: when substituting a skill into a verification row, check
    the substitute still carries the property the row tests.**
  - Rows 10, 3, 4, 4b, 5 and 6 remain open. Row 10 is cheapest — both seats are
    already provisioned and it needs no gear change.

- **Two client-launch facts worth not re-deriving.** The release binary goes
  stale silently: on 31 July it predated the `0x0189` fix, so the row would have
  reported a false FAIL against a build that never contained the code. **Check
  the binary's mtime against the commit you mean to test, and rebuild first.**
  Seat A is `cd korangar/korangar && ../target/release/korangar`; seat B is
  `cd client2 && <abs path>/target/release/korangar` (its `client/` uses absolute
  GRF paths and logs in as `headless2`).

- **`KORANGAR_PACKET_LOG=1` is not a full dump** — it prints only `impact`,
  `damage`, `add-entity`, `local equipped`, `look change` and unknown/failed
  incoming bytes. It logs **nothing for status changes or skill casts**, so its
  silence is not evidence that a cast did not happen. Confirm state against the
  `char`/`skill` tables instead.
- **Do not invest further in the suite.** Remaining ideas (parallel instances,
  per-scenario characters) buy speed and isolation, not trust, and would trade a
  known-good serial suite for new failure modes.

**Trap: never run a second tester instance while the suite is running either.**
All scenarios share one character *and* one partner character, so a concurrent
run corrupts both. Two headless instances in parallel would need two more GM
accounts plus their own characters, and the partner character name is currently
hardcoded (`context.rs`, `CHARACTER_NAME`), which is the actual blocker — RO
character names are globally unique.

**Trap that cost a whole 40-minute run:** do **not** run any `cargo` command
while the suite is executing. `cargo run` rebuilds
`target/release/examples/headless-tester` underneath the running process and
produces a cascade of bogus "disconnected" failures (14 of them), with a
perfectly healthy server. Invoke the built binary directly instead, as above.

**Still open, cheapest first, all needing the stack:** confirm the A9 stance bug
(two seats in a town, one armed — code-read only so far); then the 2026-07-26
batch leftovers below.

**Corrected 2026-08-08 — this list used to claim observer rows 10-11 were open.
They are not: both PASSED 2026-08-02** and
[plans/observer-view-verification.md](plans/observer-view-verification.md) states
that its checklist is closed, every row PASS or retired. The stale entry cost a
session's planning time. It also repeated the "`test` has the Archer skills"
claim that the GUI plan already flags as **false** — jobs and gear drift, so read
the checklist document itself rather than a summary of it, and set the job
explicitly every time.

What rows 10-11 *do* still lack is any **regression guard**: they were verified
by eye once and nothing stops them breaking again, which matters because that
checklist found six bugs single-client testing structurally cannot see. Wire-half
scenarios would cover the events; the pixels stay manual either way.

**Phase 3 should NOT be built as originally specced** — phase 1 changed its
premise. See the harness plan §6.

---

## The 2026-07-26 batch

Seven commits landed 2026-07-26 on `agent/platform-connectivity-controls`
(`3fc2c020` … `6b883a5f`) plus one Hercules commit (`ea036fac8` on
`agent/map-teleport-safety`). Both repos pushed, working trees clean.
**Nothing in this batch has been seen on screen or heard.**

## Bring the stack up (in this order)

```sh
brew services run mariadb                        # `run`, NEVER `start` — `start` re-registers autostart
cd Hercules && ./dev.sh start && ./dev.sh wait   # several minutes; loads 1156 maps
cd korangar/korangar && cargo run --release --bin korangar
```

`dev.sh` (added 2026-07-28) wraps `athena-start` and `make`; see
[MACOS_WORKFLOW.md](MACOS_WORKFLOW.md). Use `./dev.sh build` for any server-source
change — it fails loudly when `map-server` was not actually relinked, which is the
half of the `make map` trap a build log cannot show you.

Verified 2026-07-26 that the stack boots clean **with the new Hercules delta**:
`Successfully 'connected' to Database 'ragnarok'`, `Successfully loaded '1156' maps`,
map-server listening on 5121, char-server handshake OK. The map-server binary was
rebuilt after the delta, so **no `make` is needed** — but a server restart is.

Server stdout goes to **`Hercules/log/server-latest.log`** — a fresh file per run
now, so the whole file is this run. `log/map.log` stays empty. The older
`log/athena-start.out` was *appended* to across runs, which is why stale shutdown
errors from previous boots kept getting misread as live failures; it is untracked
leftover noise, so leave it uncommitted and ignore it.

## The checklist, cheapest first

Results as of the 2026-07-26 evening pass are in the **Result** column.

| # | What | How to see it | Watch for | Result |
|---|---|---|---|---|
| 1 | **Item names in messages** | Cast Land Protector with no Yellow Gemstone | "You need a Yellow Gemstone to use this skill.", never `#715`. Also check a trade-window row and a weapon-refine result — all three go through `resolve_item_name`. | **PASS** (skill-fail path; trade/refine rows not yet seen) |
| 2 | **Ammo-item projectiles** | Bow attack with plain Arrow, then Iron/Fire Arrow | The flying sprite should *change with the arrow type*. Firearms (views 17-21) and huuma shuriken (22) now fire too. Grenade launcher falls back to Bullet **by design**. | **PASS** 2026-07-29 — Fire Arrow, Iron Arrow, Bullet and Silver Bullet all resolve distinctly, on the shooter *and* on an observer. Grenade launcher's Bullet fallback is a **known deviation**, not by design: `battle_check_arrows` requires `A_GRENADE` for `W_GRENADE`, but no grenade projectile sprite was found in the GRF survey. |
| 3 | **Ground-cast walk-into-range** | Arm a ground skill, click a cell well outside its range | Should walk into range then cast, instead of nothing happening. If nothing walkable is close enough, expect a chat line rather than silence. | **PASS** — walked into range, then cast |
| 4 | **Support walk-into-range** | Heal/Blessing an ally ~15 cells away (Heal range is 9) | Same walk-then-cast. **This path changed behaviour**, so give it real attention — self-buffs must still fire instantly (self is distance 0). **Wants two seats:** the client path is entity-agnostic (`resolve_pending_cast`, `lib.rs:905`, treats `Attack`/`Support` alike), so a mob exercises the walk — but a mob closes the distance you are trying to measure. Neither test char has Heal; `@jobchange 8` + `@allskill`. | open |
| 5 | **Cast cancel** | Start a long cast, press **right-click**; repeat with **Escape** | Cast bar clears and the skill does NOT go off. Also: right-click with a skill *armed* still clears the reticle first; right-click on nothing still rotates the camera; Escape on nothing still opens the menu. **Moving must NOT cancel** — casting roots, and that is authentic. | **PASS** — both gestures cancel |
| 6 | **Ground-skill aiming footprint** | Arm Storm Gust (81 cells), then Land Protector Lv10 (225) | The real question: does a large area read as a *shape* or a solid slab? Colour/alpha are guesses (`IN_RANGE` / `OUT_OF_RANGE` in `render_skill_aiming_footprint`). Out-of-range should tint red. | **PARTIAL** — draws, and the red out-of-range tint works. The 225-cell shape-vs-slab question is still unanswered. |
| 7 | **Moonlit / Hermode** | **Two seats, Clown + Gypsy** — granting the skills is *not* enough, see below | Moonlit = flat salmon tile per cell, 9×9. **Hermode is sound-only by design** — hearing `헤르모드의 지팡이.wav` and seeing nothing is a **PASS**, not a bug. | open |

### Row 7 is an ensemble skill — it cannot be cast solo

Both `CG_MOONLIT` and `CG_HERMODE` are `Ensemble: true` (`db/re/skill_db.conf`),
`player_skill_partner_check: true` (`conf/map/battle/skill.conf:231`), and the
Admin group has **`skill_unconditional: false`** (`conf/groups.conf:308`) — so GM
99 does *not* bypass the check. `skill_check_condition_char_sub`
(`src/map/skill.c:15400`) requires a partner who is **all** of: opposite sex ·
job masks to `MAPID_BARDDANCER` · **knows the same skill** · wields an instrument
(`W_MUSICAL`) or whip (`W_WHIP`) · **in the same party** · not already dancing ·
not sitting. This is a renewal server, so the partner must independently pass the
skill's own cast requirements too.

Setup, once both seats are in: `@jobchange 4020` (Clown, seat A — male) and
`@jobchange 4021` (Gypsy, seat B — female), `@allskill` on both, an instrument
and a whip, and a party. Do this **last** — it replaces the bow gear the earlier
rows need.

### Found while driving the pass: `@item` could not take a multi-word name

`@item Iron Arrow 500` silently produced **one Iron** (id 998). Unquoted, `@item`
parsed `%99s %12d`, took `Iron` as the name, failed to read `Arrow` as the
quantity — and still returned ≥ 1, so it reported success. Only the *quoted*
form ever supported spaces.

Fixed server-side in `src/map/atcommand.c` (`atcommand_item_search` +
`atcommand_item_parse`, shared by `@item`, `@itembound`, `@item2`,
`@itembound2`). **The trap:** you cannot just peel the trailing integer off,
because **1797 items have a display name ending in a digit** (`Vesper Core 01`,
`Magic Bible Vol1`, `Vita500`). The parser therefore resolves *longest name
first* — it tries the whole argument string as a name, then peels one trailing
integer at a time until something resolves. Quoted names, bare IDs and
single-word names behave exactly as before.

**Longest-first has a trap of its own**, and it bit the first attempt: the ID
lookup was `itemdb->exists(atoi(name))`, and `atoi("1770 500")` is `1770`. So
the whole string resolved as an ID, the quantity was never peeled, and
`@item 1770 500` would have quietly handed over **one** item — a regression on
the most common DM usage. An ID is now only accepted when the string is numeric
end to end (`strtol` + endptr check). The compiler was happy either way; a
14-case throwaway C harness against a stub item table found it in one run.

Display-name lookup was already case-insensitive, so `iron arrow` works. Aegis
names (`Iron_Arrow`) stay case-sensitive because `case_sensitive_aegisnames:
true` in `conf/map/battle/misc.conf` — config left alone. For finding a name,
`@ii <partial>` already searched multi-word text correctly.

## Why item 7's alpha matters beyond item 7

Moonlit's tile is at **α 0.6**. The recovered roBrowser table for the whole
song/Gospel/Fog Wall family uses **α 0.05**, calibrated to *roBrowser's* renderer.
Korangar's ground-decal pass composites differently, and the batch-1 lesson was that
additive alphas needed ~0.5–0.8 against lit terrain. **Moonlit is the calibration
sample for that entire family** — note how it reads before anyone ports the rest.

## What comes after the live pass

Blocked on one engine decision, not on data: `UnitBody::GroundQuad` is a single flat
quad, but the authentic ground-tile recipe is *two* layers — a tinted tile plus a
texture bobbing between +0.2 and +0.6 cell with a per-cell phase offset. Adding a
`UnitBody` variant for that unlocks **`PA_GOSPEL` (`0xb3`)**, **`PF_FOGWALL`
(`0xb6`)** and **`NPC_EVILLAND` (`0xc7`)** immediately, and all 16 songs for free if
the campaign ever moves to pre-renewal. Full colour/texture table is in
[plans/classic-effect-fidelity.md](plans/classic-effect-fidelity.md).

## Test-environment traps — read before driving

- macOS F-keys and Home-to-sit: see [MACOS_WORKFLOW.md](MACOS_WORKFLOW.md).
- Hercules reports **several unrelated failures as "Skill level is not high enough"**
  (cause 0 is overloaded). Don't chase it as a client bug.
- Elemental fields silently refuse to spawn on top of *anything*, including the
  caster (`UF_NOFOOTSET`) — aim bare ground 4–5 cells away.
- The `test` character (150000) may be a Priest; `@jobchange 9` restores Wizard with
  the E1 hotbar intact.
- ~~**Do not run `rustfmt` in this repo.**~~ **Stale as of 2026-08-11 — following
  it now fights CI.** The tree was formatted end to end in `d5eb977a` (~60 files)
  and `formatting.yml` runs `cargo fmt --all --check` on every push and PR, so the
  committed tree *is* rustfmt-clean and must stay so. Use `cargo fmt --all`. The
  original warning was about `rustfmt <file>` dragging in a crate's worth of
  unrelated drift; that drift no longer exists.
