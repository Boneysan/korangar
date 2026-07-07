# Known issues
Here is a list of issues that are well known and how to fix them.

### 🗺️ Map server currently unavailable
This is a server side issue. If you see this message please send me a message on [Discord](https://discord.gg/2CqRZsvKja).

### 🐢 Terrible performance on WSL / Linux
If you just use `cargo run`, it will silently fall back to `llvmpipe` (CPU software rendering). Please use `./run-wsl.sh` to enforce the D3D12/GL driver if you are playing via WSL. See the [WSL Setup Guide](WSL.md) for more info.

### ⬛ Black world screen / UI only on WSL
This is likely caused by the GL backend struggling to resolve MSAA on WSL. Korangar automatically clamps `Msaa::Off` when it detects GL, but if you forced settings in your config files, you must disable MSAA. 

### 🔇 No audio on WSL
If you get `cannot find card '0'` errors at startup and no audio plays, you are missing ALSA to PulseAudio routing. You need to install `libasound2-plugins` and configure `~/.asoundrc`. See the [WSL Setup Guide](WSL.md) for the exact steps.

# Getting help
If your issue is not listed here or you need help, feel free to ask for help on the [Korangar Discord server](https://discord.gg/2CqRZsvKja). We have a channel specifically for getting help called `#support`. Alternatively you can also create an issue on GitHub to report bugs or request features.
