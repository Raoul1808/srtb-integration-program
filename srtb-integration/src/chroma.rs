use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Write},
};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    color::{HslColor, RgbColor},
    IntegrationError, Integrator, ParsingError, RawSrtbFile, SpinDifficulty,
};

const SRTB_KEY: &str = "SpeenChroma_ChromaTriggers";

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ChromaNoteType {
    NoteA,
    NoteB,
    Beat,
    SpinLeft,
    SpinRight,
    Scratch,
    Ancillary,
}

impl ChromaNoteType {
    pub const ALL_NOTES: [ChromaNoteType; 7] = [
        ChromaNoteType::NoteA,
        ChromaNoteType::NoteB,
        ChromaNoteType::Beat,
        ChromaNoteType::SpinLeft,
        ChromaNoteType::SpinRight,
        ChromaNoteType::Scratch,
        ChromaNoteType::Ancillary,
    ];

    pub fn from_str(note: &str) -> Result<ChromaNoteType, ParsingError> {
        use ChromaNoteType::*;
        let note = match note.to_lowercase().as_str() {
            "notea" => NoteA,
            "noteb" => NoteB,
            "beat" => Beat,
            "spinleft" | "leftspin" => SpinLeft,
            "spinright" | "rightspin" => SpinRight,
            "scratch" => Scratch,
            "ancillary" | "highlights" => Ancillary,
            _ => return Err(ParsingError::InvalidNote(note.into())),
        };
        Ok(note)
    }

    pub fn to_str_chroma(self) -> &'static str {
        use ChromaNoteType::*;
        match self {
            NoteA => "NoteA",
            NoteB => "NoteB",
            Beat => "Beat",
            SpinLeft => "SpinLeft",
            SpinRight => "SpinRight",
            Scratch => "Scratch",
            Ancillary => "Ancillary",
        }
    }
}

impl Display for ChromaNoteType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ChromaNoteType::*;
        let str = match self {
            NoteA => "Note A",
            NoteB => "Note B",
            Beat => "Beat",
            SpinLeft => "Left Spin",
            SpinRight => "Right Spin",
            Scratch => "Scratch",
            Ancillary => "Highlights",
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ChromaTrigger {
    time: f32,
    duration: f32,
    start_color: HslColor,
    end_color: HslColor,
}

impl ChromaTrigger {
    pub fn ensure_smooth_transition(&mut self) {
        if self.start_color.h == 0.
            && self.end_color.h != 0.
            && (self.start_color.s == 0. || self.start_color.l == 1.)
        {
            self.start_color.h = self.end_color.h;
        }
        if self.end_color.h == 0.
            && self.start_color.h != 0.
            && (self.end_color.s == 0. || self.end_color.l == 1.)
        {
            self.end_color.h = self.start_color.h;
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ChromaTriggersData {
    note_a: Vec<ChromaTrigger>,
    note_b: Vec<ChromaTrigger>,
    beat: Vec<ChromaTrigger>,
    spin_left: Vec<ChromaTrigger>,
    spin_right: Vec<ChromaTrigger>,
    scratch: Vec<ChromaTrigger>,
    ancillary: Vec<ChromaTrigger>,
}

fn make_key(diff: SpinDifficulty) -> String {
    if diff == SpinDifficulty::AllDifficulties {
        SRTB_KEY.to_string()
    } else {
        format!("{}_{}", SRTB_KEY, diff.to_string().to_uppercase())
    }
}

#[derive(Debug, Default)]
struct ChromaColorMaps {
    default_colors: HashMap<ChromaNoteType, HslColor>,
    variables: HashMap<String, HslColor>,
}

impl ChromaColorMaps {
    fn get_color(&self, color_str: &str) -> Result<HslColor, ParsingError> {
        let color_str = color_str.to_lowercase();
        if color_str.starts_with('#') {
            let col = RgbColor::from_hex_str(&color_str).map_err(ParsingError::ColorError)?;
            let col = HslColor::from(col);
            return Ok(col);
        }
        self.variables
            .get(&color_str)
            .copied()
            .ok_or(ParsingError::ColorVariableNotFound(color_str))
    }

    fn get_color_default_note(&self, color_str: &str) -> Result<HslColor, ParsingError> {
        let color_str = color_str.to_lowercase();
        if let Some(note_type) = color_str.strip_prefix("default") {
            let note_type = ChromaNoteType::from_str(note_type)?;
            return self
                .default_colors
                .get(&note_type)
                .copied()
                .ok_or(ParsingError::NoDefaultColorForNote(note_type.to_string()));
        }
        self.get_color(&color_str)
    }

    fn get_color_default(
        &self,
        note_type: ChromaNoteType,
        color_str: &str,
    ) -> Result<HslColor, ParsingError> {
        let color_str = color_str.to_lowercase();
        if color_str == "default" {
            return self
                .default_colors
                .get(&note_type)
                .copied()
                .ok_or(ParsingError::NoDefaultColorForNote(note_type.to_string()));
        }
        self.get_color_default_note(&color_str)
    }
}

fn text_to_chroma(content: &str) -> Result<ChromaTriggersData, IntegrationError> {
    let regex = Regex::new(r"(default)|([^a-zA-Z0-9\-_]+)").unwrap();
    let mut colors = ChromaColorMaps::default();
    let mut chroma_data = HashMap::new();
    for note_type in ChromaNoteType::ALL_NOTES {
        chroma_data.insert(note_type, vec![]);
    }

    let lines: Vec<_> = content.lines().collect();
    let mut line_number = 0;

    let mut repeating = false;
    let mut repeat_count = 0;
    let mut current_iteration = 0;
    let mut repeat_interval = 0.;
    let mut goto_line = 0;

    macro_rules! get_time {
        ($time:expr) => {{
            let time: f32 = $time.parse().map_err(|_| {
                IntegrationError::ParsingError(
                    line_number,
                    ParsingError::InvalidFloat($time.into()),
                )
            })?;
            let time = if repeating {
                time + repeat_interval * current_iteration as f32
            } else {
                time
            };
            Ok::<f32, IntegrationError>(time)
        }};
    }

    while line_number < lines.len() {
        let line = lines[line_number];
        let line = line.trim().to_lowercase();
        if line.is_empty() || line.starts_with('#') {
            line_number += 1;
            continue;
        }
        let line: Vec<_> = line.split_whitespace().collect();
        if line.is_empty() || line[0].is_empty() {
            line_number += 1;
            continue;
        }
        let verb = line[0];
        match verb {
            "start" => {
                if line.len() < 3 {
                    return Err(IntegrationError::ParsingError(
                        line_number,
                        ParsingError::MissingArguments,
                    ));
                }
                let note_type = ChromaNoteType::from_str(line[1])
                    .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                let color = colors
                    .get_color(line[2])
                    .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                chroma_data
                    .get_mut(&note_type)
                    .unwrap()
                    .push(ChromaTrigger {
                        time: 0.,
                        duration: 0.,
                        start_color: color,
                        end_color: color,
                    });
                colors.default_colors.insert(note_type, color);
            }
            "set" => {
                if line.len() < 3 {
                    return Err(IntegrationError::ParsingError(
                        line_number,
                        ParsingError::MissingArguments,
                    ));
                }
                let variable_name = line[1].to_string();
                let color = HslColor::from(RgbColor::from_hex_str(line[2]).map_err(|e| {
                    IntegrationError::ParsingError(line_number, ParsingError::ColorError(e))
                })?);
                if regex.is_match(&variable_name) {
                    return Err(IntegrationError::ParsingError(
                        line_number,
                        ParsingError::InvalidColorVariableName(variable_name),
                    ));
                }
                colors.variables.insert(variable_name.to_string(), color);
            }
            "instant" => {
                if line.len() < 4 {
                    return Err(IntegrationError::ParsingError(
                        line_number,
                        ParsingError::MissingArguments,
                    ));
                }
                let note_type = ChromaNoteType::from_str(line[1])
                    .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                let time = get_time!(line[2])?;
                let color = colors
                    .get_color_default(note_type, line[3])
                    .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                chroma_data
                    .get_mut(&note_type)
                    .unwrap()
                    .push(ChromaTrigger {
                        time,
                        duration: 0.,
                        start_color: color,
                        end_color: color,
                    });
            }
            "swap" => {
                if line.len() < 2 {
                    return Err(IntegrationError::ParsingError(
                        line_number,
                        ParsingError::MissingArguments,
                    ));
                }
                match line[1] {
                    "instant" => {
                        if line.len() < 5 {
                            return Err(IntegrationError::ParsingError(
                                line_number,
                                ParsingError::MissingArguments,
                            ));
                        }
                        let time = get_time!(line[2])?;
                        let first_note_type = ChromaNoteType::from_str(line[3])
                            .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                        let second_note_type = ChromaNoteType::from_str(line[4])
                            .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                        let (first_col, second_col) = {
                            let first_last_trigger = chroma_data
                                .get(&first_note_type)
                                .unwrap()
                                .last()
                                .ok_or(IntegrationError::ParsingError(
                                    line_number,
                                    ParsingError::NoTriggerForNote(second_note_type.to_string()),
                                ))?;
                            let second_last_trigger = chroma_data
                                .get(&second_note_type)
                                .unwrap()
                                .last()
                                .ok_or(IntegrationError::ParsingError(
                                    line_number,
                                    ParsingError::NoTriggerForNote(second_note_type.to_string()),
                                ))?;
                            (first_last_trigger.end_color, second_last_trigger.end_color)
                        };
                        chroma_data
                            .get_mut(&first_note_type)
                            .unwrap()
                            .push(ChromaTrigger {
                                time,
                                duration: 0.,
                                start_color: second_col,
                                end_color: second_col,
                            });
                        chroma_data
                            .get_mut(&second_note_type)
                            .unwrap()
                            .push(ChromaTrigger {
                                time,
                                duration: 0.,
                                start_color: first_col,
                                end_color: first_col,
                            });
                    }
                    "flash" => {
                        if line.len() < 7 {
                            return Err(IntegrationError::ParsingError(
                                line_number,
                                ParsingError::MissingArguments,
                            ));
                        }
                        let start_time = get_time!(line[2])?;
                        let end_time = get_time!(line[3])?;
                        let first_note_type = ChromaNoteType::from_str(line[4])
                            .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                        let second_note_type = ChromaNoteType::from_str(line[5])
                            .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                        let flash_col = colors
                            .get_color(line[6])
                            .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                        let (first_col, second_col) = {
                            let first_last_trigger = chroma_data
                                .get(&first_note_type)
                                .unwrap()
                                .last()
                                .ok_or(IntegrationError::ParsingError(
                                    line_number,
                                    ParsingError::NoTriggerForNote(second_note_type.to_string()),
                                ))?;
                            let second_last_trigger = chroma_data
                                .get(&second_note_type)
                                .unwrap()
                                .last()
                                .ok_or(IntegrationError::ParsingError(
                                    line_number,
                                    ParsingError::NoTriggerForNote(second_note_type.to_string()),
                                ))?;
                            (first_last_trigger.end_color, second_last_trigger.end_color)
                        };
                        chroma_data
                            .get_mut(&first_note_type)
                            .unwrap()
                            .push(ChromaTrigger {
                                time: start_time,
                                duration: end_time - start_time,
                                start_color: flash_col,
                                end_color: second_col,
                            });
                        chroma_data
                            .get_mut(&second_note_type)
                            .unwrap()
                            .push(ChromaTrigger {
                                time: start_time,
                                duration: end_time - start_time,
                                start_color: flash_col,
                                end_color: first_col,
                            });
                    }
                    _ => {
                        return Err(IntegrationError::ParsingError(
                            line_number,
                            ParsingError::UnrecognizedCommand(line[1].into()),
                        ))
                    }
                }
            }
            "repeat" => {
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

                if repeating {
                    return Err(IntegrationError::ParsingError(
                        line_number,
                        ParsingError::NoNestedRepeats,
                    ));
                }

                repeating = true;
                repeat_count = line[1].parse().map_err(|_| {
                    IntegrationError::ParsingError(
                        line_number,
                        ParsingError::InvalidInt(line[1].into()),
                    )
                })?;
                repeat_interval = line[3].parse().map_err(|_| {
                    IntegrationError::ParsingError(
                        line_number,
                        ParsingError::InvalidFloat(line[3].into()),
                    )
                })?;
                current_iteration = 0;
                goto_line = line_number;
            }
            "endrepeat" => {
                if !repeating {
                    return Err(IntegrationError::ParsingError(
                        line_number,
                        ParsingError::UnexpectedEndRepeat,
                    ));
                }

                current_iteration += 1;
                if current_iteration < repeat_count {
                    line_number = goto_line + 1;
                    continue;
                }

                repeating = false;
                repeat_count = 0;
                repeat_interval = 0.;
                goto_line = 0;
            }
            _ => {
                if line.len() < 5 {
                    return Err(IntegrationError::ParsingError(
                        line_number,
                        ParsingError::MissingArguments,
                    ));
                }
                let note_type = ChromaNoteType::from_str(line[0])
                    .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                let start_time = get_time!(line[1])?;
                let end_time = get_time!(line[2])?;
                let start_color = colors
                    .get_color_default(note_type, line[3])
                    .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                let end_color = colors
                    .get_color_default(note_type, line[4])
                    .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                let mut trigger = ChromaTrigger {
                    time: start_time,
                    duration: end_time - start_time,
                    start_color,
                    end_color,
                };
                trigger.ensure_smooth_transition();
                chroma_data.get_mut(&note_type).unwrap().push(trigger);
            }
        }
        line_number += 1;
    }

    for (_, trigger_data) in chroma_data.iter_mut() {
        trigger_data.sort_by(|a, b| a.time.total_cmp(&b.time));
    }

    {
        use ChromaNoteType::*;
        Ok(ChromaTriggersData {
            note_a: chroma_data.remove(&NoteA).unwrap(),
            note_b: chroma_data.remove(&NoteB).unwrap(),
            beat: chroma_data.remove(&Beat).unwrap(),
            spin_left: chroma_data.remove(&SpinLeft).unwrap(),
            spin_right: chroma_data.remove(&SpinRight).unwrap(),
            scratch: chroma_data.remove(&Scratch).unwrap(),
            ancillary: chroma_data.remove(&Ancillary).unwrap(),
        })
    }
}

fn chroma_to_text(data: &ChromaTriggersData) -> String {
    let mut notes = vec![];
    notes.extend(data.note_a.iter().map(|n| (ChromaNoteType::NoteA, n)));
    notes.extend(data.note_b.iter().map(|n| (ChromaNoteType::NoteB, n)));
    notes.extend(data.beat.iter().map(|n| (ChromaNoteType::Beat, n)));
    notes.extend(data.spin_left.iter().map(|n| (ChromaNoteType::SpinLeft, n)));
    notes.extend(
        data.spin_right
            .iter()
            .map(|n| (ChromaNoteType::SpinRight, n)),
    );
    notes.extend(data.scratch.iter().map(|n| (ChromaNoteType::Scratch, n)));
    notes.extend(
        data.ancillary
            .iter()
            .map(|n| (ChromaNoteType::Ancillary, n)),
    );
    notes.sort_by(|(_, t1), (_, t2)| t1.time.total_cmp(&t2.time));
    notes
        .iter()
        .fold(String::new(), |mut output, (note, trigger)| {
            let note = note.to_str_chroma();
            let src_col = RgbColor::from(trigger.start_color).hex();
            let dst_col = RgbColor::from(trigger.end_color).hex();
            let str = if trigger.time == 0. && trigger.duration == 0. {
                format!("Start {} {}", note, src_col)
            } else if trigger.duration == 0. {
                format!("Instant {} {:?} {}", note, trigger.time, dst_col)
            } else {
                format!(
                    "{} {:?} {:?} {} {}",
                    note,
                    trigger.time,
                    trigger.time + trigger.duration,
                    src_col,
                    dst_col
                )
            };
            let _ = writeln!(output, "{}", str);
            output
        })
}

pub struct ChromaIntegrator;

impl Integrator for ChromaIntegrator {
    fn file_extension(&self) -> String {
        "chroma".into()
    }

    fn integrate(
        &self,
        chart: &mut RawSrtbFile,
        data: &str,
        diff: SpinDifficulty,
    ) -> Result<(), IntegrationError> {
        let full_data = text_to_chroma(data)?;
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
        let data: ChromaTriggersData =
            serde_json::from_str(&value).map_err(IntegrationError::SerdeJsonError)?;
        let str = chroma_to_text(&data);
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
    use crate::{
        chroma::{chroma_to_text, text_to_chroma, ChromaTrigger, ChromaTriggersData},
        color::HslColor,
    };

    #[test]
    fn to_chroma() {
        let chroma = r#"
        Set red #ff0000
        Set cyan #00ffff
        Set white #ffffff
        Start NoteA red
        Start NoteB cyan
        Instant NoteA 0.5 cyan
        NoteB 1.0 2.0 cyan red
        Swap Instant 3.0 NoteA NoteB
        Swap Flash 4.0 5.0 NoteA NoteB white
        "#;

        let expected_note_a = vec![
            ChromaTrigger {
                time: 0.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 0.5,
                duration: 0.,
                start_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 3.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 4.,
                duration: 1.,
                start_color: HslColor {
                    h: 0.,
                    s: 0.,
                    l: 1.,
                },
                end_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
            },
        ];

        let expected_note_b = vec![
            ChromaTrigger {
                time: 0.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 1.,
                duration: 1.,
                start_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 3.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 4.,
                duration: 1.,
                start_color: HslColor {
                    h: 0.,
                    s: 0.,
                    l: 1.,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
        ];

        let chroma = text_to_chroma(chroma).unwrap();
        assert_eq!(chroma.note_a, expected_note_a);
        assert_eq!(chroma.note_b, expected_note_b);
    }

    #[test]
    fn to_text() {
        let note_a = vec![
            ChromaTrigger {
                time: 0.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 0.5,
                duration: 0.,
                start_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 3.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 4.,
                duration: 1.,
                start_color: HslColor {
                    h: 0.,
                    s: 0.,
                    l: 1.,
                },
                end_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
            },
        ];

        let note_b = vec![
            ChromaTrigger {
                time: 0.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 1.,
                duration: 1.,
                start_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 3.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.5,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 4.,
                duration: 1.,
                start_color: HslColor {
                    h: 0.,
                    s: 0.,
                    l: 1.,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
        ];

        let data = ChromaTriggersData {
            note_a,
            note_b,
            ..Default::default()
        };

        let expected_chroma = r#"Start NoteA #ff0000
Start NoteB #00ffff
Instant NoteA 0.5 #00ffff
NoteB 1.0 2.0 #00ffff #ff0000
Instant NoteA 3.0 #ff0000
Instant NoteB 3.0 #00ffff
NoteA 4.0 5.0 #ffffff #00ffff
NoteB 4.0 5.0 #ffffff #ff0000
"#;

        let chroma = chroma_to_text(&data);
        assert_eq!(chroma, expected_chroma);
    }

    #[test]
    fn to_chroma_repeat() {
        let chroma = r#"
        Repeat 3 interval 0.5
        Instant NoteA 0.0 #ff0000
        EndRepeat
        "#;

        let note_a = vec![
            ChromaTrigger {
                time: 0.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 0.5,
                duration: 0.,
                start_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
            ChromaTrigger {
                time: 1.,
                duration: 0.,
                start_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
                end_color: HslColor {
                    h: 0.,
                    s: 1.,
                    l: 0.5,
                },
            },
        ];

        let expected_chroma = ChromaTriggersData {
            note_a,
            ..Default::default()
        };

        let chroma = text_to_chroma(chroma).unwrap();
        assert_eq!(chroma, expected_chroma);
    }
}
