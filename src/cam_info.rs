use serde_aux::prelude::*;

use crate::config::Resolution;

#[derive(Debug, serde::Deserialize)]
pub struct CamInfo {
    pub curvals: CurrentValues,
    pub avail: Option<Available>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CurrentValues {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub zoom: u16,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub crop_x: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub crop_y: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub quality: u16,
    #[serde(with = "resolution")]
    pub video_size: Resolution,
}

#[derive(Debug, serde::Deserialize)]
pub struct Available {
    pub zoom: Vec<String>,
}

mod resolution {
    use super::Resolution;
    use serde::{de, Deserialize, Deserializer};
    use std::str::FromStr;

    pub fn deserialize<'de, D>(d: D) -> Result<Resolution, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;

        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}
