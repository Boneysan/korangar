# Security audit — fourth pass, 2026-08-17

| | |
|---|---|
| **Status** | Independent fourth pass. Remediations 2026-08-18 |
| **Parents** | [security-audit.md](security-audit.md) (first), [security-audit-2.md](security-audit-2.md) (second), [security-audit-3.md](security-audit-3.md) (third) |
| **Trigger** | Claude and Grok had already reviewed the project. Look for concrete, reachable chains they did not record or did not characterize fully |
| **Scope of THIS pass** | Hercules API request parsing and GIF decoding; end-to-end chat flow; korangar packet conversion and protocol-state handling; current dependency advisories |
| **NOT covered** | Sending denial-of-service payloads to the live API; a sanitizer or coverage-guided fuzzing campaign; Windows execution of the friends pack; a complete upstream Hercules CVE sweep |

**Remediated 2026-08-18:** P4-1 GIF dimension/frame precheck before `DGifSlurp`, and guild-master authorization before any decode (API asks char first; result `2` means send the image); P4-2 exact 16-byte token + constant-time compare, no token logging; P4-3 `min_chat_delay` 300 ms and a 500-message client cap; P4-4 fallible flag conversion, inventory list/end no longer `expect`, unknown selected character returns to an error instead of panicking.

The first three reports remain the source of truth for their findings. In
particular, C1 (published group-99 credentials), H2 (plaintext transport), N3
(the API listening on the LAN), T1 (Lua/archive trust), and T2 (stealable
session tokens) are still open unless their own reports say otherwise. Those
conditions increase the reachability or impact of this pass; they are not
renumbered here.

**Do not put the live interserver password, API token, or a working denial-of-service payload in this file.**

---

## Result

This pass found one high and three medium issues:

| ID | Severity | Finding | Attacker required |
|---|---|---|---|
| **P4-1** | **HIGH** | GIF validation allocates/decodes before resource and guild authorization checks | Any online account with its own API token |
| **P4-2** | **MEDIUM** | Short `AuthToken` causes a 16-byte heap out-of-bounds read | LAN client that knows or guesses an online account id; valid token not required |
| **P4-3** | **MEDIUM** | Unlimited server chat plus unbounded full-history client layout is a player-to-client resource DoS | Any authenticated player |
| **P4-4** | **MEDIUM** | Network-derived flags, packet order, and character ids reach Rust `expect` / `unwrap` | Malicious server or active network attacker |

No new player-to-GM path, SQL injection, remote code execution, or verified
authentication bypass was found in this pass. That is a bounded statement about
the reviewed paths, not a claim that the C server or packet surface is fully
fuzzed.

---

## Method and verification

Reviewed the current trees:

- korangar `a284dadd` on `agent/platform-connectivity-controls`;
- Hercules `71267a91b` on `agent/map-teleport-safety`.

The review traced each candidate from attacker-controlled input to its final
consumer rather than treating a dangerous function or advisory version as a
finding by itself. No live service received a crash or exhaustion payload.

Non-destructive checks:

- `cargo test -p ragnarok-packets -p korangar-networking`: **90 passed** (38 +
  52), 0 failed;
- Hercules `make test`: **4 passed** (`test_libconfig`, `test_spinlock`,
  `test_chunked`, `test_base62`);
- `cargo audit --no-fetch`: cached database of 1,216 advisories, 573 lockfile
  packages, 4 vulnerabilities and 6 allowed warnings; reachability is triaged
  below;
- both worktrees were clean after the generated Hercules test executables were
  removed.

---

## HIGH — new or materially reclassified

### P4-1. `/emblem/upload` performs attacker-amplified GIF allocation before resource and guild checks

This corrects the third pass's low/informational statement that the disabled
frame-size check was only a "cheap API CPU DoS." The underlying path can request
very large allocations, performs all frame decoding before enforcing the frame
count, and runs before guild-master authorization.

The endpoint does require an online account and its 16-byte API token:

```30:30:../../../Hercules/src/api/urlhandlers.h
handler2(HTTP_POST, "/emblem/upload", emblem_upload, REQ_EMBLEM_UPLOAD);
```

`REQ_EMBLEM_UPLOAD` includes `REQ_ACCOUNT_ID`, `REQ_WORLD_NAME`,
`REQ_AUTH_TOKEN`, `REQ_CHAR_LOGGED_IN`, `REQ_GUILD_ID`, `REQ_IMG_TYPE`, and
`REQ_IMG`. It does **not** require that the character is master of the submitted
guild before the API handler runs.

The API's only pre-decode GIF resource check is the **compressed** file size,
currently 51,200 bytes, followed by a six-byte magic/version check:

```147:181:../../../Hercules/src/api/imageparser.c
static bool imageparser_validate_gif_emblem(const char *emblem, uint64 emblem_len)
{
    ...
    if (emblem_len > extraconf->emblems->max_gif_guild_emblem_size) {
        ...
        return false;
    }
    ...
    const int ret = DGifSlurp(image);
```

The configured limit does not bound decoded pixels:

```35:47:../../../Hercules/conf/common/emblems.conf
max_gif_guild_emblem_size: 51200
min_guild_emblem_frames: 1
max_guild_emblem_frames: 100
guild_emblem_width: 24
guild_emblem_height: 24
```

`DGifSlurp` reads an attacker-controlled image descriptor and allocates its
`Width * Height` raster before the application sees the frame. The bundled
giflib rejects integer overflow but permits a product up to `INT_MAX` bytes —
roughly 2 GiB for a single frame descriptor:

```1143:1163:../../../Hercules/3rdparty/libgif/dgif_lib.c
case IMAGE_DESC_RECORD_TYPE:
    if (DGifGetImageDesc(GifFile) == GIF_ERROR)
        return (GIF_ERROR);
    ...
    if (sp->ImageDesc.Width <= 0 || sp->ImageDesc.Height <= 0 ||
            sp->ImageDesc.Width > (INT_MAX / sp->ImageDesc.Height)) {
        return GIF_ERROR;
    }
    ImageSize = sp->ImageDesc.Width * sp->ImageDesc.Height;
    ...
    sp->RasterBits = (unsigned char *)reallocarray(NULL, ImageSize,
            sizeof(GifPixelType));
```

Only **after `DGifSlurp` completes** does Hercules require a 24×24 logical
screen and 1–100 frames (`imageparser.c:201–223`). The per-frame dimension check
at `imageparser.c:190–200` is commented out. A truncated file can make the
decoder attempt the allocation before it discovers that pixel data is missing;
a highly compressible image can additionally make a small upload drive much
more decode work and resident memory than its compressed size suggests.

The API validates the image before forwarding it to the char server:

```340:367:../../../Hercules/src/api/handlers.c
if (strcmp(imgType, "BMP") == 0) {
    ...
} else if (strcmp(imgType, "GIF") == 0) {
    if (!imageparser->validate_gif_emblem(img, img_size)) {
        ...
    }
    is_gif = true;
}
...
SEND_CHAR_ASYNC_DATA(emblem_upload_guild_id, &data);
SEND_CHAR_ASYNC_DATA_SPLIT(emblem_upload, img, img_size);
```

Guild ownership is checked much later, after the decoded upload has crossed to
the char server:

```217:223:../../../Hercules/src/char/capiif.c
RFIFO_CHUNKED_COMPLETE(p) {
    bool success = false;
    if (inter_guild->is_guild_master(p->base.char_id, char_data->emblem_guild_id)) {
        success = inter_guild->update_emblem(...);
    }
```

**Reachability:** any online player can use their own token and submit someone
else's nonzero guild id. They do not need to be in that guild or be its master;
that failure occurs after decoding. N3 already records that the API binds to
all interfaces on port 7121 in the current configuration. Open registration and
C1 make account acquisition easier but are not required once a normal account
exists.

**Impact:** repeatable API-server CPU and virtual/resident memory pressure,
process termination by the allocator/OS, and possible host-wide pressure. The
API is a separate process, so a clean API crash does not directly corrupt the
map server, but the operating system is the shared resource.

**Fix and acceptance criteria:**

1. Authorize the `(char_id, guild_id)` relationship before decoding or storing
   image bytes. Reject non-masters first.
2. Do not call `DGifSlurp` on an untrusted file before limits are known. Parse
   records incrementally and reject before raster allocation when:
   - logical screen is not 24×24;
   - a frame rectangle falls outside the logical screen;
   - frame width/height or `width * height` exceed the 24×24 budget;
   - frame count exceeds 100;
   - cumulative decoded pixels exceed a small explicit budget.
3. Add a request concurrency/rate limit and an API process memory limit as
   defense in depth. Binding the API to loopback remains the immediate N3
   containment.
4. Add tests with huge descriptors, too many frames, truncated image data,
   out-of-canvas frame rectangles, and a valid 24×24 animation. Run them under
   ASan with an allocation ceiling; a rejection must not attempt the advertised
   raster allocation.

**CWE:** CWE-400 Uncontrolled Resource Consumption.

---

## MEDIUM — new

### P4-2. Short multipart `AuthToken` causes a heap out-of-bounds read

This is in the common API authorization path, not only emblem upload.

Each multipart value is allocated at exactly its supplied length plus a NUL:

```543:574:../../../Hercules/src/api/aclif.c
static void aclif_set_post_header_data(int fd, const char *value, size_t size)
{
    ...
    sd->temp_mime_header->data = aMalloc(size + 1);
    memcpy(sd->temp_mime_header->data, value, size);
    sd->temp_mime_header->data_size = (uint32)size;
    ...
    sd->temp_mime_header->data[sd->temp_mime_header->data_size] = '\x0';
}
```

`get_post_header_data_str` can return the length, but the authentication caller
passes `NULL` for it and then always reads `AUTH_TOKEN_SIZE` (16) bytes:

```748:763:../../../Hercules/src/api/aclif.c
if ((sd->handler->flags & REQ_AUTH_TOKEN) != 0) {
    ...
    char *token = NULL;
    if (!aclif->get_post_header_data_str(sd, POST_AUTH_TOKEN, &token, NULL)) {
        ...
    }

    if (memcmp(login_data->auth_token, token, AUTH_TOKEN_SIZE) != 0) {
        ShowError("Wrong auth token %d: '%s'\n", fd, token);
        return false;
    }
}
```

An empty value has a one-byte allocation; a one-byte value has a two-byte
allocation; both are read as 16 bytes. That is a remote heap out-of-bounds read
and undefined behavior in the C API server.

**Reachability:** header validation first looks up `AccountId` in the API
`online_db` (`aclif.c:684–709`). Therefore the attacker needs the id of an
account that is currently online, but does **not** need its token. The generic
bug is reachable through every URL with `REQ_API_AUTH` or `REQ_AUTH_TOKEN`.

**Impact:** the directly established impact is undefined behavior and a
possible API-server crash. This review did not establish token disclosure or an
authentication bypass. `memcmp` is also not constant-time, but a network timing
attack against a 16-byte random token was not demonstrated and is not needed
for this finding.

**Fix and acceptance criteria:**

1. Request `data_size` and reject unless it equals `AUTH_TOKEN_SIZE` before any
   comparison.
2. Compare with a reviewed constant-time 16-byte helper.
3. Do not print token material in the error log.
4. Add 0-, 1-, 15-, 16-, and 17-byte cases under ASan/UBSan. All non-16-byte
   inputs must fail before comparison; a wrong 16-byte token must return the
   normal authorization error without a sanitizer finding.

**CWE:** CWE-125 Out-of-bounds Read; CWE-20 Improper Input Validation.

### P4-3. Unlimited chat input feeds unbounded full-history client work

The third pass recorded `min_chat_delay: 0` only as a low/informational chat
configuration note. End-to-end tracing shows a resource-exhaustion chain from a
normal player through Hercules into every nearby korangar client.

The server configuration explicitly disables its shared whisper/global/party/
guild delay:

```41:43:../../../Hercules/conf/map/battle/client.conf
// Minimum delay between whisper/global/party/guild messages (in milliseconds)
// Messages that break this threshold are silently omitted.
min_chat_delay: 0
```

The enforcement branch is skipped when the value is zero
(`pc.c:12584–12589`). Accepted global messages are then broadcast to nearby
clients (`clif.c:12151–12163`). Packet size is bounded, but message **count** is
not rate-limited here.

korangar keeps all messages for the process lifetime:

```263:264:../../korangar/src/state/mod.rs
/// List of all received chat messages.
chat_messages: Vec<ChatMessage>,
```

Each received public message appends with no cap or eviction:

```4206:4209:../../korangar/src/lib.rs
NetworkEvent::ChatMessage { text, color } => {
    self.client_state
        .follow_mut(client_state().chat_messages())
        .push(ChatMessage::new(text, color));
}
```

Party messages and whispers append to the same vector
(`korangar/src/lib.rs:5579–5582`, `5629–5630`). No `clear`, `truncate`, `drain`,
`retain`, `remove`, or `pop` operation exists for this history.

The chat layout measures every stored string and collects every height on a
layout pass, then traverses the complete list again to add the text:

```64:102:../../korangar/src/interface/windows/chat.rs
let message_heights = chat_messages
    .iter()
    .map(|chat_message| {
        ...
        resolver.get_text_dimensions(&chat_message.text, ...)
        ...
    })
    .collect();
```

```113:158:../../korangar/src/interface/windows/chat.rs
let chat_messages = state.get(&self.chat_messages_path);
...
chat_messages
    .iter()
    .zip(layout_info.message_heights.iter())
    .for_each(|(chat_message, message_height)| {
        ...
        layout.add_text(...);
    });
```

**Reachability:** any authenticated group-0 player can send ordinary chat. A
public-message flood affects every korangar client in `AREA_CHAT_WOC`; party and
whisper routes provide targeted variants. No GM command permission is needed.

**Impact:** unbounded client memory growth plus O(n) text measurement/layout as
history grows. Repeated additions make the normal reactive layout path
progressively more expensive. The same stream also consumes map-server and
network resources, although this pass did not load-test their ceiling.

**Fix and acceptance criteria:**

1. Set `min_chat_delay` to a nonzero baseline (250–500 ms is reasonable for the
   friends server) and add a burst-aware token bucket if legitimate rapid chat
   must remain usable.
2. Store chat in a bounded `VecDeque` or equivalent. Keep a small visible/cache
   window and a documented hard maximum, for example 500–1,000 messages.
3. Avoid remeasuring unchanged history. Cache shaped text/layout by message and
   measure only new or width-invalidated rows.
4. Test thousands of received messages: retained count and memory must plateau,
   oldest rows must evict deterministically, and layout cost must not grow with
   total historical input after the cap.

**CWE:** CWE-400 Uncontrolled Resource Consumption; CWE-770 Allocation of
Resources Without Limits or Throttling.

### P4-4. Network-derived values reach `expect` / `unwrap` in korangar

Several concrete panic paths exist beyond the already-recorded uncapped
`#[repeating(count)]` allocation.

#### Invalid wire flags

Three `FromBytes` implementations parse attacker-controlled integer bits, then
panic instead of returning a `ConversionError`:

```1293:1296:../../ragnarok-packets/src/lib.rs
impl FromBytes for RegularItemFlags {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_reader)
            .map(|raw| Self::from_bits(raw).expect("Invalid equip position"))
    }
}
```

The same pattern is in `EquippableItemFlags` (`lib.rs:1359–1362`) and
`EquipPosition` (`lib.rs:4537–4540`). Bits outside each declared mask are enough
to panic the networking task. The `RegularItemFlags`/`EquippableItemFlags`
message also incorrectly says "equip position," making diagnosis harder.

#### Invalid inventory packet order

Inventory list and end handlers assume `InventoyStartPacket` has already
created state:

```544:546:../../korangar-networking/src/packet_versions/version_20220406.rs
move |packet: RegularItemListPacket| {
    let mut borrowed = inventory_items.borrow_mut();
    let (inv_type, items) = borrowed.as_mut().expect("Unexpected inventory packet");
```

The equippable list has the same `expect` at line 603; inventory end calls
`take().expect(...)` at line 660. A reordered, duplicated, or unsolicited server
packet panics instead of reporting a protocol error.

#### Unknown selected character id

After `NetworkEvent::CharacterSelected`, the main application looks up the
server-supplied `character_id` in the previously received slots and unwraps it:

```3818:3823:../../korangar/src/lib.rs
let character_information = self
    .client_state
    .follow(client_state().character_slots())
    .with_id(login_data.character_id)
    .cloned()
    .unwrap();
```

An id not present in the slot list panics the main client path. The nearby
`saved_login_data.as_ref().unwrap()` is also state-sensitive, although this pass
did not establish a server-only sequence that clears it before this event.

**Reachability:** these values normally come from trusted Hercules. A malicious
login/char/map server can send them directly. An active LAN attacker able to
modify the plaintext, unauthenticated TCP streams can do the same; H2/T2 already
explain why passive and active network attackers are within the current threat
model. An ordinary remote player was not shown to control another client's
inventory flags or character-selection reply through stock Hercules.

**Impact:** the packet conversion and inventory-state panics can terminate the
Tokio networking task and force a disconnect. The selected-character unwrap is
processed by the main application and can terminate the client. This is an
availability issue, not a demonstrated code-execution path.

**Fix and acceptance criteria:**

1. Replace each `from_bits(...).expect(...)` with a fallible conversion that
   returns the raw value in a packet/conversion error. If forward compatibility
   matters, explicitly retain unknown bits with `from_bits_retain` only after
   auditing every consumer.
2. Make inventory assembly a small explicit state machine. Unexpected list/end
   packets should log bounded metadata, discard/reset the partial transaction,
   and disconnect cleanly if resynchronization is unsafe.
3. Reject `CharacterSelected` when its id is absent from the advertised slots;
   show an error and return to character selection rather than connecting to
   the map server first.
4. Add regression tests for every invalid flag mask, list-before-start,
   end-before-start, duplicate start/end, and unknown selected id. Seed those
   cases into the decoder fuzzer requested since pass 1.

**CWE:** CWE-248 Uncaught Exception; CWE-20 Improper Input Validation.

---

## Dependency and maintenance triage — recorded, not promoted

### Rust advisory state

`cargo audit --no-fetch` still exits nonzero with four vulnerabilities:

| Advisory | Locked package | Reachability and disposition |
|---|---|---|
| `RUSTSEC-2026-0204` | `crossbeam-epoch 0.9.18` | **Active production graph** through `rayon`; already first-pass L1. The vulnerable operation is formatting an already-invalid epoch pointer. Upgrade to `>=0.9.20`, but no new external-input chain was found here |
| `RUSTSEC-2026-0194`, `-0195` | `quick-xml 0.39.2` | Old copy remains through the Linux `wayland-scanner` proc macro. korangar's direct parser is 0.41.0. Build-time only with trusted Wayland protocol XML; already M3/pass-2, not a shipped runtime parser |
| `RUSTSEC-2026-0185` | `quinn-proto 0.11.14` | Present in `Cargo.lock` as an optional reqwest/QUIC package, but `cargo tree -i quinn-proto --edges all --target all` prints no reverse dependency. Not in the active feature graph and not shipped; already first-pass L3 |

The six allowed warnings are the already-recorded unmaintained `cgmath`,
`paste`, and `ttf-parser`, plus unsoundness warnings for `anyhow`, `cgmath`, and
`memmap2`. The audit gate should still be made green: an ignored or lockfile-only
advisory needs a documented scoped ignore so future real advisories remain
visible.

### Bundled giflib

Hercules bundles giflib **5.2.1** (`3rdparty/libgif/gif_lib.h:16–18`) and the
current macOS `api-server` links it statically (`otool -L` lists no gif library).
Two 2026 advisories for this version were checked:

- CVE-2026-23868 is in `GifMakeSavedImage`;
- CVE-2026-26740 is in `EGifGCBToExtension`.

Hercules' emblem path is decode-only (`DGifOpen` / `DGifSlurp`). Repository
search found those two affected functions only in bundled declarations/
implementations, not under `src/`, so version match alone was not promoted to a
reachable finding. P4-1 is independent of those CVEs and is established from
the decoder's allocation order. Keep the bundled library current anyway.

References: [Ubuntu USN-8583-1](https://ubuntu.com/security/notices/USN-8583-1),
[CVE-2026-23868](https://nvd.nist.gov/vuln/detail/CVE-2026-23868),
[CVE-2026-26740](https://nvd.nist.gov/vuln/detail/CVE-2026-26740).

### Hercules upstream drift

The fork's merge base with local `upstream/stable` is `1d07ca6a2` (release
v2026.04, 2026-04-22). The local upstream tip is `410b9738c` (release v2026.07,
2026-07-21). This pass reviewed the intervening log for obvious security fixes
but did **not** perform a commit-by-commit C security audit or merge. Record this
as maintenance debt: periodically rebase/merge and review upstream security and
memory-safety changes instead of relying on an old fork point indefinitely.

---

## CWE map (this pass)

| Finding | CWE |
|---|---|
| P4-1 GIF decode before limits/authorization | CWE-400 Uncontrolled Resource Consumption |
| P4-2 short token fixed-length comparison | CWE-125 Out-of-bounds Read / CWE-20 Improper Input Validation |
| P4-3 unlimited chat history and layout | CWE-400 / CWE-770 Allocation Without Limits or Throttling |
| P4-4 network-derived panics | CWE-248 Uncaught Exception / CWE-20 Improper Input Validation |

---

## Recommended order (across all four passes)

The global order still starts with **rotate C1** and keep the API off non-loopback
interfaces (N3). For this pass:

1. **Contain and fix P4-1:** stop the API if it is not required; otherwise bind
   it to loopback, authorize guild ownership before decode, and enforce decoded
   pixel/frame budgets before allocation.
2. **Fix P4-2 in the same API patch:** exact 16-byte length gate,
   constant-time comparison, no token logging, sanitizer regression tests.
3. **Fix P4-3 on both sides:** nonzero server chat throttle and a bounded,
   incrementally laid-out client history.
4. **Fix P4-4:** remove panics from network conversion/state handling and add
   malformed-sequence regression/fuzz cases.
5. Upgrade/ignore dependencies deliberately and bring the Hercules fork forward
   after the externally reachable fixes are contained.

---

## Coverage after four passes

| Surface | Where recorded |
|---|---|
| Credentials, password storage/transport, distribution hygiene, initial Rust advisories | Pass 1 |
| API exposure/IDOR, MariaDB, DM commands, fork packets, host tools | Pass 2 |
| Lua/GRF, session tokens, official NPCs, campaign economy, instances | Pass 3 |
| API C memory/resource safety, chat resource chain, concrete packet panics, dependency reachability | **This pass** |
| Still not done | Coverage-guided packet/API fuzzing, sanitizer campaign, Windows pack execution, complete upstream Hercules CVE/commit sweep |
