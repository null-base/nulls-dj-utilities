use std::process::Command;

use crate::models::TrackInfo;

pub(crate) fn get_current_track() -> Option<TrackInfo> {
    platform::get_current_track()
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    const SEP: &str = "\u{1f}";

    pub(crate) fn get_current_track() -> Option<TrackInfo> {
        spotify_track().or_else(music_track)
    }

    fn spotify_track() -> Option<TrackInfo> {
        let script = r#"
if application "Spotify" is running then
  tell application "Spotify"
    if player state is playing then
      set t to current track
      set art to ""
      try
        set art to artwork url of t
      end try
      set AppleScript's text item delimiters to (ASCII character 31)
      return {name of t, artist of t, album of t, (player position as text), ((duration of t / 1000) as text), art} as text
    end if
  end tell
end if
"#;
        parse_track(&run_osascript(script)?, "Spotify", true)
    }

    fn music_track() -> Option<TrackInfo> {
        let script = r#"
if application "Music" is running then
  tell application "Music"
    if player state is playing then
      set t to current track
      set AppleScript's text item delimiters to (ASCII character 31)
      return {name of t, artist of t, album of t, (player position as text), (duration of t as text), ""} as text
    end if
  end tell
end if
"#;
        parse_track(&run_osascript(script)?, "Music", false)
    }

    fn run_osascript(script: &str) -> Option<String> {
        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn parse_track(text: &str, source: &str, allow_art_url: bool) -> Option<TrackInfo> {
        let mut parts = text.split(SEP);
        let title = parts.next()?.trim().to_string();
        if title.is_empty() {
            return None;
        }

        Some(TrackInfo {
            title,
            artist: parts.next().unwrap_or("").trim().to_string(),
            album: parts.next().unwrap_or("").trim().to_string(),
            file_path: String::new(),
            position_seconds: parse_seconds(parts.next()),
            duration_seconds: parse_seconds(parts.next()),
            album_art_data_url: allow_art_url
                .then(|| parts.next().unwrap_or("").trim().to_string())
                .filter(|value| value.starts_with("http")),
            bpm: None,
            source: source.to_string(),
        })
    }

    fn parse_seconds(value: Option<&str>) -> Option<f64> {
        value?
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| *value >= 0.0)
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    pub(crate) fn get_current_track() -> Option<TrackInfo> {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "$p=Get-Process Spotify -ErrorAction SilentlyContinue | Where-Object {$_.MainWindowTitle -and $_.MainWindowTitle -notmatch '^Spotify|Advertisement'} | Select-Object -First 1; if ($p) {$p.MainWindowTitle}",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let (artist, title) = title
            .split_once(" - ")
            .map(|(artist, title)| (artist.trim().to_string(), title.trim().to_string()))
            .unwrap_or_else(|| (String::new(), title));
        (!title.is_empty()).then_some(TrackInfo {
            title,
            artist,
            album: String::new(),
            file_path: String::new(),
            album_art_data_url: None,
            bpm: None,
            position_seconds: None,
            duration_seconds: None,
            source: "Spotify".to_string(),
        })
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod platform {
    use super::*;

    pub(crate) fn get_current_track() -> Option<TrackInfo> {
        None
    }
}
