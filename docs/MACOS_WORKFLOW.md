# Running on macOS

This fork is primarily developed against a WSL2/Windows machine (see the
WSL2 section in the root `CLAUDE.md`), but it also runs natively on macOS.
This doc covers the macOS-specific setup, gotchas, and the one client bug
that had to be fixed to make it work.

Repo layout on the macOS dev machine:
- `Hercules/` — the Hercules RO server (login/char/map/api), sibling directory
  to `korangar/` under `Ragnarok_Online/`.
- `korangar/` — this client.

## Server (Hercules)

Prerequisite: MariaDB running locally with the Hercules schema already imported.
The client's `sclientinfo.xml` (see below) points at `127.0.0.1:6900`.

MariaDB **does not autostart** on this Mac (the Homebrew LaunchAgent was
unregistered on 2026-07-15), so start it by hand each boot:

```sh
brew services run mariadb    # start for this session
```

Use `run`, **not** `brew services start` — `start` re-registers the LaunchAgent and
silently turns autostart back on. `brew services stop mariadb` stops and unregisters.

If you restart MariaDB while Hercules is up, restart Hercules too: the servers stay
alive but keep dead DB handles (`MYSQL_OPT_RECONNECT` is deprecated and shouldn't be
relied on). Sanity check with
`select id, host, command from information_schema.processlist;` — a healthy running
stack shows ~6 `ragnarok` connections.

**Building** (must match the client's packet version — see
`PLATFORM_BRINGUP.md` item 0; a plain `./configure` builds an incompatible
PACKETVER 20190605 server):

```sh
brew install mariadb pcre     # once
CPPFLAGS="-I/opt/homebrew/include" LDFLAGS="-L/opt/homebrew/lib" \
  ./configure --enable-packetver=20220406
make -j8
```

The `CPPFLAGS`/`LDFLAGS` are required — Homebrew's include/lib dirs aren't
on clang's default search path, and configure dies with "PCRE header not
found" without them.

From `Hercules/`:

```sh
./athena-start start     # starts login-server, char-server, map-server, api-server
./athena-start stop      # stops them (uses .{name}.pid files it wrote on start)
./athena-start restart
```

Notes:
- `athena-start stop` only works if the servers were started by
  `athena-start` in the first place (it kills by the `.pid` files it wrote).
  If you find servers already running that *weren't* started this way
  (e.g. duplicated/orphaned processes from a previous manual launch), find
  and `kill` them by PID first — check with:
  ```sh
  pgrep -fl "login-server|char-server|map-server|api-server"
  ```
- Healthy state is exactly one PID per server, listening on:
  login `6900`, char `6121`, map `5121`, api `7121`
  (`lsof -nP -iTCP -sTCP:LISTEN | grep -E '6900|6121|5121|7121'`).
- Logs are in `Hercules/log/{login,char,map}.log`. A crash-looped
  login-server manifests as a `socket.c:855 rfifoskip` assertion repeating.

## Client (Korangar)

No special environment variables are needed on macOS — unlike WSL2, there's
no Vulkan/D3D12 translation layer to fight. wgpu picks the native Metal
backend automatically.

```sh
cd korangar/korangar   # note: nested dir, this is where data.grf/rdata.grf/client live
cargo build --release --bin korangar
./../target/release/korangar
```

(or `cargo run --release --bin korangar` from `korangar/korangar/`.)

English item names are compiled into Korangar from the Hercules-derived
`docs/items.json` table. No external `System/itemInfo_EN.lua`, environment
variable, or platform-specific symlink is required. GRFs still provide the item
icon/resource paths.

Verify hardware rendering in the startup log:
```
using adapter Apple M5 Max (metal)
```

The client reads server connection info from
`korangar/archive/data/sclientinfo.xml` (`<address>127.0.0.1</address>`,
`<port>6900</port>`) — this is what makes it find the local Hercules login
server without any CLI flags (the binary only takes `--sync-cache`,
`-h`/`--help`, `-V`/`--version`).

### Known issue (fixed in-tree): startup panic on first launch

**Symptom**: the client panics almost immediately on launch with:
```
thread 'main' panicked at korangar/src/graphics/engine.rs:766:31:
called `Option::unwrap()` on a `None` value
```
(inside `get_window_size`), with a backtrace through
`winit::platform_impl::macos::view::WinitView::draw_rect`.

**Cause**: on macOS, AppKit can synchronously invoke `drawRect:` on the
window's view — which winit surfaces as `WindowEvent::RedrawRequested` —
*while still inside* `event_loop.create_window(...)` in the `resumed()`
handler (`korangar/src/lib.rs`), i.e. before `GraphicsEngine::on_resume` has
run and created the wgpu surface. Every render-path method
(`get_window_size`, `wait_for_next_frame`, etc.) assumed the surface always
exists by the time a redraw could happen — true on Windows/Linux, not
guaranteed on macOS.

**Fix**: added `GraphicsEngine::is_ready_to_render()`
(`korangar/src/graphics/engine.rs`, checks `self.surface.is_some()`) and
guarded the `WindowEvent::RedrawRequested` arm in
`ApplicationHandler::window_event` (`korangar/src/lib.rs`) to skip the
spurious frame instead of rendering.

**Follow-up bug in the first version of that fix** (symptom: window opens,
music plays, screen stays blank forever): the redraw loop is
*self-sustaining* — each handled `RedrawRequested` schedules the next via
`window.request_redraw()`. The early spurious event was the only
OS-initiated one, so silently dropping it meant the loop never started and
nothing ever rendered. The guard therefore re-requests a redraw while
waiting, and `resumed()` explicitly calls `window.request_redraw()` after
creating the surface. Keep both if touching this code.

If this fork is rebased against upstream Korangar and upstream fixes this
differently (e.g. by not marking `create_window`'s attributes `visible:
false`, or by restructuring `resumed`), this guard can likely be dropped —
but it's harmless to keep either way.

## First login on a fresh database

See `PLATFORM_BRINGUP.md` (items 2–5) for the full story; the short
version:

- A fresh DB has no playable accounts (only the `s1` inter-server account).
  Create one via SQL or the `<name>_M` self-registration suffix.
- A failed login shows "Incorrect username or password" and can be retried
  directly (the silent-wedge bug this section used to describe was a missing
  0x0B02 packet mapping, fixed 2026-07-10 — see PLATFORM_BRINGUP.md item 4).
- Pick a character server within **30 seconds** of logging in — the auth
  token expires (PLATFORM_BRINGUP.md item 4b). If rejected, the client
  returns to the login window automatically.
- Check the SQL `loginlog` table to see what the server actually received.

## Process management cheatsheet

```sh
# server
pgrep -fl "login-server|char-server|map-server|api-server"
cd Hercules && ./athena-start stop && ./athena-start start

# client
pgrep -fl korangar
kill <pid>   # no special shutdown sequence needed
```
