use std::sync::Arc;

use iced::{
    widget::{button, column, container, text},
    Alignment, Application, Command, Length, Settings, Theme,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn program() -> iced::Result {
    console_log::init().expect("failed to initialize logger");
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    App::run(Settings::default())
}

#[derive(Debug, Clone)]
enum Message {
    RequestFileOpen,
    FileOpened(Option<Arc<ReadFile>>),
    GoButton,
    None,
}

#[derive(Debug)]
struct ReadFile {
    name: String,
    content: String,
}

#[derive(Default)]
struct App {
    file: Option<Arc<ReadFile>>,
}

impl App {
    async fn request_file() -> Option<Arc<ReadFile>> {
        let file = rfd::AsyncFileDialog::new()
            .add_filter("text", &["txt"])
            .pick_file()
            .await?;
        let content = match String::from_utf8(file.read().await) {
            Ok(c) => c,
            Err(e) => {
                rfd::AsyncMessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_description(format!(
                        "file provided does not contain valid utf-8 characters. Detailed error: {}",
                        e
                    ))
                    .show()
                    .await;
                return None;
            }
        };
        let file = ReadFile {
            name: file.file_name(),
            content,
        };
        Some(Arc::new(file))
    }

    async fn test_dialog<S: Into<String>>(message: S) {
        rfd::AsyncMessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_description(message)
            .show()
            .await;
    }
}

impl Application for App {
    type Executor = iced::executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (Self::default(), Command::none())
    }

    fn title(&self) -> String {
        "SRTB Integration Program".into()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::FileOpened(file) => {
                self.file = file;
                Command::none()
            }
            Message::RequestFileOpen => Command::perform(App::request_file(), Message::FileOpened),
            Message::GoButton => match self.file.as_ref() {
                Some(f) => Command::perform(App::test_dialog(f.content.clone()), |_| Message::None),
                None => Command::none(),
            },
            Message::None => Command::none(),
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        let label = text("Hello, world!");

        let file_button = button("Pick File").on_press(Message::RequestFileOpen);
        let file_label = text(format!(
            "Selected: {}",
            self.file
                .as_ref()
                .map(|f| f.name.clone())
                .unwrap_or("None".into())
        ));
        let file_picker = column![file_button, file_label]
            .spacing(2)
            .align_items(Alignment::Start);

        let go_button = button("Go").on_press_maybe(self.file.is_some().then(|| Message::GoButton));

        let content_col = column![label, file_picker, go_button]
            .spacing(40)
            .align_items(Alignment::Center);

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
