mod app;
mod benchmark_page;
mod receiver_page;
mod sender_page;
mod settings;
mod ui_components;

use app::StareDropApp;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "staredrop=info,staredrop_app=info".into()),
        )
        .init();

    let force_x11 = std::env::var("STAREDROP_FORCE_X11").ok().as_deref() == Some("1");
    if let Err(first_err) = run_app(force_x11) {
        #[cfg(target_os = "linux")]
        {
            if !force_x11 && should_retry_with_x11(&first_err) {
                eprintln!("StareDrop: Wayland startup failed, retrying with X11 backend...");
                if !x11_runtime_available() {
                    eprintln!("X11 fallback unavailable: missing libxkbcommon-x11 runtime.");
                    print_linux_runtime_hints(&first_err.to_string());
                    return;
                }

                // Retry in a fresh process to avoid backend-global initialization state.
                let exe = match std::env::current_exe() {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Failed to locate executable for X11 retry: {err}");
                        print_linux_runtime_hints(&first_err.to_string());
                        return;
                    }
                };
                let status = std::process::Command::new(exe)
                    .env("STAREDROP_FORCE_X11", "1")
                    .status();
                match status {
                    Ok(exit) if exit.success() => return,
                    Ok(exit) => {
                        eprintln!("X11 fallback process exited with status: {exit}");
                        print_linux_runtime_hints(&first_err.to_string());
                    }
                    Err(err) => {
                        eprintln!("Failed to launch X11 fallback process: {err}");
                        print_linux_runtime_hints(&first_err.to_string());
                    }
                }
                return;
            }
        }

        eprintln!("Failed to start StareDrop app: {first_err}");
        #[cfg(target_os = "linux")]
        print_linux_runtime_hints(&first_err.to_string());
    }
}

fn run_app(force_x11: bool) -> Result<(), eframe::Error> {
    #[cfg(target_os = "linux")]
    if force_x11 {
        // Rust 2024: mutating process env is unsafe; this runs before GUI threads start.
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
            std::env::remove_var("WAYLAND_SOCKET");
        }
    }

    let mut native_options = eframe::NativeOptions::default();
    native_options.renderer = eframe::Renderer::Wgpu;
    #[cfg(target_os = "linux")]
    if force_x11 {
        native_options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            builder.with_x11();
        }));
    }

    eframe::run_native(
        "StareDrop (Phase 0/1 MVP)",
        native_options,
        Box::new(|cc| Box::new(StareDropApp::new(cc))),
    )
}

#[cfg(target_os = "linux")]
fn should_retry_with_x11(err: &eframe::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("wayland")
        || std::env::var("WAYLAND_DISPLAY")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn x11_runtime_available() -> bool {
    std::path::Path::new("/usr/lib/libxkbcommon-x11.so.0").exists()
        || std::path::Path::new("/lib/libxkbcommon-x11.so.0").exists()
        || std::path::Path::new("/usr/lib64/libxkbcommon-x11.so.0").exists()
}

#[cfg(target_os = "linux")]
fn print_linux_runtime_hints(error_message: &str) {
    let missing_vulkan_loader = !std::path::Path::new("/usr/lib/libvulkan.so.1").exists();
    if missing_vulkan_loader || error_message.to_lowercase().contains("libvulkan.so.1") {
        eprintln!(
            "Hint: Vulkan loader is missing. On Arch install: sudo pacman -S --needed vulkan-icd-loader vulkan-dzn vulkan-swrast"
        );
    }
    if error_message.to_lowercase().contains("wayland") {
        eprintln!(
            "Hint: Force X11 fallback with: STAREDROP_FORCE_X11=1 cargo run -p staredrop-app"
        );
    }
    if !x11_runtime_available() {
        eprintln!(
            "Hint: Missing X11 keyboard runtime. On Arch install: sudo pacman -S --needed libxkbcommon-x11"
        );
    }
}
