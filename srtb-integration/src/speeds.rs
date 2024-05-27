use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::{srtb::RawSrtbFile, IntegrationError, Integrator, ParsingError, SpinDifficulty};

const SRTB_KEY: &str = "SpeedHelper_SpeedTriggers";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
struct SpeedTrigger {
    time: f32,
    speed_multiplier: f32,
    #[serde(rename = "InterpolateToNextTrigger")]
    interpolate: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SpeedTriggersData {
    triggers: Vec<SpeedTrigger>,
}

fn text_to_speeds(data: &str) -> Result<SpeedTriggersData, IntegrationError> {
    let mut triggers = vec![];
    for (line_number, line) in data.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line: Vec<_> = line.split_whitespace().collect();
        if line.len() < 2 {
            return Err(IntegrationError::ParsingError(
                line_number,
                ParsingError::MissingArguments,
            ));
        }

        let time = line[0].parse().map_err(|_| {
            IntegrationError::ParsingError(line_number, ParsingError::InvalidFloat(line[0].into()))
        })?;
        let speed_multiplier = line[1].parse().map_err(|_| {
            IntegrationError::ParsingError(line_number, ParsingError::InvalidFloat(line[1].into()))
        })?;
        let interpolate = if line.len() >= 3 {
            line[2].parse().map_err(|_| {
                IntegrationError::ParsingError(
                    line_number,
                    ParsingError::InvalidBool(line[2].into()),
                )
            })?
        } else {
            false
        };
        triggers.push(SpeedTrigger {
            time,
            speed_multiplier,
            interpolate,
        });
    }

    triggers.sort_by(|t1, t2| t1.time.total_cmp(&t2.time));
    Ok(SpeedTriggersData { triggers })
}

fn speeds_to_text(data: &SpeedTriggersData) -> String {
    data.triggers.iter().fold(String::new(), |mut output, t| {
        let _ = writeln!(
            output,
            "{} {} {}",
            t.time, t.speed_multiplier, t.interpolate
        );
        output
    })
}

fn make_key(diff: SpinDifficulty) -> String {
    if diff == SpinDifficulty::AllDifficulties {
        SRTB_KEY.to_string()
    } else {
        format!("{}_{}", SRTB_KEY, diff.to_string().to_uppercase())
    }
}

pub struct SpeedsIntegrator;

impl Integrator for SpeedsIntegrator {
    fn file_extension(&self) -> String {
        "speeds".into()
    }

    fn integrate(
        &self,
        chart: &mut RawSrtbFile,
        data: &str,
        diff: SpinDifficulty,
    ) -> Result<(), IntegrationError> {
        let full_data = text_to_speeds(data)?;
        let key = make_key(diff);
        let value = serde_json::to_string(&full_data).map_err(IntegrationError::SerdeJsonError)?;
        chart.set_large_string_value(&key, &value);
        Ok(())
    }

    fn extract(
        &self,
        chart: &RawSrtbFile,
        diff: SpinDifficulty,
    ) -> Result<String, IntegrationError> {
        let key = make_key(diff);
        let value = chart
            .get_large_string_value(&key)
            .ok_or(IntegrationError::MissingData)?;
        let data: SpeedTriggersData =
            serde_json::from_str(&value).map_err(IntegrationError::SerdeJsonError)?;
        let str = speeds_to_text(&data);
        Ok(str)
    }

    fn remove(
        &self,
        chart: &mut RawSrtbFile,
        diff: SpinDifficulty,
    ) -> Result<(), IntegrationError> {
        let key = make_key(diff);
        chart.remove_large_string_value(&key);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::speeds::{speeds_to_text, text_to_speeds, SpeedTrigger, SpeedTriggersData};

    #[test]
    fn to_speeds() {
        let speeds = r#"
        0 1
        1.5  2    false
        2    1.5  true
        "#;

        let expected_speeds = vec![
            SpeedTrigger {
                time: 0.,
                speed_multiplier: 1.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 1.5,
                speed_multiplier: 2.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 2.,
                speed_multiplier: 1.5,
                interpolate: true,
            },
        ];

        let speeds = text_to_speeds(speeds).unwrap();
        assert_eq!(speeds.triggers, expected_speeds);
    }

    #[test]
    fn to_text() {
        let triggers = vec![
            SpeedTrigger {
                time: 0.,
                speed_multiplier: 1.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 1.5,
                speed_multiplier: 2.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 2.,
                speed_multiplier: 1.5,
                interpolate: true,
            },
        ];
        let speeds = SpeedTriggersData { triggers };

        let expected_speeds = "0 1 false\n1.5 2 false\n2 1.5 true\n";

        let speeds = speeds_to_text(&speeds);
        assert_eq!(speeds, expected_speeds);
    }
}
