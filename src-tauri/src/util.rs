use std::{
    env,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn replace_or_remove(format: String, placeholder: &str, value: &str) -> String {
    if value.trim().is_empty() {
        format.replace(placeholder, "")
    } else {
        format.replace(placeholder, value)
    }
}

pub(crate) fn normalize_track_path(path: &str) -> String {
    path.replace('/', std::path::MAIN_SEPARATOR_STR)
}

pub(crate) fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub(crate) fn settings_path() -> PathBuf {
    data_dir("null's DJ Utilities").join("settings.json")
}

pub(crate) fn rekordbox_dirs() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Pioneer").join("rekordbox"))
            .into_iter()
            .collect();
    }

    #[cfg(target_os = "macos")]
    {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| {
                vec![
                    home.join("Library")
                        .join("Application Support")
                        .join("Pioneer")
                        .join("rekordbox"),
                    home.join("Library").join("Pioneer").join("rekordbox"),
                    home.join("Library")
                        .join("Application Support")
                        .join("Pioneer")
                        .join("rekordbox6"),
                    home.join("Library").join("Pioneer").join("rekordbox6"),
                ]
            })
            .unwrap_or_default();
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Vec::new()
    }
}

fn data_dir(app_name: &str) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = env::var_os("LOCALAPPDATA") {
            return PathBuf::from(path).join(app_name);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(app_name);
        }
    }

    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(app_name)
}
