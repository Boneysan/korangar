#!/usr/bin/env bash
# Launch Korangar under WSL2 with hardware GPU acceleration.
#
# Ubuntu's Mesa doesn't ship the Dozen (Vulkan-on-D3D12) driver, so Vulkan
# falls back to llvmpipe (CPU). OpenGL-on-D3D12 *is* shipped but not selected
# by default, so we force it and point it at the NVIDIA adapter instead of
# the integrated GPU.
set -euo pipefail
cd "$(dirname "$0")"

export GALLIUM_DRIVER=d3d12
export MESA_D3D12_DEFAULT_ADAPTER_NAME=NVIDIA
export WGPU_BACKEND=gl

exec cargo run --bin korangar "$@"
