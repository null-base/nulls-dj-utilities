use std::{thread, time::Duration};

use crate::{
    models::{HistoryEntry, TrackInfo},
    osc::{send_chatbox, send_track_info},
    rekordbox::get_current_track,
    state::SharedState,
    util::now_ms,
};

const MAX_HISTORY: usize = 200;

pub(crate) fn spawn_monitor(state: SharedState) {
    thread::spawn(move || {
        let mut resend_counter = 0_u64;
        let mut last_seen_rekordbox_track: Option<TrackInfo> = None;

        loop {
            let interval = state
                .0
                .lock()
                .expect("state poisoned")
                .settings
                .polling_interval_ms
                .clamp(500, 30_000);
            let probe = get_current_track();
            let probe_track = probe.track;
            let next_track = playable_track(probe_track.clone(), &mut last_seen_rekordbox_track);

            {
                let mut inner = state.0.lock().expect("state poisoned");
                inner.rekordbox_status = probe.status;
                if should_keep_current_track(&probe_track, &next_track, &inner.current_track) {
                    resend_counter = 0;
                } else {
                    let track_changed = match (&next_track, &inner.current_track) {
                        (Some(next), Some(current)) => !next.same_track(current),
                        (None, None) => false,
                        _ => true,
                    };

                    if track_changed {
                        inner.current_track = next_track.clone();
                        resend_counter = 0;

                        if let Some(track) = next_track {
                            inner.history.insert(
                                0,
                                HistoryEntry {
                                    title: track.title.clone(),
                                    artist: track.artist.clone(),
                                    album: track.album.clone(),
                                    played_at_ms: now_ms(),
                                },
                            );
                            inner.history.truncate(MAX_HISTORY);

                            if inner.is_syncing {
                                let settings = inner.settings.clone();
                                send_track_info(&track, &settings, &mut inner.last_sent_message);
                            }
                        }
                    } else if next_track.is_some() {
                        inner.current_track = next_track.clone();
                    }

                    if !track_changed && inner.is_syncing && inner.current_track.is_some() {
                        resend_counter += 1;
                        let resend_interval = (10_000 / interval).max(1);
                        if resend_counter >= resend_interval {
                            let settings = inner.settings.clone();
                            if let Some(message) = inner.last_sent_message.clone() {
                                let _ = send_chatbox(
                                    &settings.osc_host,
                                    settings.osc_port,
                                    &message,
                                    true,
                                    false,
                                );
                            }
                            resend_counter = 0;
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(interval));
        }
    });
}

fn playable_track(
    track: Option<TrackInfo>,
    last_seen_rekordbox: &mut Option<TrackInfo>,
) -> Option<TrackInfo> {
    let track = track?;
    if track.source != "Rekordbox" {
        *last_seen_rekordbox = None;
        return Some(track);
    }
    if track.position_seconds.is_none() {
        *last_seen_rekordbox = Some(track.clone());
        return Some(track);
    }

    let is_playing = last_seen_rekordbox
        .as_ref()
        .and_then(|previous| {
            Some((
                previous.same_track(&track),
                previous.position_seconds?,
                track.position_seconds?,
            ))
        })
        .is_some_and(|(same_track, previous, current)| same_track && current > previous + 0.15);

    *last_seen_rekordbox = Some(track.clone());
    is_playing.then_some(track)
}

fn should_keep_current_track(
    probe_track: &Option<TrackInfo>,
    next_track: &Option<TrackInfo>,
    current_track: &Option<TrackInfo>,
) -> bool {
    next_track.is_none()
        && current_track.is_some()
        && probe_track
            .as_ref()
            .is_some_and(|track| track.source == "Rekordbox" && track.position_seconds.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rekordbox_track_waits_until_position_moves() {
        let mut last_seen = None;
        assert!(playable_track(Some(track("Rekordbox", Some(10.0))), &mut last_seen).is_none());
        assert!(playable_track(Some(track("Rekordbox", Some(10.0))), &mut last_seen).is_none());
        assert!(playable_track(Some(track("Rekordbox", Some(10.3))), &mut last_seen).is_some());
    }

    #[test]
    fn media_track_shows_immediately() {
        let mut last_seen = None;
        assert!(playable_track(Some(track("Spotify", Some(10.0))), &mut last_seen).is_some());
    }

    #[test]
    fn rekordbox_track_without_position_falls_back_to_visible() {
        let mut last_seen = None;
        assert!(playable_track(Some(track("Rekordbox", None)), &mut last_seen).is_some());
    }

    #[test]
    fn pending_rekordbox_track_keeps_current_visible() {
        assert!(should_keep_current_track(
            &Some(track("Rekordbox", Some(0.0))),
            &None,
            &Some(track("Rekordbox", Some(20.0)))
        ));
    }

    #[test]
    fn pending_track_does_not_keep_empty_or_media_display() {
        assert!(!should_keep_current_track(
            &Some(track("Rekordbox", Some(0.0))),
            &None,
            &None
        ));
        assert!(!should_keep_current_track(
            &Some(track("Spotify", Some(0.0))),
            &None,
            &Some(track("Rekordbox", Some(20.0)))
        ));
    }

    fn track(source: &str, position_seconds: Option<f64>) -> TrackInfo {
        TrackInfo {
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            album: String::new(),
            file_path: String::new(),
            album_art_data_url: None,
            bpm: None,
            position_seconds,
            duration_seconds: None,
            source: source.to_string(),
        }
    }
}
