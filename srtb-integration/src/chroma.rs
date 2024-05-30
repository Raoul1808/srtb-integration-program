use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use serde::{Deserialize, Serialize};

use crate::{
    color::{HslColor, RgbColor},
    IntegrationError, Integrator, ParsingError, RawSrtbFile, SpinDifficulty,
};

const SRTB_KEY: &str = "SpeenChroma_ChromaTriggers";

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

fn get_color(
    color_map: &HashMap<ChromaNoteType, HslColor>,
    note_type: ChromaNoteType,
    color_str: &str,
) -> Result<HslColor, ParsingError> {
    let color_str = color_str.to_lowercase();
    if color_str == "default" {
        return color_map
            .get(&note_type)
            .copied()
            .ok_or(ParsingError::NoDefaultColorForNote(note_type.to_string()));
    }
    if let Some(note_type) = color_str.strip_prefix("default") {
        let note_type = ChromaNoteType::from_str(note_type)?;
        return color_map
            .get(&note_type)
            .copied()
            .ok_or(ParsingError::NoDefaultColorForNote(note_type.to_string()));
    }
    let col = RgbColor::from_hex_str(&color_str).map_err(ParsingError::ColorError)?;
    let col = HslColor::from(col);
    Ok(col)
}

fn text_to_chroma(content: &str) -> Result<ChromaTriggersData, IntegrationError> {
    let mut default_colors = HashMap::new();
    let mut chroma_data = HashMap::new();
    for note_type in ChromaNoteType::ALL_NOTES {
        chroma_data.insert(note_type, vec![]);
    }

    for line in content.lines().enumerate() {
        let (line_number, line) = line;
        let line = line.trim().to_lowercase();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line: Vec<_> = line.split_whitespace().collect();
        if line.is_empty() || line[0].is_empty() {
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
                let color = HslColor::from(RgbColor::from_hex_str(line[2]).map_err(|e| {
                    IntegrationError::ParsingError(line_number, ParsingError::ColorError(e))
                })?);
                chroma_data
                    .get_mut(&note_type)
                    .unwrap()
                    .push(ChromaTrigger {
                        time: 0.,
                        duration: 0.,
                        start_color: color,
                        end_color: color,
                    });
                default_colors.insert(note_type, color);
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
                let time: f32 = line[2].parse().map_err(|_| {
                    IntegrationError::ParsingError(
                        line_number,
                        ParsingError::InvalidFloat(line[2].into()),
                    )
                })?;
                let color = get_color(&default_colors, note_type, line[3])
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
                        let time: f32 = line[2].parse().map_err(|_| {
                            IntegrationError::ParsingError(
                                line_number,
                                ParsingError::InvalidFloat(line[2].into()),
                            )
                        })?;
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
                        let start_time: f32 = line[2].parse().map_err(|_| {
                            IntegrationError::ParsingError(
                                line_number,
                                ParsingError::InvalidFloat(line[2].into()),
                            )
                        })?;
                        let end_time: f32 = line[3].parse().map_err(|_| {
                            IntegrationError::ParsingError(
                                line_number,
                                ParsingError::InvalidFloat(line[3].into()),
                            )
                        })?;
                        let first_note_type = ChromaNoteType::from_str(line[4])
                            .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                        let second_note_type = ChromaNoteType::from_str(line[5])
                            .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                        let flash_col = RgbColor::from_hex_str(line[6]).map_err(|e| {
                            IntegrationError::ParsingError(line_number, ParsingError::ColorError(e))
                        })?;
                        let flash_col = HslColor::from(flash_col);
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
            _ => {
                if line.len() < 5 {
                    return Err(IntegrationError::ParsingError(
                        line_number,
                        ParsingError::MissingArguments,
                    ));
                }
                let note_type = ChromaNoteType::from_str(line[0])
                    .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                let start_time: f32 = line[1].parse().map_err(|_| {
                    IntegrationError::ParsingError(
                        line_number,
                        ParsingError::InvalidFloat(line[1].into()),
                    )
                })?;
                let end_time: f32 = line[2].parse().map_err(|_| {
                    IntegrationError::ParsingError(
                        line_number,
                        ParsingError::InvalidFloat(line[2].into()),
                    )
                })?;
                let start_color = get_color(&default_colors, note_type, line[3])
                    .map_err(|e| IntegrationError::ParsingError(line_number, e))?;
                let end_color = get_color(&default_colors, note_type, line[4])
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

fn chroma_to_text(_data: &ChromaTriggersData) -> String {
    "no extraction for now :(".into()
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
        chroma::{text_to_chroma, ChromaTrigger},
        color::HslColor,
    };

    #[test]
    fn to_chroma() {
        let chroma = r#"
        Start NoteA #ff0000
        Start NoteB #00ffff
        Instant NoteA 0.5 #00ffff
        NoteB 1.0 2.0 #00ffff #ff0000
        Swap Instant 3.0 NoteA NoteB
        Swap Flash 4.0 5.0 NoteA NoteB #ffffff
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
}
