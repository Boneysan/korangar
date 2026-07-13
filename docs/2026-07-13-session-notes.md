# Session notes — 2026-07-13

Sprite lighting, emote bubbles, sprite-name resolution, and view-distance work.
Companion server-side change in `Hercules/conf/import/battle.conf`.

## Sprite lighting modes (shaders + settings)

Sprites synthesize their lighting normal from the camera
(`entity*.slang` vertex shader), so full Lambert made character brightness
swing 100%→0% as the camera orbited relative to the map sun. New **"Sprite
lighting"** dropdown in graphics settings (only active while Lighting mode is
Enhanced):

- **Classic** — flat scene light, no directional response.
- **Soft** *(default)* — wrapped diffuse `clamp(N·L * 0.35 + 0.65, 0, 1)`;
  camera orbit now swings 100%→~30%. Also applied to point lights so torch
  glow follows sprites at any camera angle.
- **Enhanced** — the previous full-Lambert response.

Implementation: `sprite_light_percent()` + wrap constants in
`shaders/modules/forward.slang`; `sprite_lighting` uniform in
`GlobalUniforms` (Rust struct padded to 496 bytes to match std140);
`SpriteLightingMode` enum in `graphics/settings.rs`; threading through
`instruction.rs`, `lib.rs`, `settings/graphic.rs` (serde-defaulted for old
settings files), and the graphics settings window.

Point-light intensity was also raised 10 → 14 and deduplicated into
`POINT_LIGHT_INTENSITY` (was hardcoded in five shaders). Tuning knobs all
live at the top of `forward.slang`.

## Emote bubbles + readable emote text

- `DisplayEmotion` events now play the matching action from
  `이팩트\emotion.spr/.act` as a one-shot billboard at the entity
  (`world/emote.rs`, `AnimationData::render_action_frame`); the wire emote ID
  is used directly as the ACT action index. The shared sheet loads lazily on
  first emote (async), so the session's first emote shows no bubble. Bubbles
  replace per entity, clear on map change, and hold the last frame ~400 ms.
  Set `KORANGAR_EMOTE_DEBUG=1` for pipeline diagnostics on stderr.
- Chat log line kept, now with human-readable text ("Sage Worm: hmph!") via
  an 81-entry table (`emotion_name` in `lib.rs`) ordered per Hercules'
  emote constants.

## Phantom green heal numbers — packet fix

`DisplaySkillEffectNoDamagePacket` (0x09CB) was unconditionally rendered as a
heal. Hercules sends `-1` ("suppress display", e.g. mob Cloaking — read as
u32 it became a 4.29-billion green number) and skill levels for buffs. Now
only real healing skills display: AL_HEAL 28, AB_CHEAL 2043,
AB_HIGHNESSHEAL 2051, HLIF_HEAL 8001 (Hercules re-labels item/regen heals as
AL_HEAL, so those still show). `korangar-networking/src/packet_versions/version_20220406.rs`.

Follow-up roadmap idea: surface suppressed mob casts as flavor lines
("The Whisper shimmers and fades away…").

## Classic mob silhouettes — jobname.lub overlay

Sprite names were derived from `jobidentity.lub`/`npcidentity.lub` JT_
constants, which diverge from actual file names for classic mobs
(JT_SOLDIER_SKELETON → `skel_soldier.spr`). `JobIdentity::load` now overlays
`jobname.lub`'s `JobNameTable` (the table the official client uses; EUC-KR
values decoded). Soldier Skeleton (1028) verified resolving.

**New audit tool**: `cargo run --release --bin entity-sprite-audit`
(`tools/entity_sprite_audit.rs` → `korangar::audit_entity_sprites()`).
2026-07-13 result: 4434 identities checked; only 34 spawnable Hercules mobs
missing sprites, all renewal/event-era content absent from the 2019 GRF.
`.gr2` entries are Granny 3D models (WoE guardians/Emperium/treasure boxes)
outside the sprite system; skill-unit pseudo-NPCs (IDs 126+) render as
effects and are expected audit noise.

## Entity view distance (server side)

Entity pop-in radius is the server's `area_size` (stock 14 cells, tuned for
the 2004 client). Set to **30** (with `dead_area_size: 48` = area_size + 18)
in `Hercules/conf/import/battle.conf` for high-resolution/zoomed-out play.
Apply with `@reloadbattleconf` + `@refresh`/map change.

## Map light-source survey (reference)

RSW light-source scan of all 899 maps (throwaway scanner, not committed):
gl_cas01 320 lights/dark ambient (best torch showcase), in_sphinx1 507,
abbey02 454, pay_dun01 223, pay_dun00 only 67 with bright 0.4 ambient (sparse
by design). Town torches are effect-only: geffen/morocc have **zero** RSW
lights, prontera 2; prt_in has 107 lamps but ~0.9 ambient washes them out —
motivation for the per-map "artistic profiles" slice in the lighting plan.
