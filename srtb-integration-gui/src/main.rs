#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs, path::PathBuf};

use iced::{
    widget::{button, column, combo_box, container, radio, row, text},
    Alignment, Length, Sandbox, Settings, Size,
};
use srtb_integration::{
    ChromaIntegrator, IntegrationError, Integrator, RawSrtbFile, SpeedsIntegrator, SpinDifficulty,
};
use strum::Display;

fn main() -> iced::Result {
    let mut settings = Settings::default();
    settings.window.size = Size::new(360., 512.);
    App::run(settings)
}

trait FilePathExt {
    fn file_name_string(&self) -> String;
}

impl FilePathExt for PathBuf {
    fn file_name_string(&self) -> String {
        self.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or("None".into())
    }
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum Message {
    SelectIntegrator(IntegratorKind),
    SelectChart,
    SelectDifficulty(SpinDifficulty),
    SelectOperation(OperationKind),
    SelectExtraFile,
    Process,
}

struct App {
    integrator_state: combo_box::State<IntegratorKind>,
    difficulty_state: combo_box::State<SpinDifficulty>,
    integrator_kind: Option<IntegratorKind>,
    difficulty: Option<SpinDifficulty>,
    operation: Option<OperationKind>,
    input_file: Option<PathBuf>,
    extra_file: Option<PathBuf>,
}

impl App {
    fn process(&self) -> Result<(), IntegrationError> {
        // Lots of unwrapping: this is bad practice, but it is checked before this function runs.
        let integrator_kind = self.integrator_kind.unwrap();
        let integrator: Box<dyn Integrator> = match integrator_kind {
            IntegratorKind::Speeds => Box::new(SpeedsIntegrator),
            IntegratorKind::Chroma => Box::new(ChromaIntegrator),
        };

        let diff = self.difficulty.unwrap();
        let operation = self.operation.unwrap();
        let in_file = self.input_file.as_ref().unwrap();
        let mut chart = RawSrtbFile::open(in_file)?;

        match operation {
            OperationKind::Integrate => {
                // This one is also checked
                let extra_data = self.extra_file.as_ref().unwrap();
                let extra_data =
                    fs::read_to_string(extra_data).map_err(IntegrationError::IoError)?;
                integrator.integrate(&mut chart, &extra_data, diff)?;
                let dest_file = rfd::FileDialog::new()
                    .add_filter("Spin Rhythm Track Bundle", &["srtb"])
                    .save_file()
                    .ok_or(IntegrationError::Cancelled)?;
                chart.save(dest_file)?;
            }
            OperationKind::Extract => {
                let data = integrator.extract(&chart, diff)?;
                let dest_file = rfd::FileDialog::new()
                    .add_filter(
                        format!("{} triggers file", integrator_kind),
                        &[integrator_kind.ext()],
                    )
                    .save_file()
                    .ok_or(IntegrationError::Cancelled)?;
                fs::write(dest_file, data).map_err(IntegrationError::IoError)?;
            }
            OperationKind::Remove => {
                integrator.remove(&mut chart, diff)?;
                let dest_file = rfd::FileDialog::new()
                    .add_filter("Spin Rhythm Track Bundle", &["srtb"])
                    .save_file()
                    .ok_or(IntegrationError::Cancelled)?;
                chart.save(dest_file)?;
            }
        }

        Ok(())
    }
}

impl Sandbox for App {
    type Message = Message;

    fn new() -> Self {
        Self {
            integrator_state: combo_box::State::new(IntegratorKind::ALL.to_vec()),
            difficulty_state: combo_box::State::new(SpinDifficulty::ALL.to_vec()),
            integrator_kind: None,
            difficulty: None,
            operation: None,
            input_file: None,
            extra_file: None,
        }
    }

    fn title(&self) -> String {
        "SRTB Integration Program".into()
    }

    fn update(&mut self, message: Self::Message) {
        use Message::*;
        match message {
            SelectIntegrator(integrator) => {
                self.integrator_kind = Some(integrator);
            }
            SelectChart => {
                self.input_file = rfd::FileDialog::new()
                    .add_filter("Spin Rhythm Track Bundle", &["srtb"])
                    .pick_file();
            }
            SelectDifficulty(diff) => {
                self.difficulty = Some(diff);
            }
            SelectOperation(op) => {
                self.operation = Some(op);
            }
            SelectExtraFile => {
                let kind = self.integrator_kind.unwrap_or_default();
                self.extra_file = rfd::FileDialog::new()
                    .add_filter(format!("{} triggers file", kind), &[kind.ext()])
                    .pick_file();
            }
            Process => {
                match self.process() {
                    Ok(_) => rfd::MessageDialog::new()
                        .set_title("All good")
                        .set_level(rfd::MessageLevel::Info)
                        .set_description("Operation completed successfully")
                        .show(),
                    Err(e) => rfd::MessageDialog::new()
                        .set_title("Error")
                        .set_level(rfd::MessageLevel::Error)
                        .set_description(format!("An error occurred: {}", e))
                        .show(),
                };
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
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
        let input_chart_button = button("Select").on_press(Message::SelectChart);
        let input_chart_row = row![input_chart_label, input_chart_button]
            .spacing(10)
            .align_items(Alignment::Center);
        let selected_chart_label = text(format!(
            "Selected: {}",
            self.input_file
                .as_ref()
                .map(|f| f.file_name_string())
                .unwrap_or("None".into())
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
        let extra_data_button =
            button("Select").on_press_maybe(is_integrating.then_some(Message::SelectExtraFile));
        let extra_data_row = row![extra_data_label, extra_data_button]
            .spacing(10)
            .align_items(Alignment::Center);
        let selected_extra_label = text(format!(
            "Selected: {}",
            self.extra_file
                .as_ref()
                .map(|f| f.file_name_string())
                .unwrap_or("None".into())
        ));
        let full_extra_data_col = column![extra_data_row, selected_extra_label]
            .spacing(2)
            .align_items(Alignment::Center);

        let can_process = self.integrator_kind.is_some()
            && self.input_file.is_some()
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

        let version = text(format!("v{}", env!("CARGO_PKG_VERSION"))).size(10);
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
