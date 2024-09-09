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
    let mut line_number = 0;

    let mut repeat_depth = 0;
    let mut repeat_counts = Vec::<i32>::new();
    let mut current_iterations = vec![];
    let mut repeat_intervals = Vec::<f32>::new();
    let mut goto_line_buf = vec![];

    let lines: Vec<_> = data.lines().collect();
    while line_number < lines.len() {
        println!("Working on line {}", line_number);
        let line = lines[line_number];
        let line = line.to_lowercase();
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            line_number += 1;
            continue;
        }
        let line: Vec<_> = line.split_whitespace().collect();

        if line[0] == "repeat" {
            if line.len() < 4 {
                return Err(IntegrationError::ParsingError(
                    line_number,
                    ParsingError::MissingArguments,
                ));
            }

            if line[2] != "interval" {
                return Err(IntegrationError::ParsingError(
                    line_number,
                    ParsingError::InvalidRepeatCommand,
                ));
            }

            repeat_depth += 1;
            repeat_counts.push(line[1].parse().map_err(|_| {
                IntegrationError::ParsingError(
                    line_number,
                    ParsingError::InvalidInt(line[1].into()),
                )
            })?);
            repeat_intervals.push(line[3].parse().map_err(|_| {
                IntegrationError::ParsingError(
                    line_number,
                    ParsingError::InvalidFloat(line[3].into()),
                )
            })?);
            current_iterations.push(0);
            goto_line_buf.push(line_number);
            line_number += 1;
            continue;
        }

        if line[0] == "endrepeat" {
            if repeat_depth == 0 {
                return Err(IntegrationError::ParsingError(
                    line_number,
                    ParsingError::UnexpectedEndRepeat,
                ));
            }

            current_iterations[repeat_depth - 1] += 1;
            if current_iterations[repeat_depth - 1] < repeat_counts[repeat_depth - 1] {
                line_number = goto_line_buf[repeat_depth - 1] + 1;
                continue;
            }

            repeat_depth -= 1;
            repeat_counts.pop();
            repeat_intervals.pop();
            goto_line_buf.pop();
            line_number += 1;
            current_iterations.pop();
            continue;
        }

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
            time: if repeat_depth > 0 {
                let mut time = time;
                for i in 0..repeat_depth {
                    time += repeat_intervals[i] * current_iterations[i] as f32;
                }
                time
            } else {
                time
            },
            speed_multiplier,
            interpolate,
        });
        line_number += 1;
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

    #[test]
    fn to_speeds_repeat() {
        let speeds = r#"
        Repeat 3 interval 1.0
        0.0 0.0 true
        0.75 1.0 false
        EndRepeat
        "#;

        let expected_speeds = vec![
            SpeedTrigger {
                time: 0.,
                speed_multiplier: 0.,
                interpolate: true,
            },
            SpeedTrigger {
                time: 0.75,
                speed_multiplier: 1.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 1.,
                speed_multiplier: 0.,
                interpolate: true,
            },
            SpeedTrigger {
                time: 1.75,
                speed_multiplier: 1.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 2.,
                speed_multiplier: 0.,
                interpolate: true,
            },
            SpeedTrigger {
                time: 2.75,
                speed_multiplier: 1.,
                interpolate: false,
            },
        ];

        let speeds = text_to_speeds(speeds).unwrap();
        assert_eq!(speeds.triggers, expected_speeds);
    }

    #[test]
    fn nested_repeat() {
        let speeds = r#"
        Repeat 3 interval 2.0
        Repeat 2 interval 0.5
        1.0 0.5 true
        1.25 1.0 false
        EndRepeat
        EndRepeat
        "#;

        let expected_speeds = vec![
            SpeedTrigger {
                time: 1.,
                speed_multiplier: 0.5,
                interpolate: true,
            },
            SpeedTrigger {
                time: 1.25,
                speed_multiplier: 1.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 1.5,
                speed_multiplier: 0.5,
                interpolate: true,
            },
            SpeedTrigger {
                time: 1.75,
                speed_multiplier: 1.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 3.,
                speed_multiplier: 0.5,
                interpolate: true,
            },
            SpeedTrigger {
                time: 3.25,
                speed_multiplier: 1.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 3.5,
                speed_multiplier: 0.5,
                interpolate: true,
            },
            SpeedTrigger {
                time: 3.75,
                speed_multiplier: 1.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 5.,
                speed_multiplier: 0.5,
                interpolate: true,
            },
            SpeedTrigger {
                time: 5.25,
                speed_multiplier: 1.,
                interpolate: false,
            },
            SpeedTrigger {
                time: 5.5,
                speed_multiplier: 0.5,
                interpolate: true,
            },
            SpeedTrigger {
                time: 5.75,
                speed_multiplier: 1.,
                interpolate: false,
            },
        ];

        let speeds = text_to_speeds(speeds).unwrap();
        assert_eq!(speeds.triggers, expected_speeds);
    }
}
