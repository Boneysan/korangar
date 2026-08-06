# Work backlog — assembled 2026-08-02

| | |
|---|---|
| **Why** | A day of fixes came out of lists nobody was reading (the suite's unmodeled-packet ledger, discarded `NetworkEvent`s). This is the standing inventory so that stops depending on someone remembering |
| **How it was built** | Fresh audits, not recollection — see §How to regenerate |
| **Ordering** | §1 is debt and blocks confidence in everything else. §2–§5 are new work |

## 1. Live verification — the debt, and it grew today

**~15 commits of UI have been built and never seen on screen.** Compiles and
wire-verified, which by this project's own rule says nothing about whether it
draws. This is the single largest risk in the tree.

- [ ] **Extend [gui-verification-pass.md](gui-verification-pass.md) §2c** — it
      covers the party and friend windows only, and **predates every window
      below**. Write rows before walking it.
- [ ] Target frame (left-click a player): name, class, Whisper / Invite / Trade /
      Add friend / Ignore; excluded on your own sprite
- [ ] Chat channels: Say / Party / Whisper, the whisper-target field, **Reply**,
      and the distinct colours for whisper vs party
- [ ] Three popups name their sender: party invite (via fork packet `0x0EFF`),
      trade request, friend request — each with Whisper
- [ ] Trade: add item from the right-click menu, add zeny from the field
- [ ] Party: kick / promote / share toggles (leader-gated), class in the roster,
      dead members
- [ ] Auto Spell window, instance window
- [ ] **Ice Wall blocks pathing** — walk into a cast Ice Wall and confirm the
      client refuses to path through it, then that the cells free up when it
      expires. **No headless scenario can cover this**: the suite does not link
      the `korangar` crate, so the pathfinder is unreachable from it
- [ ] Row 4 (Land Protector 225: shape or slab?), row 3 (support
      walk-into-range — *changed behaviour*), row 6 (item names), row 5
      (Moonlit/Hermode, last — replaces gear)

## 2. Features that exist only as discarded data

Each is a case where the server already sends everything needed.

- [ ] **`cutin` images (`DisplayImagePacket` 0x01B3)** — **83 NPC script files use
      it**. Highest value for a DM campaign per CLAUDE.md rule 1. Cost is real:
      the interface has **no image primitive**, so it needs an async texture load
      plus a custom `Element` in the shape of `interface/components/item_box.rs`
- [ ] **Quest system** — `QuestAdded` / `QuestRemoved` / `QuestList` are produced
      and dropped (`lib.rs:4362-4364`), plus `HuntingQuestNotificationPacket` and
      `HuntingQuestUpdateObjectivePacket` as no-ops. A window and state; the
      natural substrate for campaign objectives
- [ ] **Cast circles** — row 4b's root cause: *nothing triggers one*. `start_cast`
      only arms a bar and the `Beginspell`/`Lockon` recipes are driven by
      `EffectId`s the server sends only from quest scripts. In the original client
      this is client-side and keyed on the skill's element; take the mapping from
      roBrowserLegacy's tables, never guess
- [ ] **Refinery UI — refine from the inventory instead of only what you wear.**
      Raised by the tester at N25 (2026-08-05) and it is the *modern* behaviour,
      not a fork idea: Gravity replaced the refiner NPCs with a refinery window in
      Nov 2016. The pre-2016 script we use builds its menu from
      `getequipisequiped()` only (`npc/merchants/refine.txt:602-611`), which is
      why an unequipped item produces *"I don't think I can refine any items you
      have"*. **Hercules already implements the modern path** — `openrefineryui()`
      is called from that same script behind `features/replace_refine_npcs`, and
      both it and `enable_refinery_ui` need PACKETVER ≥ 2016-11-30 (we build
      **20220406**, so the server side is ready today).
      Four packets, currently present **only in the generated length table**
      (`lengths_20220406.rs:1349-1352`), so they are consumed by the length
      fallback and produce no `NetworkEvent`:
      `0x0AA0` `ZC_OPEN_REFINING_UI`, `0x0AA1` `CZ_REFINING_SELECT_ITEM`,
      `0x0AA2` `ZC_REFINING_MATERIAL_LIST` (variable length), `0x0AA3`
      `CZ_REQ_REFINING`. Cost is comparable to the trade window: four packets plus
      a window listing refinable inventory items with ore cost and odds.
      **THE TRAP — do not flip the flag first.** With
      `replace_refine_npcs: true` and no client support, `openrefineryui()`
      returns true and the script immediately `close()`s, so the NPC greets you
      and stops. Refining would go from equipped-only to **impossible**, with no
      fallback dialog. Client first, flag second.
- [ ] **Spirit spheres** — stored on `Common` from `ZC_SPIRITS`, never drawn.
      Sprite work
- [ ] **`EntityEffectState` (0x028A)** — modelled, handled as `{}`. May drive
      entity status visuals; needs a look before it earns a window
- [ ] **Phase 4 appearance** — headgear, robe, clothes/hair colour, body style.
      `EntityData` has carried all seven fields since phase 1; composition is
      still body + head + weapon + shield. Blocked on accessory paths and palette
      files, not effort

## 3. Small, bounded gaps

- [ ] **`set_all_ignored` is unreachable** — added 2026-08-02 and never wired to
      anything. Wants `/ignore all` plus a toggle. *(Self-inflicted; the audit in
      §How to regenerate is what caught it.)*
- [ ] **Chat input history (up-arrow)** — the text box handler only receives
      `handle_character`, so arrow keys never reach it. Supporting it means
      changing **`korangar-interface`**, which is shared upstream code and cuts
      against the rebaseability rule (CLAUDE.md §4). **A decision, not a task**
- [ ] **`ZC_TALKBOX_CHATCONTENTS` (0x0191)** — an NPC message board's contents,
      unmodeled. Low priority on its own, but see the note about *how it was
      found* under §How to regenerate: it appeared only under a shuffled run
- [ ] **Triage the remaining `register_noop` packets** — 25 left. Likely worth
      modelling: `ConnectionRefusedPacket` (silent login failures?), `OpenUiPacket`
      (server asking the client to open a UI), `PersonalInformationPacket` (exp /
      drop rate modifiers). Almost certainly not: clan, mail, market, achievement,
      reputation, pincode, character-slot paging
- [ ] **DM instances are unusable for most maps — upstream Hercules limit.**
      Found at N24 (2026-08-05). `instance.c:240` formats the instanced map's name
      into `MAP_NAME_LENGTH` (`11 + 1`, `mmo.h:343`), and the `NNN#` prefix takes
      four of the eleven, leaving **seven characters for the map name**.
      `000#prontera` needs twelve and is clipped to `000#pronter`;
      `mapindex_getmapname_ext` then strips the prefix and sends the client
      `pronter.gat`, which exists in no GRF. **Any map name longer than seven
      characters fails**, `prt_fild08` included — an official client fails
      identically, so this is not korangar's bug and not fixable client-side.
      Options are a Hercules delta widening the buffer (touches `mmo.h`, so wide
      blast radius and it changes a wire-visible constant) or restricting DM
      instances to short-named maps. **Decide before building DM instance
      content**, since `@dminstance` is campaign tooling per CLAUDE.md rule 1.
      Two operational notes: a stuck instance survives client restarts and
      re-warps every login back onto the phantom map until the **server** is
      restarted, and `$dm_inst_<party>` records in `map_reg_num_db` outlive dead
      parties and permanently bar a recycled party id from hosting one (cleared
      2026-08-05).
- [ ] **~131 `TODO`/`FIXME`** across client + networking, mostly upstream cosmetic
      ("put this in the theme"). Worth one sweep to separate ours from upstream's

## 4. Test-suite depth

- [ ] **The eleven packets modelled on 2026-08-02 have no scenarios** — party
      death, NPC refine, Auto Spell, spirit spheres, skill-unit update, snap,
      effect state, star/feel, and the three instance packets. They are modelled
      and framed (the length audit covers the fixed-length ones) but nothing
      asserts their *behaviour*
- [ ] **Ice Wall cell blocking — only half of it is testable headlessly.** The
      **wire** half is: cast one, assert `MapCellChanged` arrives and that the
      revert follows on expiry. The **pathing** half is not, and this is a
      general limit worth internalising: the headless tester links only
      `ragnarok-packets` and `korangar-networking`, **not the `korangar` crate**,
      so `Map`, `Traversable` and the pathfinder are invisible to every scenario.
      Anything in `korangar/src/` can only be verified in the client — which is
      also why "the suite is green" says nothing about a rendering or pathing
      change
- [ ] **Ground skills assert almost nothing** — ~32 addressable casts;
      `skill_db`'s `Unit:` block is the authority
- [ ] **`MG_FROSTDIVER` is a known intermittent** in `skills-super-novice`
- [ ] **`SM_PROVOKE` in `skills-swordman` is order-dependent** — fails in a full
      run, passes alone (2026-08-02 gate). Logged as finding §4 in
      [../../tools/testing/headless_findings.md](../../tools/testing/headless_findings.md).
      **Bisect it, do not theorise** — every previous instance of this class had
      a confident wrong hypothesis first. **`--shuffle 42` does not reproduce
      it** (clean 122/0/1), so that seed is not the lever; try other seeds to
      find a replayable ordering before hand-bisecting
- [x] ~~Run `--scenario all --shuffle <seed>`~~ — **done 2026-08-02, seed 42:
      122 passed / 0 failed / 1 skipped.** The ten scenarios added that day all
      survive reordering, including the four that leave server-side state
      (party membership, share rules, the ignore list, an open trade). Worth
      repeating with other seeds: one seed samples one ordering, it does not
      prove the suite order-independent

## 5. Server-side deltas to re-apply after any upstream Hercules merge

Documented in korangar `CLAUDE.md` §3b; listed here so a merge does not lose them
silently. Every one fails *quietly* — the feature simply stops appearing.

- [ ] `KORANGAR_PARTY_SP_TO_GROUPM` — party-member SP (guarded by
      `party-member-vitals` + `party-sp-only-broadcast`)
- [ ] `ZC_PARTY_INVITE_SENDER` `0x0EFF` — names the inviter (guarded by
      `party-invite-sender`)
- [ ] `CZ_CANCEL_CAST` `0x0F00`, `SC_LANDPROTECTOR`, `status_get_val_flag`,
      `LOOK_AMMO`, multi-word `@item`, `ip_rules` disabled

## How to regenerate this list

These four greps produced §2–§4. **They disagree with each other on purpose** —
an event can be discarded in three different places, and only the third shows up
in the suite's report:

```sh
# 1. Events produced by the networking layer, then thrown away by the client.
grep -n "NetworkEvent::[A-Za-z]* *{ *\.\. *} *=> *{}" korangar/src/lib.rs

# 2. Packets modelled but deliberately not acted on.
grep -o "register_noop::<[A-Za-z]*>" \
  korangar-networking/src/packet_versions/version_20220406.rs | sort -u

# 3. Packets with no handler at all — the suite prints these every run under
#    "Unmodeled packets seen". THAT LIST IS A BUG REPORT. Two real bugs came out
#    of reading it on 2026-08-02.
#
#    ONE RUN UNDER-REPORTS. The list depends on execution order: the natural
#    order showed 1 unmodeled header and `--shuffle 42` showed 2, turning up
#    ZC_TALKBOX_CHATCONTENTS (0x0191) that the natural order never reached.
#    Incoming-handled moved too (164 vs 163). Combine several seeds before
#    treating the list as complete.
cargo run --release --example headless-tester -p korangar-networking -- --scenario all
cargo run --release --example headless-tester -p korangar-networking -- --scenario all --shuffle 42

# 4. Client actions the UI never reaches. Grep the *whole* client, not just
#    lib.rs -- several are called from `state/`, and a narrow grep produced three
#    false positives the first time this was run.
grep -rn "\.<action_name>(" --include='*.rs' korangar/src
```

**`0x0078` must stay unmodeled.** It is `clif_sendfakenpc`, the invisible
class-111 NPC that drives script dialogs, and at 78 occurrences it is the loudest
entry in the ledger. Modelling it would spawn phantom entities. **Frequency there
ranks noise, not importance.**
