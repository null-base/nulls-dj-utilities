use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rusqlite::{Connection, OpenFlags};
use std::{fs, path::Path, process::Command};

use crate::{
    models::TrackInfo,
    util::{normalize_track_path, rekordbox_dirs},
};

const DB_PASSWORD: &str = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";

pub(crate) struct TrackProbe {
    pub(crate) track: Option<TrackInfo>,
    pub(crate) status: String,
}

pub(crate) fn get_current_track() -> TrackProbe {
    let is_running = is_rekordbox_running();
    let rekordbox_dirs = rekordbox_dirs();
    let Some((rekordbox_dir, db_path)) = rekordbox_dirs
        .iter()
        .map(|dir| (dir, dir.join("master.db")))
        .find(|(_, db_path)| db_path.exists())
    else {
        let checked = rekordbox_dirs
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return TrackProbe {
            track: None,
            status: if checked.is_empty() {
                "Rekordbox folder was not found.".to_string()
            } else {
                format!("master.db was not found. Checked: {checked}")
            },
        };
    };

    let conn = match Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => conn,
        Err(err) => {
            return TrackProbe {
                track: None,
                status: format!("Could not open rekordbox DB: {err}"),
            };
        }
    };
    if let Err(err) = conn.pragma_update(None, "key", DB_PASSWORD) {
        return TrackProbe {
            track: None,
            status: format!("Could not unlock rekordbox DB: {err}"),
        };
    }

    let content_id: String = match conn.query_row(
        "SELECT ContentID FROM djmdSongHistory ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    ) {
        Ok(content_id) => content_id,
        Err(err) => {
            return TrackProbe {
                track: None,
                status: format!("No rekordbox play history found: {err}"),
            };
        }
    };

    let mut stmt = match conn.prepare(
        "SELECT c.Title, c.FolderPath, a.Name, al.Name, c.ImagePath
         FROM djmdContent c
         LEFT JOIN djmdArtist a ON c.ArtistID = a.ID
         LEFT JOIN djmdAlbum al ON c.AlbumID = al.ID
         WHERE c.ID = ?1",
    ) {
        Ok(stmt) => stmt,
        Err(err) => {
            return TrackProbe {
                track: None,
                status: format!("Could not read rekordbox track table: {err}"),
            };
        }
    };

    match stmt.query_row([content_id], |row| {
        let image_path: Option<String> = row.get(4)?;
        Ok(TrackInfo {
            title: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
            file_path: normalize_track_path(&row.get::<_, Option<String>>(1)?.unwrap_or_default()),
            artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            album: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            album_art_data_url: load_album_art_data_url(rekordbox_dir, image_path.as_deref()),
        })
    }) {
        Ok(track) => TrackProbe {
            track: Some(track),
            status: if is_running {
                String::new()
            } else {
                format!(
                    "Rekordbox process was not detected; using latest DB history: {}",
                    db_path.display()
                )
            },
        },
        Err(err) => TrackProbe {
            track: None,
            status: format!("Could not read current rekordbox track: {err}"),
        },
    }
}

fn load_album_art_data_url(rekordbox_dir: &Path, image_path: Option<&str>) -> Option<String> {
    let image_path = image_path?;
    if image_path.is_empty() {
        return None;
    }

    let image_path = Path::new(image_path);
    let bytes = fs::read(image_path)
        .or_else(|_| {
            fs::read(
                rekordbox_dir
                    .join("share")
                    .join(image_path.strip_prefix("/").unwrap_or(image_path)),
            )
        })
        .ok()?;
    let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(b"\xff\xd8") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    };

    Some(format!("data:{mime};base64,{}", BASE64.encode(bytes)))
}

#[cfg(target_os = "windows")]
fn is_rekordbox_running() -> bool {
    Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq rekordbox.exe", "/NH"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("rekordbox.exe"))
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn is_rekordbox_running() -> bool {
    Command::new("pgrep")
        .args(["-if", "rekordbox"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn is_rekordbox_running() -> bool {
    false
}
