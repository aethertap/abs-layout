use serde::Deserialize;
use std::path::{Path,PathBuf};
use anyhow::Result;

pub fn split_genre_string<'de,D>(deserializer:D) -> Result<Option<Vec<String>>,D::Error> 
where D: serde::de::Deserializer<'de> {
    let st:&str = serde::de::Deserialize::deserialize(deserializer)?;
    Ok(Some(st.split(':').map(|s| s.trim().to_owned()).collect()))
}

#[derive(Deserialize,Debug)]
pub struct BookRecord {
    #[serde(default)]
    pub destination: Option<PathBuf>,
    #[serde(rename="filename")]
    pub location: PathBuf,

    #[serde(default)] pub title: Option<String>,
    #[serde(default,deserialize_with="self::split_genre_string")] pub genre: Option<Vec<String>>,
    #[serde(default,rename="series_name")] pub series: Option<String>,
    #[serde(default)] pub author: Option<String>,
}

impl BookRecord {
    pub fn resolve_extension(&self, dir: &Path) -> Result<&'static str> {
        let extensions = ["m4b","mp3","aax"];
        for ext in extensions {
            if std::fs::exists(dir.join(self.location.with_added_extension(ext)))? {
                return Ok(ext)
            }
        }
        Err(anyhow::anyhow!("No extension found for book: {:?}. Dir was {:?}",self.title, dir))
    }    

    pub fn target_dir(&self) -> Result<PathBuf> {
        Ok(self.destination.as_ref()
            .and_then(|dest| dest.parent().map(|p| p.to_owned()))
            .ok_or_else(|| anyhow::anyhow!("BookRecord has no parent directory in its target path"))?)
    }
}


