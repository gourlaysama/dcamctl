use serde_aux::prelude::*;

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
}

#[derive(Debug, serde::Deserialize)]
pub struct Available {
    pub zoom: Vec<String>,
}
