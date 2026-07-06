use clap::Parser;
use korangar::Client;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Synchronize the asset cache and exit.
    #[arg(long)]
    sync_cache: bool,
}

fn main() {
    configure_graphics_environment();

    let arguments = Arguments::parse();
    let event_loop = (!arguments.sync_cache).then(Client::create_event_loop);

    if let Some(mut client) = Client::init(arguments.sync_cache, event_loop.as_ref()) {
        if let Some(event_loop) = event_loop {
            client.run(event_loop);
        }
    }
}

#[cfg(target_os = "linux")]
fn configure_graphics_environment() {
    let product_name = std::fs::read_to_string("/sys/class/dmi/id/product_name").unwrap_or_default();
    let is_vmware = product_name.to_lowercase().contains("vmware");

    if is_vmware && std::env::var_os("DISPLAY").is_some() {
        // VMware's Wayland/EGL path can expose the SVGA GL adapter without any
        // presentable surface formats. Prefer X11 when it is available.
        set_env("EGL_PLATFORM", "x11");
        set_env("WINIT_UNIX_BACKEND", "x11");
        remove_env("WAYLAND_DISPLAY");
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_graphics_environment() {}

#[cfg(target_os = "linux")]
fn set_env(key: &str, value: &str) {
    // This happens at process startup before Korangar creates threads.
    unsafe {
        std::env::set_var(key, value);
    }
}

#[cfg(target_os = "linux")]
fn remove_env(key: &str) {
    // This happens at process startup before Korangar creates threads.
    unsafe {
        std::env::remove_var(key);
    }
}
