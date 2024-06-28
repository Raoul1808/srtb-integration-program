use color::ColorError;
use strum::Display;
use thiserror::Error;

pub(crate) mod color;

mod chroma;
mod speeds;
mod srtb;

pub use chroma::ChromaIntegrator;
pub use speeds::SpeedsIntegrator;
pub use srtb::RawSrtbFile;

#[derive(Debug, Default, Display, PartialEq, Eq, Clone, Copy, Hash)]
pub enum SpinDifficulty {
    Easy,
    Normal,
    Hard,
    Expert,
    XD,
    RemiXD,
    #[default]
    #[strum(serialize = "All Difficulties")]
    AllDifficulties,
}

impl SpinDifficulty {
    pub const ALL: [Self; 7] = [
        Self::Easy,
        Self::Normal,
        Self::Hard,
        Self::Expert,
        Self::XD,
        Self::RemiXD,
        Self::AllDifficulties,
    ];
}

pub trait Integrator {
    fn file_extension(&self) -> String;
    fn integrate(
        &self,
        chart: &mut RawSrtbFile,
        data: &str,
        diff: SpinDifficulty,
    ) -> Result<(), IntegrationError>;
    fn extract(
        &self,
        chart: &RawSrtbFile,
        diff: SpinDifficulty,
    ) -> Result<String, IntegrationError>;
    fn remove(&self, chart: &mut RawSrtbFile, diff: SpinDifficulty)
        -> Result<(), IntegrationError>;
}

#[derive(Error, Debug)]
pub enum IntegrationError {
    #[error("io error: {0}")]
    IoError(std::io::Error),

    #[error("json serialization error: {0}")]
    SerdeJsonError(serde_json::Error),

    #[error("parsing error on line {0}: {1}")]
    ParsingError(usize, ParsingError),

    #[error("no integrated data found")]
    MissingData,

    #[error("operation cancelled")]
    Cancelled,
}

#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("not enough arguments")]
    MissingArguments,

    #[error("color variable \"{0}\" does not exist")]
    ColorVariableNotFound(String),

    #[error("invalid color variable name: {0}")]
    InvalidColorVariableName(String),

    #[error("color error: {0}")]
    ColorError(ColorError),

    #[error("invalid floating-point number: {0}")]
    InvalidFloat(String),

    #[error("invalid boolean: {0}")]
    InvalidBool(String),

    #[error("invalid note type: {0}")]
    InvalidNote(String),

    #[error("no default color for note type {0}")]
    NoDefaultColorForNote(String),

    #[error("no trigger in store for note type {0}")]
    NoTriggerForNote(String),

    #[error("unrecognized command: {0}")]
    UnrecognizedCommand(String),
}
