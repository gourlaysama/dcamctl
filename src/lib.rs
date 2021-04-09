use std::str::FromStr;

use anyhow::*;

pub mod cli;

#[derive(Debug)]
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
        let height: u16 = if let Some(left) = parts.next() {
            left.parse()?
        } else {
            bail!("No height found")
        };

        Ok(Self { height, width })
    }
}
