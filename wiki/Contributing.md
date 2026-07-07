# Developing

> [!IMPORTANT]
> Korangar is purely a reverse engineering project. We **_do not_** allow any code that has been taken from or inspired by GRAVITY's intellectual property.

### 🚩 Issues
We have [some issues](https://github.com/vE5li/korangar/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) specifically for people wanting to contribute. Most of them are relatively small issues that (hopefully) offer a good introduction into the code base.

### 🧪 Tests
Although there are no issues regarding this, Korangar is severely lacking when it comes to tests of any sort. We wouldn't expect anyone to write tests but if you want to we are very thankful!

### 📝 Style
The entire project is formatted using [Rustfmt](https://github.com/rust-lang/rustfmt). Additionally, please make sure that all commit messages start with a capital letter. Other than that we don't have any specific rules on coding style but we strive to keep the look and feel of the code consistent.

### 🐧 WSL / GL Backend Caveats
If you are contributing rendering or shader code and developing under WSL, be aware of a few limitations of the WSL OpenGL backend:
- `TEXTURE_BINDING_ARRAY` and non-uniform indexing are unsupported; Korangar has fallback paths for this.
- GLSL via Naga cannot bind one texture to multiple samplers or use `GatherCmp` easily. Our shaders use texel-snapped linear sampling and `SampleCmp`-based PCF to work around this.
- MSAA resolve is completely broken and yields black screens. `GraphicsEngine::on_resume` clamps `Msaa::Off` if the GL backend is detected. 
Always test rendering code natively on Windows or Linux to ensure the Vulkan/DX12 paths perform as expected.

### 🪟 Windows Cross-Compilation
Native Windows toolchain development sometimes gets flagged by aggressive antivirus programs (like BitDefender). A recommended workflow is cross-compiling from WSL using `cargo-xwin` (`cargo xwin build --target x86_64-pc-windows-msvc`).
