# Running Korangar on WSL (Windows Subsystem for Linux)

Running Korangar inside WSL is fully supported but requires some specific configurations. Without these, you might encounter slow performance (silent fallback to software rendering) or missing audio.

## 🚀 Hardware Acceleration (Video)

By default, if you just run `cargo run` inside WSL, Korangar will silently fall back to `llvmpipe` (CPU software rendering), which will severely impact performance.

To enable hardware rendering in WSL:

1. **Use the Wrapper Script**: We provide a `./run-wsl.sh` script in the root directory that sets the necessary environment variables. Always run Korangar via this script when using WSL.
2. **Under the Hood**: The script forces WSL to use the D3D12 passthrough GL driver instead of llvmpipe by setting:
   - `GALLIUM_DRIVER=d3d12`
   - `WGPU_BACKEND=gl`
   - `MESA_D3D12_DEFAULT_ADAPTER_NAME=<Your_GPU_Name>`

When you start Korangar, verify it's working by checking the console output for something like:
`using adapter D3D12 (NVIDIA GeForce RTX ...) (gl)`

### ⚠️ Known GL Backend Limitations on WSL
The WSL GL backend currently has a few caveats due to unsupported features (which Korangar works around with fallback paths):
- **MSAA**: Resolving MSAA silently produces black frames on the GL backend (the UI will be visible, but the world will be black). `Msaa::Off` is forced on GL to avoid this.
- **Performance Evaluation**: Since it uses a fallback rendering path without some advanced features, **do not use WSL to judge the engine's performance**. For native-level performance, test on Windows natively or bare-metal Linux.

## 🔊 Audio Setup (WSLg)

If you don't hear any sound and see errors like `cannot find card '0'` in the console at startup, you need to configure ALSA to route to PulseAudio (which WSLg uses).

1. Install the ALSA Pulse plugin:
   ```bash
   sudo apt install libasound2-plugins
   ```
2. Create or edit `~/.asoundrc` to route the default output to `pulse`:
   ```text
   pcm.!default pulse
   ctl.!default pulse
   ```

After this, Korangar audio should correctly pass through to Windows via WSLg.

## 📦 Windows Cross-Compilation

Native Windows toolchain development sometimes gets blocked by aggressive antivirus programs (like BitDefender) flagging build tooling. 

If you encounter this, an alternative is to cross-compile from WSL and run the resulting `.exe` natively on Windows. You can do this using `cargo-xwin`:
```bash
cargo xwin build --target x86_64-pc-windows-msvc --release
```
