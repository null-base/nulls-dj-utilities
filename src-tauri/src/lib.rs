mod models;
mod monitor;
mod obs;
mod osc;
mod rekordbox;
mod state;
mod util;

use models::AppSettings;
use osc::{send_chatbox, send_track_info};
use serde::Serialize;
use state::{Inner, SharedState};
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    is_syncing: bool,
    status_text: String,
    rekordbox_status: String,
    current_track: Option<models::TrackInfo>,
    history: Vec<models::HistoryEntry>,
    settings: AppSettings,
}

impl Inner {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            is_syncing: self.is_syncing,
            status_text: if self.is_syncing {
                format!(
                    "Sync Active ({}:{})",
                    self.settings.osc_host, self.settings.osc_port
                )
            } else {
                "OSC Disconnected".to_string()
            },
            rekordbox_status: self.rekordbox_status.clone(),
            current_track: self.current_track.clone(),
            history: self.history.clone(),
            settings: self.settings.clone(),
        }
    }
}

#[tauri::command]
fn get_snapshot(state: State<'_, SharedState>) -> Snapshot {
    state.0.lock().expect("state poisoned").snapshot()
}

#[tauri::command]
fn start_sync(state: State<'_, SharedState>) -> Snapshot {
    let mut inner = state.0.lock().expect("state poisoned");
    inner.settings = AppSettings::load().normalized();
    inner.is_syncing = true;

    if let Some(track) = inner.current_track.clone() {
        let settings = inner.settings.clone();
        send_track_info(&track, &settings, &mut inner.last_sent_message);
    }

    inner.snapshot()
}

#[tauri::command]
fn stop_sync(state: State<'_, SharedState>) -> Snapshot {
    let mut inner = state.0.lock().expect("state poisoned");
    let settings = inner.settings.clone();
    let _ = send_chatbox(&settings.osc_host, settings.osc_port, "", true, false);
    inner.last_sent_message = None;
    inner.is_syncing = false;
    inner.snapshot()
}

#[tauri::command]
fn save_settings(settings: AppSettings, state: State<'_, SharedState>) -> Result<Snapshot, String> {
    let settings = settings.normalized();
    settings.save()?;

    let mut inner = state.0.lock().expect("state poisoned");
    inner.settings = settings;
    Ok(inner.snapshot())
}

#[tauri::command]
fn clear_history(state: State<'_, SharedState>) -> Snapshot {
    let mut inner = state.0.lock().expect("state poisoned");
    inner.history.clear();
    inner.snapshot()
}

pub fn run() {
    let state = SharedState(Arc::new(Mutex::new(Inner::new())));
    monitor::spawn_monitor(state.clone());
    obs::spawn_obs_server(state.clone());

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            start_sync,
            stop_sync,
            save_settings,
            clear_history
        ])
        .run(tauri::generate_context!())
        .expect("failed to run null's DJ Utilities");
}
