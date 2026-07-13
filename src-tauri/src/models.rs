use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf};

use crate::util::{replace_or_remove, settings_path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSettings {
    pub(crate) osc_host: String,
    pub(crate) osc_port: u16,
    pub(crate) osc_delay_ms: u64,
    pub(crate) polling_interval_ms: u64,
    pub(crate) message_format: String,
    pub(crate) send_notification_sound: bool,
    pub(crate) obs_port: u16,
    pub(crate) obs_show_qr: bool,
    pub(crate) obs_qr_urls: BTreeMap<String, String>,
    pub(crate) obs_html: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            osc_host: "127.0.0.1".to_string(),
            osc_port: 9000,
            osc_delay_ms: 0,
            polling_interval_ms: 3000,
            message_format: "♫ {title} - {artist}".to_string(),
            send_notification_sound: false,
            obs_port: 17777,
            obs_show_qr: true,
            obs_qr_urls: BTreeMap::new(),
            obs_html: String::new(),
        }
    }
}

impl AppSettings {
    pub(crate) fn load() -> Self {
        let Ok(json) = fs::read_to_string(settings_path()) else {
            return Self::default();
        };
        serde_json::from_str(&json).unwrap_or_default()
    }

    pub(crate) fn save(&self) -> Result<(), String> {
        let path: PathBuf = settings_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|err| err.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|err| err.to_string())?;
        fs::write(path, json).map_err(|err| err.to_string())
    }

    pub(crate) fn normalized(mut self) -> Self {
        self.osc_host = self.osc_host.trim().to_string();
        if self.osc_host.is_empty() {
            self.osc_host = "127.0.0.1".to_string();
        }
        self.osc_delay_ms = self.osc_delay_ms.clamp(0, 30_000);
        self.polling_interval_ms = self.polling_interval_ms.clamp(500, 30_000);
        if self.message_format.trim().is_empty() {
            self.message_format = AppSettings::default().message_format;
        }
        self
    }

    pub(crate) fn format_message(&self, track: &TrackInfo) -> String {
        let mut result = self.message_format.clone();
        result = replace_or_remove(result, "{title}", &track.title);
        result = replace_or_remove(result, "{artist}", &track.artist);
        result = replace_or_remove(result, "{album}", &track.album);

        while result.contains(" -  - ") {
            result = result.replace(" -  - ", " - ");
        }

        result.trim_matches([' ', '-']).trim().to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrackInfo {
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) album: String,
    pub(crate) file_path: String,
    pub(crate) album_art_data_url: Option<String>,
}

impl TrackInfo {
    pub(crate) fn same_track(&self, other: &TrackInfo) -> bool {
        self.title == other.title
            && self.artist == other.artist
            && self.file_path == other.file_path
    }

    pub(crate) fn key(&self) -> String {
        if self.file_path.trim().is_empty() {
            format!("{}|{}|{}", self.title, self.artist, self.album)
        } else {
            self.file_path.clone()
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryEntry {
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) album: String,
    pub(crate) played_at_ms: u128,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObsPayload {
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) album: String,
    pub(crate) album_art_data_url: Option<String>,
    pub(crate) qr_code_data_url: Option<String>,
    pub(crate) message: String,
    pub(crate) is_playing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_message_without_empty_artist() {
        let settings = AppSettings {
            message_format: "♫ {title} - {artist}".to_string(),
            ..Default::default()
        };
        let track = TrackInfo {
            title: "Song".to_string(),
            artist: String::new(),
            album: String::new(),
            file_path: String::new(),
            album_art_data_url: None,
        };

        assert_eq!(settings.format_message(&track), "♫ Song");
    }

    #[test]
    fn normalizes_osc_delay() {
        let settings = AppSettings {
            osc_delay_ms: 99_999,
            ..Default::default()
        }
        .normalized();

        assert_eq!(settings.osc_delay_ms, 30_000);
    }
}
