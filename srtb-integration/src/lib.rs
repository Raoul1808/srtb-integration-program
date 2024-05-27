use strum::Display;
use thiserror::Error;

mod speeds;
mod srtb;

pub use speeds::SpeedsIntegrator;
pub use srtb::RawSrtbFile;

#[derive(Debug, Display, PartialEq, Eq, Clone, Copy, Hash)]
pub enum SpinDifficulty {
    Easy,
    Normal,
    Hard,
    Expert,
    XD,
    RemiXD,
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

    #[error("parsing error at line {0}: not enough arguments")]
    ArgumentsMissing(usize),

    #[error("parsing error at line {0}: invalid floating-point number")]
    InvalidFloat(usize),

    #[error("parsing error at line {0}: invalid boolean")]
    InvalidBool(usize),

    #[error("no integrated data found")]
    MissingData,
}
