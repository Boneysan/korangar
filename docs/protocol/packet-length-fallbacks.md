# Automated packet-length fallbacks

Korangar frames incoming packets *by deserialization*: it reads a 2-byte header,
looks up a registered handler, and lets that handler consume the packet's bytes.
If a header has **no** registered handler, `process_one` returns
`UnhandledPacket`, the networking loop sets `cut_off_buffer_base = 0`, and **the
entire rest of the read buffer is dropped** (see
`korangar-networking/src/lib.rs`). That is the "silent framing desync" warned
about in `CLAUDE.md`: one unknown packet loses every packet queued behind it.

Hand-modeling every server packet before it can be received is a lot of toil.
The length-fallback mechanism removes the *framing* half of that problem
automatically, leaving only the *semantic* half (what a packet means) to do by
hand, and only for the packets you actually care about.

## How it works

1. **`tools/generate_packet_lengths.sh`** runs the C preprocessor over Hercules'
   own `common/packets_len.h` at the server's exact `PACKETVER`, using a
   redefined `packetLen(id, len)` macro to scrape every `(header, length)` pair
   the server knows how to frame. `length` is the total on-wire size including
   the 2-byte header, or `-1` for variable-length packets (whose real size is
   the 2-byte length field after the header). Output:
   `korangar-networking/src/packet_versions/lengths_20220406.rs`.

2. **`PacketHandler::register_length_fallbacks`** (in
   `ragnarok-packets/src/handler.rs`) is called last, after every dedicated
   handler is registered (end of `register_map_server_packets`). For any header
   **not already handled**, it installs a handler that consumes exactly the
   right number of bytes, then reports the packet through
   `PacketCallback::unknown_packet`.

The result: an unmodeled-but-known packet still shows up in the packet history /
`KORANGAR_PACKET_LOG` for auditing — exactly as before — but the buffer stays
framed and every packet behind it survives. Dedicated handlers always win
(fallbacks are skipped for headers that already have one), so promoting a packet
to a real, semantic handler is purely additive.

## Regenerating

```
tools/generate_packet_lengths.sh [HERCULES_DIR] [PACKETVER] [VARIANT]
```

Defaults: `~/GitHub/Hercules_RO`, `20220406`, `main`. The `PACKETVER`/`VARIANT`
**must** match how the server is built — the wire lengths come from the server's
compiled version, not the client's. `src/common/mmo.h` only shows the *default*
(20190605, behind `#ifndef`); a `--enable-packetver` build overrides it via
`-DPACKETVER=...` in the Makefiles, so verify with:

```
grep -o 'DPACKETVER=[0-9]*' <HERCULES_DIR>/Makefile | head -1
```

The server **must** be built `--enable-packetver=20220406`, `main` variant (no
`ENABLE_PACKETVER_RE/ZERO/SAK/AD`) to match the client — see
`docs/PLATFORM_BRINGUP.md` item 0 for why a default 20190605 build fails.
(History note: this table was originally scraped at 20190605, which matched
that era's claim about the server build; it was regenerated at 20220406 on
2026-07-10 — 1595 packets — when the server-version confusion was resolved.
The script now also runs on macOS: portable `mktemp`, `cc -E` instead of
GNU `cpp`.)

## Scope & caveats

- Registered on **all three** connections (login, character, map) since
  2026-07-10. The login/character flows were originally believed to be "short
  and fully modeled", which proved false twice on the same day: the login
  refusal `AC_REFUSE_LOGIN_R3` (0x0B02) and the character list
  `HC_ACK_CHARINFO_PER_PAGE` (0x099D) are both PACKETVER-dependent headers the
  20220406-oriented models missed, and each silently dropped the buffer.
- Fallbacks only fix **framing**, never **meaning**. A packet consumed by a
  fallback produces no `NetworkEvent`; model it explicitly when you need its
  contents.
- A high-frequency unmodeled packet will now be reported to `unknown_packet` on
  every arrival (previously it desynced once and the buffer was lost). That is
  the intended trade for buffer stability; model such a packet to silence it.
- The table is direction-agnostic (Hercules' length DB is a single id→len map),
  so it also contains client→server packets. Registering fallbacks for them is
  harmless: the server never sends them, so those entries never fire.
