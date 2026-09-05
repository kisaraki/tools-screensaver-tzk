//! Native Win32 screensaver with packaging-safe installation integration (Phase 5).

#[cfg(not(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc")))]
compile_error!("This project requires the x86_64-pc-windows-msvc target.");

pub mod app;
pub mod cli;
pub mod config;
mod dialog;
pub mod error;
pub mod font;
mod gdi;
mod install;
pub mod layout;
mod lifecycle;
pub mod model;
mod monitor;
mod native;
pub mod registry;
mod render;
pub mod utf16;
mod window;

mod resource_ids {
    include!(concat!(env!("OUT_DIR"), "/resource_ids.rs"));
}
