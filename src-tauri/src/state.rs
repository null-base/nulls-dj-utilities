use std::sync::{Arc, Mutex};

use crate::models::{AppSettings, HistoryEntry, TrackInfo};

#[derive(Clone)]
pub(crate) struct SharedState(pub(crate) Arc<Mutex<Inner>>);

#[derive(Clone)]
pub(crate) struct Inner {
    pub(crate) is_syncing: bool,
    pub(crate) settings: AppSettings,
    pub(crate) current_track: Option<TrackInfo>,
    pub(crate) rekordbox_status: String,
    pub(crate) history: Vec<HistoryEntry>,
    pub(crate) last_sent_message: Option<String>,
}

impl Inner {
    pub(crate) fn new() -> Self {
        Self {
            is_syncing: false,
            settings: AppSettings::load(),
            current_track: None,
            rekordbox_status: "Waiting for rekordbox...".to_string(),
            history: Vec::new(),
            last_sent_message: None,
        }
    }
}
