use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::IntegrationError;

#[derive(Debug, Serialize, Deserialize)]
struct ValuesContainer<T> {
    values: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnityObjectValue {
    key: String,
    json_key: String,
    full_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LargeStringValue {
    key: String,
    val: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSrtbFile {
    unity_object_values_container: ValuesContainer<UnityObjectValue>,
    large_string_values_container: ValuesContainer<LargeStringValue>,
    clip_info_count: Option<i32>,
}

impl RawSrtbFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, IntegrationError> {
        let file_contents = fs::read_to_string(path).map_err(IntegrationError::IoError)?;
        serde_json::from_str(&file_contents).map_err(IntegrationError::SerdeJsonError)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), IntegrationError> {
        let chart_string = serde_json::to_string(self).map_err(IntegrationError::SerdeJsonError)?;
        fs::write(path, chart_string).map_err(IntegrationError::IoError)
    }

    pub fn get_large_string_value(&self, key_string: &str) -> Option<String> {
        self.large_string_values_container
            .values
            .iter()
            .find(|v| v.key == key_string)
            .map(|v| v.val.clone())
    }

    pub fn set_large_string_value(&mut self, key_string: &str, value: &str) {
        if let Some(val) = self
            .large_string_values_container
            .values
            .iter_mut()
            .find(|v| v.key == key_string)
        {
            val.val = value.to_string();
        } else {
            self.large_string_values_container
                .values
                .push(LargeStringValue {
                    key: key_string.to_string(),
                    val: value.to_string(),
                });
        }
    }

    pub fn remove_large_string_value(&mut self, key_string: &str) {
        if let Some(i) = self
            .large_string_values_container
            .values
            .iter()
            .enumerate()
            .find(|(_, v)| v.key == key_string)
            .map(|(i, _)| i)
        {
            self.large_string_values_container.values.remove(i);
        }
    }
}
