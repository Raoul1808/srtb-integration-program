use iced::{
    widget::{column, container, text},
    Alignment, Application, Command, Length, Settings, Theme,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn program() -> iced::Result {
    console_log::init().expect("failed to initialize logger");
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    App::run(Settings::default())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {}

struct App;

impl Application for App {
    type Executor = iced::executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (Self, Command::none())
    }

    fn title(&self) -> String {
        "SRTB Integration Program".into()
    }

    fn update(&mut self, _message: Self::Message) -> iced::Command<Self::Message> {
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        let label = text("Hello, world!");

        let content_col = column![label].spacing(40).align_items(Alignment::Center);

        let version = text(format!("v{} - WASM build", VERSION)).size(10);
        let col = column![content_col, version].spacing(10.);

        container(col)
            .padding(20.)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
