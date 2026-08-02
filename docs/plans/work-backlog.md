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
- [ ] **Triage the remaining `register_noop` packets** — 25 left. Likely worth
      modelling: `ConnectionRefusedPacket` (silent login failures?), `OpenUiPacket`
      (server asking the client to open a UI), `PersonalInformationPacket` (exp /
      drop rate modifiers). Almost certainly not: clan, mail, market, achievement,
      reputation, pincode, character-slot paging
- [ ] **~131 `TODO`/`FIXME`** across client + networking, mostly upstream cosmetic
      ("put this in the theme"). Worth one sweep to separate ours from upstream's

## 4. Test-suite depth

- [ ] **The eleven packets modelled on 2026-08-02 have no scenarios** — party
      death, NPC refine, Auto Spell, spirit spheres, skill-unit update, snap,
      effect state, star/feel, and the three instance packets. They are modelled
      and framed (the length audit covers the fixed-length ones) but nothing
      asserts their *behaviour*
- [ ] **Ice Wall cell blocking has no scenario** — cast one, assert
      `MapCellChanged`, and that the reverse arrives on expiry
- [ ] **Ground skills assert almost nothing** — ~32 addressable casts;
      `skill_db`'s `Unit:` block is the authority
- [ ] **`MG_FROSTDIVER` is a known intermittent** in `skills-super-novice`
- [ ] **Run `--scenario all --shuffle <seed>`** now that ten scenarios have been
      added; order-dependence was a live problem three weeks running

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
cargo run --release --example headless-tester -p korangar-networking -- --scenario all

# 4. Client actions the UI never reaches. Grep the *whole* client, not just
#    lib.rs -- several are called from `state/`, and a narrow grep produced three
#    false positives the first time this was run.
grep -rn "\.<action_name>(" --include='*.rs' korangar/src
```

**`0x0078` must stay unmodeled.** It is `clif_sendfakenpc`, the invisible
class-111 NPC that drives script dialogs, and at 78 occurrences it is the loudest
entry in the ledger. Modelling it would spawn phantom entities. **Frequency there
ranks noise, not importance.**
