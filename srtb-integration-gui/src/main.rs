#[cfg(not(target_arch = "wasm32"))]
mod desktop;

#[cfg(target_arch = "wasm32")]
mod web;

fn main() -> iced::Result {
    #[cfg(not(target_arch = "wasm32"))]
    desktop::program()?;

    #[cfg(target_arch = "wasm32")]
    web::program()?;

    Ok(())
}
