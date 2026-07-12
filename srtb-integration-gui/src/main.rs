#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

fn main() -> iced::Result {
    #[cfg(not(target_arch = "wasm32"))]
    desktop::program()?;

    Ok(())
}
