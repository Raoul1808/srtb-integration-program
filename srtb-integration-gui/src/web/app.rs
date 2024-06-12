use std::sync::Arc;

use iced::{
    widget::{button, column, combo_box, container, radio, row, text},
    Alignment, Application, Command, Length, Settings, Theme,
};
use srtb_integration::{
    ChromaIntegrator, IntegrationError, Integrator, RawSrtbFile, SpeedsIntegrator, SpinDifficulty,
};
use strum::Display;

use super::file::save_file;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn program() -> iced::Result {
    console_log::init().expect("failed to initialize logger");
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    App::run(Settings::default())
}

#[derive(Debug, Display, Default, Clone, Copy, PartialEq, Eq)]
enum IntegratorKind {
    #[default]
    Speeds,
    Chroma,
}

impl IntegratorKind {
    const ALL: [Self; 2] = [Self::Speeds, Self::Chroma];

    pub fn ext(self) -> &'static str {
        match self {
            IntegratorKind::Speeds => "speeds",
            IntegratorKind::Chroma => "chroma",
        }
    }
}

#[derive(Debug, Display, Default, Clone, Copy, PartialEq, Eq)]
enum OperationKind {
    #[default]
    Integrate,
    Extract,
    Remove,
}

#[derive(Debug, Clone)]
enum Message {
    SelectIntegrator(IntegratorKind),
    RequestSelectChart,
    SelectedChart(Option<Arc<ReadFile>>),
    SelectDifficulty(SpinDifficulty),
    SelectOperation(OperationKind),
    RequestSelectExtraFile,
    SelectedExtraFile(Option<Arc<ReadFile>>),
    Process,
    None,
}

#[derive(Debug)]
struct ReadFile {
    name: String,
    content: String,
}

struct App {
    integrator_state: combo_box::State<IntegratorKind>,
    difficulty_state: combo_box::State<SpinDifficulty>,
    integrator_kind: Option<IntegratorKind>,
    difficulty: Option<SpinDifficulty>,
    operation: Option<OperationKind>,
    chart: Option<Arc<ReadFile>>,
    extra_file: Option<Arc<ReadFile>>,
}

struct ProcessData {
    integrator: IntegratorKind,
    diff: SpinDifficulty,
    op: OperationKind,
    in_file: Arc<ReadFile>,
    extra: Option<Arc<ReadFile>>,
}

impl App {
    async fn request_file<S: Into<String>, T: ToString>(
        filter_name: S,
        filter_ext: &[T],
    ) -> Option<Arc<ReadFile>> {
        let file = rfd::AsyncFileDialog::new()
            .add_filter(filter_name, filter_ext)
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

    async fn request_chart_file() -> Option<Arc<ReadFile>> {
        Self::request_file("Spin Rhythm Track Bundle", &["srtb"]).await
    }

    async fn request_extra_file(integrator: IntegratorKind) -> Option<Arc<ReadFile>> {
        Self::request_file(integrator.to_string(), &[integrator.ext()]).await
    }

    async fn process(data: ProcessData) {
        match Self::try_process(data).await {
            Ok(_) => {
                rfd::AsyncMessageDialog::new()
                    .set_level(rfd::MessageLevel::Info)
                    .set_description("operation complete")
                    .show()
                    .await
            }
            Err(e) => {
                rfd::AsyncMessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_description(format!("an error occurred: {}", e))
                    .show()
                    .await
            }
        };
    }

    async fn try_process(data: ProcessData) -> Result<(), IntegrationError> {
        let ProcessData {
            integrator: integrator_kind,
            diff,
            op,
            in_file,
            extra,
        } = data;

        let integrator: Box<dyn Integrator> = match integrator_kind {
            IntegratorKind::Speeds => Box::new(SpeedsIntegrator),
            IntegratorKind::Chroma => Box::new(ChromaIntegrator),
        };
        let mut chart = RawSrtbFile::from_bytes(in_file.content.as_bytes())?;
        let in_file_no_ext = in_file.name.strip_suffix(".srtb").unwrap_or(&in_file.name);
        match op {
            OperationKind::Integrate => {
                let data = &extra.unwrap().content;
                integrator.integrate(&mut chart, data, diff)?;
                let filename = format!(
                    "{}_INTEGRATED_{}.srtb",
                    in_file_no_ext,
                    integrator_kind.to_string().to_uppercase()
                );
                save_file(&filename, &chart.to_bytes()?);
            }
            OperationKind::Extract => {
                let data = integrator.extract(&chart, diff)?;
                let filename = format!(
                    "{}_EXTRACTED_{}.{}",
                    in_file_no_ext,
                    integrator_kind.to_string().to_uppercase(),
                    integrator_kind.ext()
                );
                save_file(&filename, data.as_bytes());
            }
            OperationKind::Remove => {
                integrator.remove(&mut chart, diff)?;
                let filename = format!(
                    "{}_REMOVED_{}.srtb",
                    in_file_no_ext,
                    integrator_kind.to_string().to_uppercase()
                );
                save_file(&filename, &chart.to_bytes()?);
            }
        }

        Ok(())
    }
}

impl Application for App {
    type Executor = iced::executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            Self {
                integrator_state: combo_box::State::new(IntegratorKind::ALL.to_vec()),
                difficulty_state: combo_box::State::new(SpinDifficulty::ALL.to_vec()),
                integrator_kind: None,
                difficulty: None,
                operation: None,
                chart: None,
                extra_file: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "SRTB Integration Program".into()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::SelectIntegrator(integrator) => {
                self.integrator_kind = Some(integrator);
                Command::none()
            }
            Message::RequestSelectChart => {
                Command::perform(Self::request_chart_file(), Message::SelectedChart)
            }
            Message::SelectedChart(file) => {
                if file.is_some() {
                    self.chart = file;
                }
                Command::none()
            }
            Message::SelectDifficulty(diff) => {
                self.difficulty = Some(diff);
                Command::none()
            }
            Message::SelectOperation(op) => {
                self.operation = Some(op);
                Command::none()
            }
            Message::RequestSelectExtraFile => Command::perform(
                Self::request_extra_file(self.integrator_kind.unwrap_or_default()),
                Message::SelectedExtraFile,
            ),
            Message::SelectedExtraFile(file) => {
                if file.is_some() {
                    self.extra_file = file;
                }
                Command::none()
            }
            Message::Process => {
                if self.chart.is_none() {
                    return Command::none();
                }
                let in_file = self.chart.clone().unwrap();
                let extra = self.extra_file.clone();
                let data = ProcessData {
                    integrator: self.integrator_kind.unwrap_or_default(),
                    diff: self.difficulty.unwrap_or_default(),
                    op: self.operation.unwrap_or_default(),
                    in_file,
                    extra,
                };
                Command::perform(Self::process(data), |_| Message::None)
            }
            Message::None => Command::none(),
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        let integrator_label = text("Integrator Type");
        let integrator_combo_box = combo_box(
            &self.integrator_state,
            "Integrator",
            self.integrator_kind.as_ref(),
            Message::SelectIntegrator,
        );
        let integrator_type_row = row![integrator_label, integrator_combo_box]
            .spacing(10)
            .align_items(Alignment::Center);

        let input_chart_label = text("Input Chart");
        let input_chart_button = button("Select").on_press(Message::RequestSelectChart);
        let input_chart_row = row![input_chart_label, input_chart_button]
            .spacing(10)
            .align_items(Alignment::Center);
        let selected_chart_label = text(format!(
            "Selected: {}",
            self.chart
                .as_ref()
                .map(|f| f.name.as_str())
                .unwrap_or("None")
        ));
        let full_input_chart_col = column![input_chart_row, selected_chart_label]
            .spacing(2)
            .align_items(Alignment::Center);

        let diff_label = text("Target Difficulty");
        let diff_combo_box = combo_box(
            &self.difficulty_state,
            "Difficulty...",
            self.difficulty.as_ref(),
            Message::SelectDifficulty,
        );
        let diff_row = row![diff_label, diff_combo_box]
            .spacing(10)
            .align_items(Alignment::Center);

        let radio_integrate = radio(
            "Integrate",
            OperationKind::Integrate,
            self.operation,
            Message::SelectOperation,
        );
        let radio_extract = radio(
            "Extract",
            OperationKind::Extract,
            self.operation,
            Message::SelectOperation,
        );
        let radio_remove = radio(
            "Remove",
            OperationKind::Remove,
            self.operation,
            Message::SelectOperation,
        );
        let radio_operation_col = column![radio_integrate, radio_extract, radio_remove]
            .spacing(10)
            .align_items(Alignment::Start);

        let is_integrating = self
            .operation
            .is_some_and(|o| o == OperationKind::Integrate);
        let extra_data_label = text("Extra File");
        let extra_data_button = button("Select")
            .on_press_maybe(is_integrating.then_some(Message::RequestSelectExtraFile));
        let extra_data_row = row![extra_data_label, extra_data_button]
            .spacing(10)
            .align_items(Alignment::Center);
        let selected_extra_label = text(format!(
            "Selected: {}",
            self.extra_file
                .as_ref()
                .map(|f| f.name.as_str())
                .unwrap_or("None")
        ));
        let full_extra_data_col = column![extra_data_row, selected_extra_label]
            .spacing(2)
            .align_items(Alignment::Center);

        let can_process = self.integrator_kind.is_some()
            && self.chart.is_some()
            && self.difficulty.is_some()
            && self.operation.is_some()
            && if let Some(OperationKind::Integrate) = self.operation {
                self.extra_file.is_some()
            } else {
                true
            };

        let process_button = button(text("PROCESS").size(24.))
            .padding(10)
            .on_press_maybe(can_process.then_some(Message::Process));

        let settings_col = column![
            integrator_type_row,
            full_input_chart_col,
            diff_row,
            radio_operation_col,
            full_extra_data_col,
        ]
        .spacing(20)
        .align_items(Alignment::Center);

        let content_col = column![settings_col, process_button]
            .spacing(40)
            .align_items(Alignment::Center);

        let version = text(format!("v{}", VERSION)).size(10);
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
