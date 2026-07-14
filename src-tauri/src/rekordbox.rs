use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rusqlite::{Connection, OpenFlags, Row};
use std::{
    fs,
    path::Path,
    process::Command,
    sync::{Mutex, OnceLock},
};

use crate::{
    media, memory,
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
        if let Some(track) = media::get_current_track() {
            return media_probe(track);
        }
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
            if let Some(track) = media::get_current_track() {
                return media_probe(track);
            }
            return TrackProbe {
                track: None,
                status: format!("Could not open rekordbox DB: {err}"),
            };
        }
    };
    if let Err(err) = conn.pragma_update(None, "key", DB_PASSWORD) {
        if let Some(track) = media::get_current_track() {
            return media_probe(track);
        }
        return TrackProbe {
            track: None,
            status: format!("Could not unlock rekordbox DB: {err}"),
        };
    }

    if is_running {
        if let Ok(live_track) = memory::current_track() {
            if live_track.master_verified && !live_track.file_path.is_empty() {
                if let Ok(Some(mut track)) =
                    get_track_info_by_path(&conn, rekordbox_dir, &live_track.file_path)
                {
                    apply_live_track(&mut track, &live_track);
                    fill_missing_album_art(&mut track);
                    return TrackProbe {
                        track: Some(track),
                        status: String::new(),
                    };
                }
            }

            if live_track.master_verified && !live_track.title.is_empty() {
                let mut track = get_track_info_by_metadata(&conn, rekordbox_dir, &live_track)
                    .ok()
                    .unwrap_or_else(|| TrackInfo {
                        title: live_track.title.clone(),
                        artist: live_track.artist.clone(),
                        album: live_track.album.clone(),
                        file_path: String::new(),
                        album_art_data_url: None,
                        bpm: None,
                        position_seconds: None,
                        duration_seconds: None,
                        source: String::new(),
                    });
                apply_live_track(&mut track, &live_track);
                fill_missing_album_art(&mut track);
                return TrackProbe {
                    track: Some(track),
                    status: String::new(),
                };
            }
        }
    }

    if let Some(track) = media::get_current_track() {
        return media_probe(track);
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

    match get_track_info_by_content_id(&conn, rekordbox_dir, &content_id) {
        Ok(track) if has_title(&track) => TrackProbe {
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
        Ok(_) => {
            if let Some(track) = media::get_current_track() {
                return media_probe(track);
            }
            TrackProbe {
                track: None,
                status: "Current Rekordbox track has BPM but no title metadata.".to_string(),
            }
        }
        Err(err) => TrackProbe {
            track: None,
            status: format!("Could not read current rekordbox track: {err}"),
        },
    }
}

fn has_title(track: &TrackInfo) -> bool {
    !track.title.trim().is_empty()
}

fn media_probe(track: TrackInfo) -> TrackProbe {
    TrackProbe {
        status: format!("Source: {}", track.source),
        track: Some(track),
    }
}

fn apply_live_track(track: &mut TrackInfo, live: &memory::LiveTrack) {
    if track.title.trim().is_empty() {
        track.title = live.title.clone();
    }
    if track.artist.trim().is_empty() {
        track.artist = live.artist.clone();
    }
    if track.album.trim().is_empty() {
        track.album = live.album.clone();
    }
    if track.bpm.is_none() {
        track.bpm = live.bpm;
    }
    if track.position_seconds.is_none() {
        track.position_seconds = live.position_seconds;
    }
    if track.duration_seconds.is_none() {
        track.duration_seconds = live.duration_seconds;
    }
    if track.source.is_empty() {
        track.source = live.source.clone();
    }
}

fn fill_missing_album_art(track: &mut TrackInfo) {
    if track.album_art_data_url.is_some() {
        return;
    }

    if !track.file_path.trim().is_empty() {
        track.album_art_data_url = memory_album_art(track);
        if track.album_art_data_url.is_some() {
            return;
        }
    }

    if let Some(art) = online_album_art(track) {
        track.album_art_data_url = Some(art);
    }
}

fn memory_album_art(track: &TrackInfo) -> Option<String> {
    static CACHE: OnceLock<Mutex<Option<(String, Option<String>)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    let key = track.file_path.clone();
    let mut cached = cache.lock().expect("album art cache poisoned");
    if let Some((cached_key, cached_art)) = &*cached {
        if cached_key == &key {
            return cached_art.clone();
        }
    }

    let art = memory::current_album_art_data_url();
    *cached = Some((key, art));
    cached.as_ref().and_then(|(_, art)| art.clone())
}

fn online_album_art(track: &TrackInfo) -> Option<String> {
    static CACHE: OnceLock<Mutex<Option<(String, Option<String>)>>> = OnceLock::new();
    let key = format!("{}|{}|{}", track.title, track.artist, track.album);
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    let mut cached = cache.lock().expect("online art cache poisoned");
    if let Some((cached_key, cached_art)) = &*cached {
        if cached_key == &key {
            return cached_art.clone();
        }
    }

    let art = fetch_itunes_album_art(track);
    *cached = Some((key, art.clone()));
    art
}

fn fetch_itunes_album_art(track: &TrackInfo) -> Option<String> {
    let term = [&track.title, &track.artist, &track.album]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    if term.trim().is_empty() {
        return None;
    }

    let url = format!(
        "https://itunes.apple.com/search?media=music&entity=song&limit=5&term={}",
        percent_encode(&term)
    );
    let output = Command::new("curl")
        .args(["-LfsS", "--max-time", "3", &url])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    itunes_artwork_url(&output.stdout)
}

fn itunes_artwork_url(bytes: &[u8]) -> Option<String> {
    let json: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    json.get("results")?
        .as_array()?
        .iter()
        .find_map(|item| item.get("artworkUrl100")?.as_str())
        .map(|url| url.replace("100x100", "600x600"))
}

fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn get_track_info_by_metadata(
    conn: &Connection,
    rekordbox_dir: &Path,
    track: &memory::LiveTrack,
) -> rusqlite::Result<TrackInfo> {
    let mut result = conn.query_row(
        "SELECT c.Title, c.FolderPath, a.Name, al.Name, c.ImagePath
         FROM djmdContent c
         LEFT JOIN djmdArtist a ON c.ArtistID = a.ID
         LEFT JOIN djmdAlbum al ON c.AlbumID = al.ID
         WHERE c.Title = ?1
           AND (?2 = '' OR IFNULL(a.Name, '') = ?2)
           AND (?3 = '' OR IFNULL(al.Name, '') = ?3)
        LIMIT 1",
        [&track.title, &track.artist, &track.album],
        |row| row_to_track(row, rekordbox_dir),
    )?;
    enrich_track_by_path(conn, &mut result);
    Ok(result)
}

fn get_track_info_by_content_id(
    conn: &Connection,
    rekordbox_dir: &Path,
    content_id: &str,
) -> rusqlite::Result<TrackInfo> {
    let mut track = conn.query_row(
        "SELECT c.Title, c.FolderPath, a.Name, al.Name, c.ImagePath
         FROM djmdContent c
         LEFT JOIN djmdArtist a ON c.ArtistID = a.ID
         LEFT JOIN djmdAlbum al ON c.AlbumID = al.ID
         WHERE c.ID = ?1",
        [content_id],
        |row| row_to_track(row, rekordbox_dir),
    )?;
    enrich_track_by_content_id(conn, &mut track, content_id);
    enrich_track_from_history(conn, &mut track, content_id);
    Ok(track)
}

fn get_track_info_by_path(
    conn: &Connection,
    rekordbox_dir: &Path,
    memory_path: &str,
) -> rusqlite::Result<Option<TrackInfo>> {
    let slash_path = memory_path.replace('\\', "/");
    let backslash_path = memory_path.replace('/', "\\");
    for candidate in [memory_path, slash_path.as_str(), backslash_path.as_str()] {
        let result = conn.query_row(
            "SELECT c.Title, c.FolderPath, a.Name, al.Name, c.ImagePath
             FROM djmdContent c
             LEFT JOIN djmdArtist a ON c.ArtistID = a.ID
             LEFT JOIN djmdAlbum al ON c.AlbumID = al.ID
             WHERE c.FolderPath = ?1 OR replace(c.FolderPath, char(92), '/') = ?1
             LIMIT 1",
            [candidate],
            |row| row_to_track(row, rekordbox_dir),
        );
        match result {
            Ok(mut track) => {
                enrich_track_by_path(conn, &mut track);
                return Ok(Some(track));
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(err) => return Err(err),
        }
    }
    Ok(None)
}

fn row_to_track(row: &Row<'_>, rekordbox_dir: &Path) -> rusqlite::Result<TrackInfo> {
    let image_path: Option<String> = row.get(4)?;
    Ok(TrackInfo {
        title: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
        file_path: normalize_track_path(&row.get::<_, Option<String>>(1)?.unwrap_or_default()),
        artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        album: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        album_art_data_url: load_album_art_data_url(rekordbox_dir, image_path.as_deref()),
        bpm: None,
        position_seconds: None,
        duration_seconds: None,
        source: "Rekordbox".to_string(),
    })
}

fn enrich_track_by_content_id(conn: &Connection, track: &mut TrackInfo, content_id: &str) {
    if track.bpm.is_none() {
        track.bpm = content_number(conn, "ID = ?1", content_id, bpm_column(conn).as_deref())
            .and_then(normalize_bpm);
    }
    if track.duration_seconds.is_none() {
        track.duration_seconds = content_number(
            conn,
            "ID = ?1",
            content_id,
            duration_column(conn).as_deref(),
        )
        .and_then(normalize_duration);
    }
}

fn enrich_track_from_history(conn: &Connection, track: &mut TrackInfo, content_id: &str) {
    if track.title.trim().is_empty() {
        if let Some(value) = history_text(
            conn,
            content_id,
            &["Title", "TrackTitle", "SongTitle", "Name", "SongName"],
        ) {
            track.title = value;
        }
    }
    if track.artist.trim().is_empty() {
        if let Some(value) =
            history_text(conn, content_id, &["Artist", "ArtistName", "ArtistNames"])
        {
            track.artist = value;
        }
    }
    if track.album.trim().is_empty() {
        if let Some(value) = history_text(conn, content_id, &["Album", "AlbumName"]) {
            track.album = value;
        }
    }
}

fn history_text(conn: &Connection, content_id: &str, candidates: &[&str]) -> Option<String> {
    let column = table_column(conn, "djmdSongHistory", candidates)?;
    conn.query_row(
        &format!(
            "SELECT {column} FROM djmdSongHistory
             WHERE ContentID = ?1
             ORDER BY created_at DESC
             LIMIT 1"
        ),
        [content_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

fn enrich_track_by_path(conn: &Connection, track: &mut TrackInfo) {
    if track.file_path.is_empty() {
        return;
    }
    let slash_path = track.file_path.replace('\\', "/");
    let where_sql = "FolderPath = ?1 OR replace(FolderPath, char(92), '/') = ?1";
    if track.bpm.is_none() {
        track.bpm = content_number(conn, where_sql, &slash_path, bpm_column(conn).as_deref())
            .and_then(normalize_bpm);
    }
    if track.duration_seconds.is_none() {
        track.duration_seconds = content_number(
            conn,
            where_sql,
            &slash_path,
            duration_column(conn).as_deref(),
        )
        .and_then(normalize_duration);
    }
}

fn content_number(
    conn: &Connection,
    where_sql: &str,
    value: &str,
    column: Option<&str>,
) -> Option<f64> {
    let column = column?;
    conn.query_row(
        &format!("SELECT {column} FROM djmdContent WHERE {where_sql} LIMIT 1"),
        [value],
        |row| row.get::<_, Option<f64>>(0),
    )
    .ok()
    .flatten()
}

fn bpm_column(conn: &Connection) -> Option<String> {
    table_column(conn, "djmdContent", &["BPM", "AverageBpm", "AverageBPM"])
}

fn duration_column(conn: &Connection) -> Option<String> {
    table_column(conn, "djmdContent", &["Length", "Duration", "TrackLength"])
}

fn table_column(conn: &Connection, table: &str, candidates: &[&str]) -> Option<String> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})")).ok()?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .ok()?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    candidates
        .iter()
        .find_map(|candidate| {
            rows.iter()
                .find(|name| name.eq_ignore_ascii_case(candidate))
        })
        .cloned()
}

fn normalize_bpm(value: f64) -> Option<f32> {
    let bpm = if value > 1000.0 { value / 100.0 } else { value };
    (bpm.is_finite() && bpm > 0.0).then_some(bpm as f32)
}

fn normalize_duration(value: f64) -> Option<f64> {
    let seconds = if value > 86_400.0 {
        value / 1000.0
    } else {
        value
    };
    (seconds.is_finite() && seconds > 0.0).then_some(seconds)
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
        .args(["-x", "rekordbox"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn is_rekordbox_running() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encodes_search_terms() {
        assert_eq!(percent_encode("C & B feat. Such"), "C+%26+B+feat.+Such");
    }

    #[test]
    fn parses_itunes_artwork_url() {
        let json = br#"{"results":[{"artworkUrl100":"https://example.test/100x100bb.jpg"}]}"#;

        assert_eq!(
            itunes_artwork_url(json).as_deref(),
            Some("https://example.test/600x600bb.jpg")
        );
    }
}
