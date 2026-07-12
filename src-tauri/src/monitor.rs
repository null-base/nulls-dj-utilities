use std::{thread, time::Duration};

use crate::{
    models::HistoryEntry,
    osc::{send_chatbox, send_track_info},
    rekordbox::get_current_track,
    state::SharedState,
    util::now_ms,
};

const MAX_HISTORY: usize = 200;

pub(crate) fn spawn_monitor(state: SharedState) {
    thread::spawn(move || {
        let mut resend_counter = 0_u64;

        loop {
            let interval = state
                .0
                .lock()
                .expect("state poisoned")
                .settings
                .polling_interval_ms
                .clamp(500, 30_000);
            let probe = get_current_track();
            let next_track = probe.track;

            {
                let mut inner = state.0.lock().expect("state poisoned");
                inner.rekordbox_status = probe.status;
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
                } else if inner.is_syncing && inner.current_track.is_some() {
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

            thread::sleep(Duration::from_millis(interval));
        }
    });
}
