use std::{net::UdpSocket, thread, time::Duration};

use crate::models::{AppSettings, TrackInfo};

pub(crate) fn send_track_info(
    track: &TrackInfo,
    settings: &AppSettings,
    last_sent: &mut Option<String>,
) {
    let message = settings.format_message(track);
    if last_sent.as_deref() == Some(&message) {
        return;
    }

    if settings.osc_delay_ms == 0 {
        if send_chatbox(
            &settings.osc_host,
            settings.osc_port,
            &message,
            true,
            settings.send_notification_sound,
        )
        .is_err()
        {
            return;
        }
        *last_sent = Some(message);
        return;
    }

    let host = settings.osc_host.clone();
    let port = settings.osc_port;
    let notify = settings.send_notification_sound;
    let delay = settings.osc_delay_ms;
    *last_sent = Some(message.clone());
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(delay));
        let _ = send_chatbox(&host, port, &message, true, notify);
    });
}

pub(crate) fn send_chatbox(
    host: &str,
    port: u16,
    text: &str,
    bypass: bool,
    notify: bool,
) -> std::io::Result<()> {
    let mut data = Vec::new();
    write_osc_string(&mut data, "/chatbox/input");
    write_osc_string(
        &mut data,
        &format!(
            ",s{}{}",
            if bypass { "T" } else { "F" },
            if notify { "T" } else { "F" }
        ),
    );
    write_osc_string(&mut data, text);

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.send_to(&data, (host, port))?;
    Ok(())
}

fn write_osc_string(data: &mut Vec<u8>, value: &str) {
    data.extend_from_slice(value.as_bytes());
    data.push(0);
    while data.len() % 4 != 0 {
        data.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc_strings_are_four_byte_padded() {
        let mut data = Vec::new();
        write_osc_string(&mut data, "/chatbox/input");
        write_osc_string(&mut data, ",sTF");
        write_osc_string(&mut data, "abc");

        assert_eq!(data.len() % 4, 0);
        assert!(data.starts_with(b"/chatbox/input\0\0"));
    }
}
