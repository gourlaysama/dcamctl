use std::{path::PathBuf, str::FromStr};

use anyhow::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProgramConfig {
    pub port: u16,
    pub device: PathBuf,
    #[serde(with = "resolution")]
    pub resolution: Option<Resolution>,
    pub no_audio: bool,
    pub no_echo_cancel: bool,
    pub flip: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Resolution {
    pub height: u16,
    pub width: u16,
}

impl FromStr for Resolution {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('x');
        let width: u16 = if let Some(left) = parts.next() {
            left.parse()?
        } else {
            bail!("No width found")
        };
        let height: u16 = if let Some(right) = parts.next() {
            right.parse()?
        } else {
            bail!("No height found")
        };

        Ok(Self { height, width })
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)?;

        Ok(())
    }
}

mod resolution {
    use super::Resolution;
    use serde::{de, Deserialize, Deserializer};
    use std::str::FromStr;

    pub fn deserialize<'de, D>(d: D) -> Result<Option<Resolution>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;

        if s == "auto" {
            Ok(None)
        } else {
            FromStr::from_str(&s).map(Some).map_err(de::Error::custom)
        }
    }
}
