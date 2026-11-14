// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use voxflow_app_lib::CliArgs;

fn main() {
    // Let users pick GTK's display backend even inside the AppImage, whose
    // launch hook forces GDK_BACKEND=x11 over the user's own value (Handy
    // #1729, #2034; hook workaround for tauri-apps/tauri#8541). Runs first so
    // GTK never sees the forced value.
    #[cfg(target_os = "linux")]
    voxflow_app_lib::apply_voxflow_gdk_backend();

    #[cfg(target_os = "linux")]
    {
        // DMABUF renderer causes crashes on various GPU/display server configurations
        // See: https://github.com/tauri-apps/tauri/issues/9394
        // Preserve explicit overrides so affected users can opt out with =0 (#2166).
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Avoid overlay/capture layer crashes (#2049). Set before backend
        // initialization, preserving user overrides.
        if std::env::var_os("VK_LOADER_LAYERS_DISABLE").is_none()
            && !voxflow_app_lib::env_flag_enabled("VOXFLOW_KEEP_VULKAN_IMPLICIT_LAYERS")
        {
            std::env::set_var("VK_LOADER_LAYERS_DISABLE", "~implicit~");
        }
    }

    #[cfg(target_os = "linux")]
    voxflow_app_lib::disable_gamescope_wsi_layer();

    // Transcription worker: runs transcribe.cpp in isolation and never touches
    // Tauri, the CLI, or single-instance handling. Checked after the env setup
    // above so the worker inherits it (e.g. the Vulkan layer opt-out).
    if voxflow_app_lib::engine_supervisor::is_worker_invocation() {
        std::process::exit(voxflow_app_lib::engine_supervisor::run_worker());
    }

    let cli_args = CliArgs::parse();
    voxflow_app_lib::run(cli_args)
}
