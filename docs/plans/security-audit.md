# Security audit — first pass, 2026-08-17

| | |
|---|---|
| **Status** | **First audit ever run on this project.** Findings below are unfixed unless marked otherwise |
| **Trigger** | The client is about to be handed to friends and the server exposed beyond localhost |
| **Scope of THIS pass** | Credentials and privilege, server exposure config, Rust dependency CVEs, fork script gating, pack hygiene |
| **NOT covered — say so rather than imply coverage** | Memory safety of the C server (Hercules is C, and our 11 fork deltas touch `clif.c` packet parsing), fuzzing of korangar's packet decoders, upstream Hercules CVEs, the Lua/GRF asset path |

## Threat model, stated first

Everything below is graded against **the deployment that actually exists**: a
Hercules server on a home LAN, a handful of friends' Windows machines, an
unsigned client handed out on Google Drive. **The attacker is someone on that
LAN** — a housemate, a guest on the wifi, a compromised laptop — not the
internet. The grading changes completely if this is ever port-forwarded, and
several MEDIUMs become CRITICAL the day it is.

---

## CRITICAL

### C1. Two full-admin accounts with guessable passwords, and the credentials are published

| account | password | group |
|---|---|---|
| `korangar` | `korangar` | **99 (Admin, level 99)** |
| `headless2` | `headless2pw` | **99 (Admin, level 99)** |

Group 99 carries `all_commands: true` and `hchsys_admin`. The password is the
username in one case, and **`Boneysan/korangar` is a PUBLIC repository with
`headless2pw` in tracked files** — `headless-tester/main.rs`,
`run-integration-tests.sh` and two docs — so the credential is on the internet
and indexed, not merely weak.

**What an attacker gets, stated precisely.** They reach 6900 with any RO client,
authenticate, are handed 6121 and 5121 (both open), and arrive in-game with every
GM command: `@kick` / `@charban` any player, `@item`, `@monster`, `@delitem` off
other characters, `@accinfo` on other accounts, and `@reloadscript` /
`@unloadnpcfile` to change server behaviour mid-session.

**What they do NOT get, because an earlier draft of this document overstated it:
the machine.** No atcommand shells out, runs raw SQL, or reads arbitrary files —
checked against every `ACMD()` in `atcommand.c`. This is total control of the
game and its data, **not** remote code execution and not root on the host. Grade
it as the former.

**Reachability is the only thing keeping this bounded**, and it is bounded by the
LAN, not by the password. Port-forward anything and this becomes remotely
exploitable by anyone who reads the repo.

**Fix:** change both passwords, and drop `headless2` to group 0 or delete it.
**Blocker to check first:** the headless suite authenticates as these accounts,
so changing them without updating the harness fixtures turns the entire test
suite red. Fix both together.

---

## HIGH

### H1. Passwords are stored in plaintext (OWASP A02 — Cryptographic Failures)

`login.user_pass` holds the literal password — `LENGTH('korangar') = 8`. There
is no MD5, no bcrypt, no salt. Anyone with a moment at the DB, a stolen backup,
or SQL injection anywhere in a script reads every friend's password. **People
reuse passwords**, so the blast radius is their other accounts, not this game.

### H2. Credentials cross the network in plaintext (OWASP A02)

The RO login protocol sends the password unencrypted, `packet_obfuscation` is
`0` by project decision, and there is no TLS. Any device on the LAN can read
friends' passwords off the wire with tcpdump. **This is not fixable without
changing the protocol**, so the mitigation is procedural: tell friends in the
Drive doc to use a throwaway password they use nowhere else. That sentence is
currently missing.

---

## MEDIUM

### M1. Connection-flood protection is disabled (OWASP A05 — Security Misconfiguration)

`conf/common/socket.conf` has `ip_rules: { enable: false }`. It was turned off
deliberately (see `CLAUDE.md`) because the headless harness reconnects rapidly
from localhost and tripped the DDoS heuristic. The reasoning was sound for a
localhost-only box and stops being sound the moment anything else can connect.

### M2. Interserver account is at Hercules defaults

`s1` / `p1`, unchanged from the shipped example. Low impact while the three
servers are on one machine, and free to fix.

### M3. `quick-xml 0.39.2` — two advisories rated 7.5 (OWASP A06)

`RUSTSEC-2026-0195` (unbounded namespace allocation → memory exhaustion) and
`RUSTSEC-2026-0194` (quadratic time on duplicate attribute names). **Reachable**:
korangar parses `sclientinfo.xml` through it. Practical risk is low because that
file ships inside our own archive rather than arriving from the network — but it
is a version bump to `>=0.41.0`, so there is no reason to carry it.

### M4. The client stores passwords in plaintext

`settings/login.rs` writes `password` verbatim into `client/login_settings.ron`
when "remember password" is ticked. Every friend's machine will hold one.
`make-pack.sh` refuses to ship *ours* (see G4), which is a different problem
from theirs.

---

## Code review — first pass over the fork's own code

Reviewed the code that touches attacker-reachable bytes, since that is where a
C server's real risk lives. **One defect found and fixed; the rest verified
clean rather than assumed clean.**

### M5. Client panicked on a short first read (FIXED 2026-08-17)

`korangar-networking/src/lib.rs` opened the map-server connection by reading a
bare four-byte account id and **`.unwrap()`ing it**. `stream.read()` returns
whatever the OS has — as little as one byte — and the only guard was
`received_bytes == 0`, so a first TCP segment carrying 1-3 bytes **panicked the
entire client on connect**.

This needs no attacker: it is ordinary TCP segmentation, invisible over loopback
(where every dev session runs) and perfectly possible on wifi or a LAN. It is an
availability bug, not a memory-safety one — Rust — but a friend would see "the
game crashes when I log in".

Fixed by retaining the fragment and waiting for the rest, reusing the same
machinery `PacketCutOff` already uses for partial packets. **Not covered by a
test** — the read loop lives in a tokio task with no seam for injecting a short
read, and inventing one was out of scope for this pass.

### Verified clean

- **`clif_parse_CancelCast` (0x0F00)**, the fork's only client-facing parser:
  zero payload, no buffer access, one call. Nothing to get wrong.
- **The `@item` multi-word parser**, our own and the most string-handling code in
  the fork: `safestrncpy` bounded, `Assert_retr` pinning `peeled[16]` against
  `max_numbers`, a `count == max_numbers` guard before every write, `strtol` with
  endptr validation rather than `atoi`, and `sscanf` bounded at `%99[^\"]` into a
  100-byte buffer.
- **`ZC_PARTY_INVITE_SENDER` (0x0EFF)** writer: `safestrncpy(..., NAME_LENGTH)`
  into `char[NAME_LENGTH]`, matching the client's `#[length(24)]`.
- **`ragnarok-bytes`**, the wire decoder: every `unwrap` is in test code, no
  slicing or indexing of wire data, everything returns `Result`.

### Still not reviewed

The remaining C deltas (`LOOK_AMMO`'s five touch points, the party-SP widening,
`clif_skill_fail_reason`) and korangar's ~80 other non-test `unwrap`s outside the
byte layer. **A pass that stopped after finding one bug is not a completed
review.**

## LOW / INFORMATIONAL

- **L1.** `crossbeam-epoch` invalid pointer dereference (`RUSTSEC-2026-0204`),
  fixed in `>=0.9.20`. Reached only through a `fmt::Pointer` impl on an already
  invalid pointer.
- **L2.** Unmaintained: `cgmath`, `paste`, `ttf-parser`. Unsound advisories:
  `anyhow`, `memmap2`. No action beyond awareness.
- **L3.** `quinn-proto`'s 7.5 advisory appears in `cargo audit` but **`cargo tree
  -i quinn-proto` finds nothing** — not in our graph, not counted.
- **L4.** Client and server binaries are unsigned (SmartScreen / Gatekeeper).
  Accepted for a friends pack; it does train friends to click through warnings.

---

## What is already RIGHT — verified, not assumed

- **G1.** The default `Player` group (id 0, every new account) grants **no
  commands at all** — only `can_trade` and `can_party`. A friend's self-registered
  account cannot `@item` itself anything.
- **G2.** DM commands are bound at group level ≥ 1 (`bindatcmd(..., 1, 99, 1)`),
  so they are unreachable from a default account.
- **G3.** No SQL is built from script input anywhere in `npc/custom/dm_campaign/`
  — no `query_sql` at all.
- **G4.** `make-pack.sh` **aborts** if `login_settings.ron` reaches the pack, so
  the admin credentials cannot be uploaded to Drive by accident.
- **G5.** All three servers are in the macOS firewall allow list explicitly.

---

## Frameworks — which ones earn their place

**CWE Top 25 is the best fit and the findings map onto it cleanly**, which is
the test of whether a taxonomy is doing work or decorating:

| Finding | CWE | |
|---|---|---|
| C1 admin passwords in tracked files | **CWE-798** Use of Hard-coded Credentials | open |
| H1 plaintext `user_pass` column | **CWE-256 / CWE-257** Plaintext Storage of a Password | open |
| H2 credentials on the wire | **CWE-319** Cleartext Transmission of Sensitive Information | won't fix (protocol) |
| M1 `ip_rules: enable: false` | **CWE-770** Allocation Without Limits or Throttling | open |
| M3 `quick-xml` advisories | **CWE-400** Uncontrolled Resource Consumption | open |
| M5 `.unwrap()` on a short read | **CWE-248** Uncaught Exception → crash | **FIXED** |
| Verified absent in the DM scripts | **CWE-89** SQL Injection | clean |
| Verified bounded in the `@item` parser | **CWE-787 / CWE-120** Buffer Write | clean |

**Why not the others.** **SAMM** measures an *organisation's* security programme
— training, champions, policy, incident response — and this is one person and a
friends server, so every score would be a zero that means nothing. **ASVS** is a
*web application* verification standard: no browser, no HTML, no cookies, no
sessions in the web sense. Running either would produce a long document that
assures nothing, and the effort is better spent on checks that re-run themselves.

**What is worth stealing from SAMM without adopting it** is three practices, and
all three are now automated rather than assessed
([`.github/workflows/security.yml`](../../.github/workflows/security.yml)):

- **Dependency scanning** — `cargo audit` on push, PR, and a **weekly cron**.
  The cron is the load-bearing trigger: advisories are published against code
  that has not changed, so a push-only scan finds them late or never.
  Unmaintained-crate warnings are explicitly ignored (`cgmath`, `paste`,
  `ttf-parser` all arrive through `wgpu`/`cosmic-text` and are not ours to fix)
  because reddening a build over something nobody can action trains people to
  ignore the build.
- **Secret detection** — [`committed-secrets.sh`](../../tools/audits/committed-secrets.sh),
  targeted at known credentials rather than entropy, because this codebase is
  full of packet hex and GRF hashes and a generic scanner would drown in false
  positives. It carries a **baseline** of the three sites the audit found, so it
  passes today and fails on any *new* leak — known debt stays visible instead of
  making CI permanently red. Verified in both directions.
- **Defect management** — this document, with severities and an explicit
  "not reviewed" section.

**SLSA is the gap worth naming.** We hand friends an unsigned binary built on a
developer laptop, over Drive, with no checksum and no provenance. Nobody can tell
a good download from a corrupted or swapped one. That is the next supply-chain
step, and it is cheap: publish `SHA256SUMS` beside the pack, and prefer the CI
`windows-2025` build over a laptop build as the artifact's source.

## On OWASP, honestly

**The OWASP Top 10 is a web-application list and this is not a web
application.** Roughly half of it has no meaning here: there is no browser, no
HTML, no cookies, no SSRF surface. Forcing the whole taxonomy onto a game client
would produce a document that looks thorough and audits nothing.

What genuinely maps is used above: **A02 Cryptographic Failures** (H1, H2),
**A05 Security Misconfiguration** (M1, M2), **A06 Vulnerable Components** (M3,
L1), **A07 Identification and Authentication Failures** (C1). What is missing
from any web framework — and matters most here — is that **Hercules is C**, so
the real risk class is memory safety in packet handling, which is CWE territory
(CWE-787, CWE-125, CWE-20) rather than OWASP.

**Recommended next passes, in order of value:**

1. **Fix C1.** Nothing else on this page matters while two published passwords
   grant full admin.
2. **Add `cargo audit` to CI.** It is one job, it found four real advisories
   today, and dependency drift is invisible otherwise.
3. **Review the 11 fork deltas in `clif.c` for input validation** — they parse
   attacker-reachable bytes and they are the code upstream has never reviewed.
   Start with `clif_parse_CancelCast` (0x0F00), since a client packet with a bad
   length already has a documented failure mode: session disconnect.
4. **Fuzz korangar's packet decoders.** A malicious *server* is not the threat
   model, but a corrupted stream is, and the client already has a documented
   failure where one bad packet costs the whole read buffer.
