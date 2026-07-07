# Requirements

### 🦀 Cargo

Cargo (the Rust package manager) can best be installed and managed through [Rustup](https://rustup.rs/).

Note that Korangar requires Rust nightly to compile. If you need help configuring Rustup accordingly, please
read [this stackoverflow issue](https://stackoverflow.com/questions/58226545/how-to-switch-between-rust-toolchains).

# OS-specific

### 🪟 Notes on Windows

On Windows you will need the following additional dependencies to be installed:

- Git (https://git-scm.com/downloads/win)
- Slangc (https://github.com/shader-slang/slang/releases)

Slangc is also part of the VulkanSDK and can be installed with it.
You can also use the windows packet manager winget to download the dependencies.

e.g.:

```powershell
winget install --id Git.Git -e --source winget
winget install --id KhronosGroup.VulkanSDK -e --source winget
```

### ❄️ Nix & NixOS

There is a `flake.nix` in the repository that exposes a dev shell with all dependencies for testing and running Korangar
on Linux and MacOS.

### 🐧 WSL (Windows Subsystem for Linux)

If you are developing or running Korangar inside WSL, you will need specific configurations for hardware rendering and audio to work correctly. Please read the [WSL Setup Guide](WSL.md) for detailed instructions.

# Compiling

You can compile Korangar by running:

```fish
cargo build --release
```

### 🪲 Debug tools

To gain access to Korangar's built-in developer tools, explicitly enable the `debug` feature:

```fish
cargo build --release --features debug
```

If your terminal supports Unicode, you might want to enable the `unicode` feature as well:

```fish
cargo build --release --features "debug unicode"
```

# Running

For information on how to use Korangar, please take a look at the wiki page called [Running](Running.md).
