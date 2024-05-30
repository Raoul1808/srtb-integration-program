#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::{widget::text, Sandbox, Settings, Size};

fn main() -> iced::Result {
    let mut settings = Settings::default();
    settings.window.size = Size::new(360., 450.);
    App::run(settings)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Message {}

struct App;

impl Sandbox for App {
    type Message = Message;

    fn new() -> Self {
        Self
    }

    fn title(&self) -> String {
        "SRTB Integration Program".into()
    }

    fn update(&mut self, message: Self::Message) {
        match message {}
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        text("Hello, world!").into()
    }
}
