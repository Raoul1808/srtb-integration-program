use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::{fmt::Write, fs, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSrtbFile {
    pub unity_object_values_container: UnityObjectValuesContainer,
    pub large_string_values_container: LargeStringValuesContainer,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityObjectValuesContainer {
    pub values: Vec<UnityObjectValue>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityObjectValue {
    pub key: String,
    pub json_key: String,
    pub full_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeStringValuesContainer {
    pub values: Vec<LargeStringValue>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeStringValue {
    pub key: String,
    pub val: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SpeedTriggersData {
    pub triggers: Vec<SpeedTrigger>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SpeedTrigger {
    pub time: f32,
    pub speed_multiplier: f32,
    pub interpolate_to_next_trigger: bool,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ChromaTriggersData {
    pub note_a: Vec<ChromaTrigger>,
    pub note_b: Vec<ChromaTrigger>,
    pub beat: Vec<ChromaTrigger>,
    pub spin_left: Vec<ChromaTrigger>,
    pub spin_right: Vec<ChromaTrigger>,
    pub scratch: Vec<ChromaTrigger>,
    pub ancillary: Vec<ChromaTrigger>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ChromaTrigger {
    pub time: f32,
    pub duration: f32,
    pub start_color: HslColor,
    pub end_color: HslColor,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "PascalCase")]
pub struct HslColor {
    pub h: f32,
    pub s: f32,
    pub l: f32,
}

impl HslColor {
    pub fn from_hex_rgb(hex: &str) -> Result<HslColor, String> {
        let mut hex = hex.to_string();
        if hex.starts_with("#") {
            hex = hex.replace("#", "");
        }
        if hex.len() != 6 {
            return Err(format!(
                "Invalid hex color code: expected 6 characters (excluding #), found {}",
                hex.len()
            ));
        }

        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex code".to_string())?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex code".to_string())?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex code".to_string())?;

        let r = r as f64 / 255.;
        let g = g as f64 / 255.;
        let b = b as f64 / 255.;

        let min = f64::min(r, f64::min(g, b));
        let max = f64::max(r, f64::max(g, b));
        let diff = max - min;
        let l = (max + min) / 2.;

        if diff == 0. {
            return Ok(HslColor {
                h: 0.,
                s: 0.,
                l: l as f32,
            });
        }

        let s = if l < 0.5 {
            diff / (max + min)
        } else {
            diff / (2. - max - min)
        };
        let r = (((max - r) / 6.) + (max / 2.)) / diff;
        let g = (((max - g) / 6.) + (max / 2.)) / diff;
        let b = (((max - b) / 6.) + (max / 2.)) / diff;

        let h = if r == max {
            b - g
        } else if g == max {
            (1. / 3.) + r - b
        } else if b == max {
            (2. / 3.) + g - r
        } else {
            0.
        };

        let h = if h < 0. {
            h + 1.
        } else if h > 1. {
            h - 1.
        } else {
            h
        };

        Ok(HslColor {
            h: h as f32,
            s: s as f32,
            l: l as f32,
        })
    }
}

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

    pub fn from_str(note: &str) -> Result<ChromaNoteType, String> {
        use ChromaNoteType::*;
        let note = match note.to_lowercase().as_str() {
            "notea" => NoteA,
            "noteb" => NoteB,
            "beat" => Beat,
            "spinleft" | "leftspin" => SpinLeft,
            "spinright" | "rightspin" => SpinRight,
            "scratch" => Scratch,
            "ancillary" | "highlights" => Ancillary,
            _ => return Err(format!("Invalid note type: {}", note)),
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

pub fn speeds_to_json(content: &str) -> Result<SpeedTriggersData, String> {
    let mut triggers = Vec::new();
    for line in content.lines().enumerate() {
        let (line_number, line) = line;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line: Vec<_> = line.split_whitespace().collect();
        if line.is_empty() {
            continue;
        }
        if line.len() < 2 || line.len() > 3 {
            return Err(format!(
                "Line {}: expected 2 or 3 values values, found {}",
                line_number,
                line.len()
            ));
        }
        let time = line[0].parse();
        let time: f32 = match time {
            Ok(t) => t,
            Err(_) => {
                return Err(format!(
                    "Line {}: time value is not a valid number",
                    line_number
                ))
            }
        };

        let speed = line[1].parse();
        let speed: f32 = match speed {
            Ok(s) => s,
            Err(_) => {
                return Err(format!(
                    "Line {}: speed multiplier is not a valid number",
                    line_number
                ))
            }
        };

        let interpolate = if line.len() != 3 {
            false
        } else {
            let interpolate = line[2].parse();
            match interpolate {
                Ok(i) => i,
                Err(_) => {
                    return Err(format!(
                        "Line {}: interpolation is not a valid boolean",
                        line_number
                    ))
                }
            }
        };

        let trigger = SpeedTrigger {
            time,
            speed_multiplier: speed,
            interpolate_to_next_trigger: interpolate,
        };
        println!("Created trigger {:?}", trigger);
        triggers.push(trigger);
    }
    let data = SpeedTriggersData { triggers };
    Ok(data)
}

pub fn json_to_speeds(speeds: &SpeedTriggersData) -> String {
    speeds.triggers.iter().fold(String::new(), |mut output, t| {
        let _ = writeln!(
            output,
            "{} {} {}",
            t.time, t.speed_multiplier, t.interpolate_to_next_trigger
        );
        output
    })
}

fn get_color(
    color_map: &HashMap<ChromaNoteType, HslColor>,
    note_type: ChromaNoteType,
    color_str: &str,
) -> Result<HslColor, String> {
    let color_str = color_str.to_lowercase();
    if color_str == "default" {
        return color_map
            .get(&note_type)
            .map(|col| *col)
            .ok_or(format!("no default color for note type {}", note_type));
    }
    if color_str.starts_with("default") {
        let note_type = ChromaNoteType::from_str(&color_str[7..])?;
        return color_map
            .get(&note_type)
            .map(|col| *col)
            .ok_or(format!("no default color for note type {}", note_type));
    }
    let col = HslColor::from_hex_rgb(&color_str)?;
    Ok(col)
}

pub fn chroma_to_json(content: &str) -> Result<ChromaTriggersData, String> {
    let mut default_colors = HashMap::new();
    let mut chroma_data = HashMap::new();
    for note_type in ChromaNoteType::ALL_NOTES {
        chroma_data.insert(note_type, vec![]);
    }

    for line in content.lines().enumerate() {
        println!("Parsing line {:?}", line);
        let (line_number, line) = line;
        let line = line.trim().to_lowercase();
        if line.is_empty() || line.starts_with("#") {
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
                    return Err(format!(
                        "Line {}: not enough arguments for trigger type `Start`",
                        line_number
                    ));
                }
                let note_type = ChromaNoteType::from_str(line[1])
                    .map_err(|e| format!("Line {}: {}", line_number, e))?;
                let color = HslColor::from_hex_rgb(line[2])
                    .map_err(|e| format!("Line {}: {}", line_number, e))?;
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
                    return Err(format!(
                        "Line {}: not enough arguments for trigger type `Instant`",
                        line_number
                    ));
                }
                let note_type = ChromaNoteType::from_str(line[1])
                    .map_err(|e| format!("Line {}: {}", line_number, e))?;
                let time: f32 = line[2]
                    .parse()
                    .map_err(|_| format!("Line {}: invalid trigger time", line_number))?;
                let color = get_color(&default_colors, note_type, line[3])
                    .map_err(|e| format!("Line {}: {}", line_number, e))?;
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
                    return Err(format!(
                        "Line {}: not enough arguments for trigger type `Swap`",
                        line_number
                    ));
                }
                match line[1] {
                    "instant" => {
                        if line.len() < 5 {
                            return Err(format!(
                                "Line {}: not enough arguments for trigger type `Swap Instant`",
                                line_number
                            ));
                        }
                        let time: f32 = line[2]
                            .parse()
                            .map_err(|_| format!("Line {}: invalid trigger time", line_number))?;
                        let first_note_type = ChromaNoteType::from_str(line[3])
                            .map_err(|e| format!("Line {}: {}", line_number, e))?;
                        let second_note_type = ChromaNoteType::from_str(line[4])
                            .map_err(|e| format!("Line {}: {}", line_number, e))?;
                        let (first_col, second_col) = {
                            let first_last_trigger =
                                chroma_data.get(&first_note_type).unwrap().last().ok_or(
                                    format!(
                                        "Line {}: no trigger for {}",
                                        line_number, first_note_type
                                    ),
                                )?;
                            let second_last_trigger =
                                chroma_data.get(&second_note_type).unwrap().last().ok_or(
                                    format!(
                                        "Line {}: no trigger for {}",
                                        line_number, second_note_type
                                    ),
                                )?;
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
                            return Err(format!(
                                "Line {}: not enough arguments for trigger type `Swap Flash`",
                                line_number
                            ));
                        }
                        let start_time: f32 = line[2]
                            .parse()
                            .map_err(|_| format!("Line {}: invalid trigger time", line_number))?;
                        let end_time: f32 = line[3]
                            .parse()
                            .map_err(|_| format!("Line {}: invalid trigger time", line_number))?;
                        let first_note_type = ChromaNoteType::from_str(line[4])
                            .map_err(|e| format!("Line {}: {}", line_number, e))?;
                        let second_note_type = ChromaNoteType::from_str(line[5])
                            .map_err(|e| format!("Line {}: {}", line_number, e))?;
                        let flash_col = HslColor::from_hex_rgb(line[6])
                            .map_err(|e| format!("Line {}: {}", line_number, e))?;
                        let (first_col, second_col) = {
                            let first_last_trigger =
                                chroma_data.get(&first_note_type).unwrap().last().ok_or(
                                    format!(
                                        "Line {}: no trigger for {}",
                                        line_number, first_note_type
                                    ),
                                )?;
                            let second_last_trigger =
                                chroma_data.get(&second_note_type).unwrap().last().ok_or(
                                    format!(
                                        "Line {}: no trigger for {}",
                                        line_number, second_note_type
                                    ),
                                )?;
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
                        return Err(format!(
                            "Line {}: unknown `Swap` trigger subtype `{}`",
                            line_number, line[1]
                        ))
                    }
                }
            }
            _ => {
                if line.len() < 5 {
                    return Err(format!(
                        "Line {}: not enough arguments for chroma trigger",
                        line_number
                    ));
                }
                let note_type = ChromaNoteType::from_str(line[0])
                    .map_err(|e| format!("Line {}: {}", line_number, e))?;
                let start_time: f32 = line[1]
                    .parse()
                    .map_err(|_| format!("Line {}: invalid trigger time", line_number))?;
                let end_time: f32 = line[2]
                    .parse()
                    .map_err(|_| format!("Line {}: invalid trigger time", line_number))?;
                let start_color = get_color(&default_colors, note_type, line[3])
                    .map_err(|e| format!("Line {}: {}", line_number, e))?;
                let end_color = get_color(&default_colors, note_type, line[4])
                    .map_err(|e| format!("Line {}: {}", line_number, e))?;
                chroma_data
                    .get_mut(&note_type)
                    .unwrap()
                    .push(ChromaTrigger {
                        time: start_time,
                        duration: end_time - start_time,
                        start_color,
                        end_color,
                    });
            }
        }
    }

    for (_, trigger_data) in chroma_data.iter_mut() {
        trigger_data.sort_by(|a, b| a.time.total_cmp(&b.time));
    }

    {
        use ChromaNoteType::*;
        return Ok(ChromaTriggersData {
            note_a: chroma_data.remove(&NoteA).unwrap(),
            note_b: chroma_data.remove(&NoteB).unwrap(),
            beat: chroma_data.remove(&Beat).unwrap(),
            spin_left: chroma_data.remove(&SpinLeft).unwrap(),
            spin_right: chroma_data.remove(&SpinRight).unwrap(),
            scratch: chroma_data.remove(&Scratch).unwrap(),
            ancillary: chroma_data.remove(&Ancillary).unwrap(),
        });
    }
}

pub fn integrate(srtb: &Path, speeds: &Path, diff_key: &str) -> Result<(), String> {
    println!("Reading file contents");
    let chart_contents = fs::read_to_string(srtb).map_err(|e| e.to_string())?;
    let speeds_contents = fs::read_to_string(speeds).map_err(|e| e.to_string())?;

    println!("Converting speeds");
    let speeds = speeds_to_json(&speeds_contents)?;
    let speeds_json = serde_json::to_string(&speeds).map_err(|e| e.to_string())?;

    println!("Integrating to srtb");
    let mut chart: RawSrtbFile =
        serde_json::from_str(&chart_contents).map_err(|e| e.to_string())?;
    if let Some(value) = chart
        .large_string_values_container
        .values
        .iter_mut()
        .find(|v| v.key == diff_key)
    {
        value.val.clone_from(&speeds_json);
    } else {
        chart
            .large_string_values_container
            .values
            .push(LargeStringValue {
                key: diff_key.to_string(),
                val: speeds_json.clone(),
            });
    }
    let chart = serde_json::to_string(&chart).map_err(|e| e.to_string())?;

    println!("Integration complete! Please select where you would like to save your file");
    let file = rfd::FileDialog::new()
        .add_filter("Spin Rhythm Track Bundle", &["srtb"])
        .save_file();
    let dest_file = file.ok_or("no destination file selected")?;
    fs::write(dest_file, chart).map_err(|e| e.to_string())?;
    println!("All done!");
    Ok(())
}

pub fn integrate_chroma(srtb: &Path, chroma: &Path, diff_key: &str) -> Result<(), String> {
    println!("Reading file contents");
    let chart_contents = fs::read_to_string(srtb).map_err(|e| e.to_string())?;
    let speeds_contents = fs::read_to_string(chroma).map_err(|e| e.to_string())?;

    println!("Converting speeds");
    let chroma = chroma_to_json(&speeds_contents)?;
    let chroma_json = serde_json::to_string(&chroma).map_err(|e| e.to_string())?;

    println!("Integrating to srtb");
    let mut chart: RawSrtbFile =
        serde_json::from_str(&chart_contents).map_err(|e| e.to_string())?;
    if let Some(value) = chart
        .large_string_values_container
        .values
        .iter_mut()
        .find(|v| v.key == diff_key)
    {
        value.val.clone_from(&chroma_json);
    } else {
        chart
            .large_string_values_container
            .values
            .push(LargeStringValue {
                key: diff_key.to_string(),
                val: chroma_json.clone(),
            });
    }
    let chart = serde_json::to_string(&chart).map_err(|e| e.to_string())?;

    println!("Integration complete! Please select where you would like to save your file");
    let file = rfd::FileDialog::new()
        .add_filter("Spin Rhythm Track Bundle", &["srtb"])
        .save_file();
    let dest_file = file.ok_or("no destination file selected")?;
    fs::write(dest_file, chart).map_err(|e| e.to_string())?;
    println!("All done!");
    Ok(())
}

pub fn extract(file: &Path, diff_key: &str) -> Result<(), String> {
    println!("Checking for speeds data");
    let srtb_contents = fs::read_to_string(file).map_err(|e| e.to_string())?;
    let chart: RawSrtbFile = serde_json::from_str(&srtb_contents).map_err(|e| e.to_string())?;

    if let Some(value) = chart
        .large_string_values_container
        .values
        .iter()
        .find(|v| v.key == diff_key)
    {
        println!("Found speeds data. Converting");
        let speeds: SpeedTriggersData =
            serde_json::from_str(&value.val).map_err(|e| e.to_string())?;
        let speeds = json_to_speeds(&speeds);

        println!(
            "Conversion done! Please select where you would like to save the resulting speeds file"
        );
        let file = rfd::FileDialog::new()
            .add_filter("Speed Triggers file", &["speeds"])
            .save_file();
        let file = file.ok_or("no destination file selected")?;
        let file = file.with_extension("speeds");
        fs::write(file, speeds).map_err(|e| e.to_string())?;
        println!("All done!");
    } else {
        println!("No speeds data found.");
    }
    Ok(())
}

pub fn remove(file: &Path, diff_key: &str) -> Result<(), String> {
    println!("Checking for speeds data");
    let srtb_contents = fs::read_to_string(file).map_err(|e| e.to_string())?;
    let mut chart: RawSrtbFile = serde_json::from_str(&srtb_contents).map_err(|e| e.to_string())?;

    if let Some((index, _)) = chart
        .large_string_values_container
        .values
        .iter()
        .enumerate()
        .find(|(_, v)| v.key == diff_key)
    {
        println!("Found speeds data. Removing");
        chart.large_string_values_container.values.remove(index);
        let chart_contents = serde_json::to_string(&chart).map_err(|e| e.to_string())?;
        println!("Removed! Please select a saving location");
        let file = rfd::FileDialog::new()
            .add_filter("Spin Rhythm Track Bundle", &["srtb"])
            .save_file();
        let file = file.ok_or("no destination file selected")?;
        let file = file.with_extension("srtb");
        fs::write(file, chart_contents).map_err(|e| e.to_string())?;
        println!("All done!");
    } else {
        println!("No speeds data found.");
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{json_to_speeds, speeds_to_json, SpeedTrigger, SpeedTriggersData};

    #[test]
    fn test_speeds_to_json() {
        let speeds = r#"
        0 1
        1.5  2    false
        2    1.5  true
        "#;

        let expected_speeds = vec![
            SpeedTrigger {
                time: 0.,
                speed_multiplier: 1.,
                interpolate_to_next_trigger: false,
            },
            SpeedTrigger {
                time: 1.5,
                speed_multiplier: 2.,
                interpolate_to_next_trigger: false,
            },
            SpeedTrigger {
                time: 2.,
                speed_multiplier: 1.5,
                interpolate_to_next_trigger: true,
            },
        ];

        let speeds = speeds_to_json(speeds).unwrap();
        assert_eq!(speeds.triggers, expected_speeds);
    }

    #[test]
    fn struct_to_speeds() {
        let triggers = vec![
            SpeedTrigger {
                time: 0.,
                speed_multiplier: 1.,
                interpolate_to_next_trigger: false,
            },
            SpeedTrigger {
                time: 1.5,
                speed_multiplier: 2.,
                interpolate_to_next_trigger: false,
            },
            SpeedTrigger {
                time: 2.,
                speed_multiplier: 1.5,
                interpolate_to_next_trigger: true,
            },
        ];
        let speeds = SpeedTriggersData { triggers };

        let expected_speeds = "0 1 false\n1.5 2 false\n2 1.5 true\n";

        let speeds = json_to_speeds(&speeds);
        assert_eq!(speeds, expected_speeds);
    }
}
