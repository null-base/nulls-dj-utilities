use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use qrcode::{render::svg, EcLevel, QrCode};

use crate::{models::ObsPayload, state::SharedState};

const QR_IMAGE_SIZE: u32 = 320;

pub(crate) fn spawn_obs_server(state: SharedState) {
    thread::spawn(move || {
        let port = state.0.lock().expect("state poisoned").settings.obs_port;
        let Ok(listener) = TcpListener::bind(("0.0.0.0", port)) else {
            return;
        };

        for stream in listener.incoming().flatten() {
            let state = state.clone();
            thread::spawn(move || handle_obs_request(stream, state));
        }
    });
}

fn handle_obs_request(mut stream: TcpStream, state: SharedState) {
    let mut buffer = [0_u8; 1024];
    let Ok(read) = stream.read(&mut buffer) else {
        return;
    };
    let request = String::from_utf8_lossy(&buffer[..read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    if path == "/track" {
        let payload = obs_payload(&state);
        let body = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
        write_http_response(&mut stream, "application/json; charset=utf-8", &body);
    } else {
        let html = obs_html(&state);
        write_http_response(&mut stream, "text/html; charset=utf-8", &html);
    }
}

fn obs_html(state: &SharedState) -> String {
    let inner = state.0.lock().expect("state poisoned");
    if inner.settings.obs_html.trim().is_empty() {
        OBS_HTML.to_string()
    } else {
        inner.settings.obs_html.clone()
    }
}

fn obs_payload(state: &SharedState) -> ObsPayload {
    let inner = state.0.lock().expect("state poisoned");
    if let Some(track) = &inner.current_track {
        ObsPayload {
            theme: inner.settings.obs_theme.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            album_art_data_url: track.album_art_data_url.clone(),
            bpm: track.bpm,
            position_seconds: track.position_seconds,
            duration_seconds: track.duration_seconds,
            source: track.source.clone(),
            qr_code_data_url: inner
                .settings
                .obs_show_qr
                .then(|| inner.settings.obs_qr_urls.get(&track.key()))
                .flatten()
                .map(String::as_str)
                .and_then(qr_code_data_url),
            message: inner.settings.format_message(track),
            is_playing: true,
        }
    } else {
        ObsPayload {
            theme: inner.settings.obs_theme.clone(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            album_art_data_url: None,
            bpm: None,
            position_seconds: None,
            duration_seconds: None,
            source: String::new(),
            qr_code_data_url: None,
            message: String::new(),
            is_playing: false,
        }
    }
}

fn qr_code_data_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let code = QrCode::with_error_correction_level(url, EcLevel::H).ok()?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(QR_IMAGE_SIZE, QR_IMAGE_SIZE)
        .dark_color(svg::Color("#000"))
        .light_color(svg::Color("#fff"))
        .build();
    Some(format!("data:image/svg+xml;base64,{}", BASE64.encode(svg)))
}

fn write_http_response(stream: &mut TcpStream, content_type: &str, body: &str) {
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(headers.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}

const OBS_HTML: &str = r##"<!doctype html>
<html lang="ja">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>null's DJ Utilities OBS</title>
<style>
  html, body {
    margin: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: transparent;
    font-family: system-ui, -apple-system, BlinkMacSystemFont, "Hiragino Sans", "Yu Gothic UI", Meiryo, sans-serif;
  }
  html[data-theme="minimal"],
  html[data-theme="minimal"] body { background: #000; }
  #wrap {
    display: flex;
    align-items: center;
    gap: 28px;
    box-sizing: border-box;
    width: 100vw;
    max-width: 100vw;
    padding: 20px 28px 20px 20px;
    border-radius: 18px;
    background: linear-gradient(135deg, rgba(9, 11, 13, 0.86), rgba(25, 29, 31, 0.72));
    box-shadow: 0 18px 52px rgba(0,0,0,.38);
    color: white;
    text-shadow: 0 2px 6px rgba(0,0,0,.45);
  }
  #wrap[data-theme="minimal"] {
    position: relative;
    gap: 0;
    min-height: 100vh;
    padding: 0;
    border-radius: 18px;
    background: transparent;
    box-shadow: none;
    overflow: hidden;
    text-shadow: 0 1px 3px rgba(0,0,0,.65);
  }
  #wrap.loaded {
    animation: pageLoad .82s cubic-bezier(.18,.8,.18,1) both;
  }
  #recordSlot {
    position: relative;
    flex: 0 0 auto;
    width: 188px;
    height: 188px;
  }
  #record,
  .record-ghost {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: 50%;
  }
  .record-disc {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      radial-gradient(circle at center, transparent 0 33%, rgba(255,255,255,.12) 34% 35%, transparent 36%),
      repeating-radial-gradient(circle at center, #111 0 7px, #1a1b1c 8px 10px),
      radial-gradient(circle at 34% 30%, rgba(255,255,255,.18), transparent 28%),
      #111;
    box-shadow: inset 0 0 0 2px rgba(255,255,255,.08), inset 0 0 34px rgba(0,0,0,.9), 0 12px 32px rgba(0,0,0,.45);
    animation: spin 5s linear infinite;
  }
  .record-disc::after {
    content: "";
    position: absolute;
    inset: 50%;
    width: 16px;
    height: 16px;
    transform: translate(-50%, -50%);
    border-radius: 50%;
    background: #090909;
    box-shadow: 0 0 0 5px rgba(255,255,255,.18);
  }
  #art, #artFallback, .record-art, .record-fallback {
    position: absolute;
    inset: 47px;
    width: 94px;
    height: 94px;
    border-radius: 50%;
  }
  #art, .record-art {
    object-fit: cover;
  }
  #artFallback, .record-fallback {
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, #83d7a4, #e7c264);
    color: #121416;
    font-size: 44px;
    font-weight: 900;
  }
  #copy {
    flex: 1 1 auto;
    min-width: 0;
  }
  #label {
    color: #83d7a4;
    font-size: 18px;
    font-weight: 700;
    letter-spacing: .08em;
    text-transform: uppercase;
  }
  #title {
    display: flex;
    flex-wrap: wrap;
    margin-top: 6px;
    font-size: 46px;
    font-weight: 800;
    line-height: 1.14;
    overflow-wrap: anywhere;
    overflow: hidden;
  }
  .title-char {
    display: inline-block;
  }
  .title-char-in {
    animation: titleCharIn .14s cubic-bezier(.18,.8,.18,1) both;
  }
  .title-char-out {
    transform-origin: right center;
    animation: titleCharOut .05s ease both;
  }
  #artist {
    margin-top: 10px;
    color: rgba(255,255,255,.88);
    font-size: 28px;
    font-weight: 650;
    overflow-wrap: anywhere;
  }
  #meta {
    margin-top: 6px;
    color: rgba(255,255,255,.66);
    font-size: 22px;
    overflow-wrap: anywhere;
  }
  #qrSlot {
    flex: 0 0 auto;
    width: 156px;
    height: 156px;
    padding: 12px;
    border-radius: 10px;
    background: #fff;
    box-shadow: 0 12px 28px rgba(0,0,0,.28);
  }
  #qr {
    display: block;
    width: 100%;
    height: 100%;
    image-rendering: pixelated;
  }
  #minimal {
    display: none;
    position: absolute;
    inset: 0;
    align-items: center;
    justify-content: center;
    width: 100%;
    min-height: 100vh;
    box-sizing: border-box;
  }
  #wrap[data-theme="minimal"] #minimal { display: flex; }
  #wrap[data-theme="minimal"] #recordSlot,
  #wrap[data-theme="minimal"] #copy,
  #wrap[data-theme="minimal"] #qrSlot { display: none; }
  #minimalCopy {
    position: absolute;
    top: 50%;
    left: 24px;
    width: calc(100% - 108px);
    min-width: 0;
    transform: translateY(-50%);
    text-align: left;
  }
  #wrap.has-qr #minimalCopy { left: 116px; width: calc(100% - 200px); }
  #miniClock {
    position: absolute;
    top: 50%;
    right: 24px;
    display: flex;
    align-items: baseline;
    transform: translateY(-50%);
  }
  #miniTime {
    width: auto;
    color: rgba(255,255,255,.96);
    font-size: 22px;
    font-variant-numeric: tabular-nums;
    font-weight: 500;
    line-height: 1;
  }
  #miniTitle {
    max-width: 100%;
    overflow: hidden;
    font-size: 26px;
    font-weight: 500;
    line-height: 1.3;
    text-align: left;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  #miniRight {
    position: absolute;
    top: 50%;
    left: 24px;
    display: flex;
    align-items: center;
    transform: translateY(-50%);
  }
  #miniQrSlot {
    box-sizing: border-box;
    flex: 0 0 auto;
    width: 72px;
    height: 72px;
    padding: 4px;
    border-radius: 6px;
    background: #fff;
  }
  #miniQr {
    display: block;
    width: 100%;
    height: 100%;
    image-rendering: pixelated;
  }
  #wrap.changed #record {
    animation: recordIn .76s cubic-bezier(.18,.8,.18,1) both;
  }
  .record-ghost {
    animation: recordOut .76s cubic-bezier(.18,.8,.18,1) both;
  }
  #wrap.changed #label {
    animation: textKicker .42s ease both;
  }
  #wrap.changed #artist {
    animation: textLine .54s cubic-bezier(.18,.8,.18,1) .08s both;
  }
  #wrap.changed #meta {
    animation: textLine .54s cubic-bezier(.18,.8,.18,1) .14s both;
  }
  #wrap.changed #miniClock {
    animation: miniClockIn .46s cubic-bezier(.18,.8,.18,1) both;
  }
  #wrap.changed #qrSlot {
    animation: qrChange .5s ease .18s both;
  }
  #wrap.changed #miniQrSlot {
    animation: qrChange .5s ease .18s both;
  }
  .hidden { display: none !important; }
  @media (max-width: 600px) {
    #wrap[data-theme="minimal"] #minimal { min-height: 100vh; }
    #minimalCopy { left: 12px; width: calc(100% - 82px); }
    #wrap.has-qr #minimalCopy { left: 68px; width: calc(100% - 138px); }
    #miniTime { font-size: 18px; }
    #miniTitle { font-size: 20px; }
    #miniClock { right: 10px; }
    #miniRight { left: 10px; }
    #miniQrSlot { width: 48px; height: 48px; padding: 3px; }
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes pageLoad {
    0% { opacity: 0; transform: translateY(18px) scale(.985); filter: blur(8px); }
    100% { opacity: 1; transform: translateY(0) scale(1); filter: blur(0); }
  }
  @keyframes recordOut {
    0% { opacity: 1; transform: translateX(0) scale(1); filter: brightness(1); }
    100% { opacity: 0; transform: translateX(-54px) scale(.92); filter: brightness(.72); }
  }
  @keyframes recordIn {
    0% { opacity: 0; transform: translateX(54px) scale(.92); filter: brightness(1.16); }
    100% { opacity: 1; transform: translateX(0) scale(1); filter: brightness(1); }
  }
  @keyframes textKicker {
    0% { opacity: 0; transform: translateX(-10px); }
    100% { opacity: 1; transform: translateX(0); }
  }
  @keyframes titleCharIn {
    0% { opacity: 0; transform: translateY(2px); clip-path: inset(0 100% 0 0); }
    100% { opacity: 1; transform: translateY(0); clip-path: inset(0); }
  }
  @keyframes titleCharOut {
    0% { opacity: 1; transform: translateX(0) scaleX(1); filter: blur(0); }
    100% { opacity: 0; transform: translateX(-.18em) scaleX(.25); filter: blur(2px); }
  }
  @keyframes textLine {
    0% { opacity: 0; transform: translateX(18px); }
    100% { opacity: 1; transform: translateX(0); }
  }
  @keyframes miniClockIn {
    0% { opacity: 0; transform: translateY(-5px); }
    100% { opacity: 1; transform: translateY(0); }
  }
  @keyframes qrChange {
    0% { opacity: 0; transform: translateX(12px) scale(.96); }
    100% { opacity: 1; transform: translateX(0) scale(1); }
  }
</style>
<div id="wrap" class="hidden">
  <div id="minimal">
    <div id="minimalCopy">
      <div id="miniTitle"></div>
    </div>
    <div id="miniClock">
      <div id="miniTime">--:--</div>
    </div>
    <div id="miniRight">
      <div id="miniQrSlot" class="hidden">
        <img id="miniQr" alt="">
      </div>
    </div>
  </div>
  <div id="recordSlot">
    <div id="record">
      <div class="record-disc">
        <img id="art" alt="">
        <div id="artFallback">♫</div>
      </div>
    </div>
  </div>
  <div id="copy">
    <div id="label">Now Playing</div>
    <div id="title"></div>
    <div id="artist"></div>
    <div id="meta"></div>
  </div>
  <div id="qrSlot" class="hidden">
    <img id="qr" alt="">
  </div>
</div>
<script>
const wrap = document.querySelector("#wrap");
const recordSlot = document.querySelector("#recordSlot");
const record = document.querySelector("#record");
const art = document.querySelector("#art");
const artFallback = document.querySelector("#artFallback");
const title = document.querySelector("#title");
const artist = document.querySelector("#artist");
const meta = document.querySelector("#meta");
const qrSlot = document.querySelector("#qrSlot");
const qr = document.querySelector("#qr");
const miniTitle = document.querySelector("#miniTitle");
const miniTime = document.querySelector("#miniTime");
const miniQrSlot = document.querySelector("#miniQrSlot");
const miniQr = document.querySelector("#miniQr");
const TITLE_STEP_MS = 55;
let lastTrackKey = "";
let renderedTitle = "";
let titleTimer = 0;
let artUrl = "";
let emptyTicks = 0;
let titleRenderId = 0;
const japanTimeFormatter = new Intl.DateTimeFormat("en-GB", {
  timeZone: "Asia/Tokyo",
  hour: "2-digit",
  minute: "2-digit",
  hourCycle: "h23"
});
function updateJapanClock() {
  miniTime.textContent = japanTimeFormatter.format(new Date());
}
async function tick() {
  try {
    const track = await fetch("/track", { cache: "no-store" }).then(r => r.json());
    if (!track.isPlaying && wrap.classList.contains("loaded") && emptyTicks++ < 8) return;
    if (track.isPlaying) emptyTicks = 0;
    wrap.dataset.theme = track.theme === "minimal" ? "minimal" : "record";
    document.documentElement.dataset.theme = wrap.dataset.theme;
    const trackKey = track.isPlaying ? [track.title, track.artist, track.album].join("\n") : "";
    const changed = Boolean(trackKey && trackKey !== lastTrackKey);
    const firstVisible = track.isPlaying && !wrap.classList.contains("loaded");
    if (changed && !firstVisible) cloneOutgoingRecord();
    wrap.classList.toggle("hidden", !track.isPlaying);
    if (firstVisible) wrap.classList.add("loaded");
    renderTitle(track.title || track.message || "", changed && !firstVisible);
    artist.textContent = track.artist || "";
    meta.textContent = track.album || "";
    setArtwork(track.albumArtDataUrl || "");
    setQrCode(track.qrCodeDataUrl || "");
    if (changed && !firstVisible) restartAnimation();
    lastTrackKey = trackKey;
  } catch (_) {}
}
function setQrCode(url) {
  const visible = Boolean(url);
  wrap.classList.toggle("has-qr", visible);
  qrSlot.classList.toggle("hidden", !visible);
  miniQrSlot.classList.toggle("hidden", !visible);
  if (!visible) return;
  if (qr.getAttribute("src") !== url) qr.src = url;
  if (miniQr.getAttribute("src") !== url) miniQr.src = url;
}
function cloneOutgoingRecord() {
  const ghost = record.cloneNode(true);
  ghost.removeAttribute("id");
  const ghostArt = ghost.querySelector("#art");
  const ghostFallback = ghost.querySelector("#artFallback");
  if (ghostArt) ghostArt.classList.add("record-art");
  if (ghostFallback) ghostFallback.classList.add("record-fallback");
  ghost.querySelectorAll("[id]").forEach(node => node.removeAttribute("id"));
  ghost.classList.add("record-ghost");
  recordSlot.append(ghost);
  setTimeout(() => ghost.remove(), 800);
}
function setArtwork(nextUrl) {
  if (!nextUrl) {
    artUrl = "";
    art.removeAttribute("src");
    art.classList.add("hidden");
    artFallback.classList.remove("hidden");
    return;
  }
  if (nextUrl === artUrl) return;
  artUrl = nextUrl;
  art.classList.add("hidden");
  artFallback.classList.remove("hidden");
  const img = new Image();
  img.onload = () => {
    if (artUrl !== nextUrl) return;
    art.src = nextUrl;
    art.classList.remove("hidden");
    artFallback.classList.add("hidden");
  };
  img.src = nextUrl;
}
function renderTitle(text, animated) {
  const targets = [title, miniTitle];
  if (!animated && text === renderedTitle) return;
  clearTimeout(titleTimer);
  const renderId = ++titleRenderId;
  const visibleText = targets[0].textContent.replaceAll("\u00a0", " ");
  if (!animated) {
    if (text !== renderedTitle) targets.forEach(target => target.textContent = text);
    renderedTitle = text;
    return;
  }
  if (text === visibleText) {
    targets.forEach(target => target.querySelectorAll(".title-char-out").forEach(character => {
      character.classList.remove("title-char-out");
    }));
    renderedTitle = text;
    return;
  }

  const splitCharacters = value => {
    if (typeof Intl.Segmenter === "function") {
      return [...new Intl.Segmenter("ja", { granularity: "grapheme" }).segment(value)]
        .map(item => item.segment);
    }
    return [...value];
  };
  const makeCharacter = (character, animationClass = "") => {
    const span = document.createElement("span");
    span.className = `title-char${animationClass ? ` ${animationClass}` : ""}`;
    span.textContent = character === " " ? "\u00a0" : character;
    return span;
  };
  const currentCharacters = splitCharacters(visibleText);
  const nextCharacters = splitCharacters(text);
  targets.forEach(target => target.replaceChildren(...currentCharacters.map(character => makeCharacter(character))));
  renderedTitle = text;

  const typeNextCharacter = index => {
    if (renderId !== titleRenderId || index >= nextCharacters.length) return;
    const character = nextCharacters[index];
    targets.forEach(target => target.append(makeCharacter(character, "title-char-in")));
    titleTimer = setTimeout(() => typeNextCharacter(index + 1), TITLE_STEP_MS);
  };
  const eraseLastCharacter = remaining => {
    if (renderId !== titleRenderId) return;
    if (remaining === 0) {
      typeNextCharacter(0);
      return;
    }
    targets.forEach(target => target.lastElementChild?.classList.add("title-char-out"));
    titleTimer = setTimeout(() => {
      targets.forEach(target => target.lastElementChild?.remove());
      eraseLastCharacter(remaining - 1);
    }, TITLE_STEP_MS);
  };
  eraseLastCharacter(currentCharacters.length);
}
function restartAnimation() {
  wrap.classList.remove("changed");
  void wrap.offsetWidth;
  wrap.classList.add("changed");
  setTimeout(() => wrap.classList.remove("changed"), 900);
}
updateJapanClock();
setInterval(updateJapanClock, 1000);
tick();
setInterval(tick, 1000);
</script>
</html>"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_obs_html_uses_obs_compatible_record_spin() {
        assert!(OBS_HTML.contains("@keyframes spin { to { transform: rotate(360deg); } }"));
        assert!(OBS_HTML.contains("<div class=\"record-disc\">"));
        assert!(!OBS_HTML.contains("@keyframes spin { to { rotate: 360deg; } }"));
    }

    #[test]
    fn default_obs_html_does_not_animate_whole_wrap_on_track_change() {
        assert!(!OBS_HTML.contains("#wrap.changed {\n    animation"));
        assert!(OBS_HTML.contains("emptyTicks++ < 8"));
    }

    #[test]
    fn default_obs_html_fits_browser_width() {
        assert!(OBS_HTML.contains("box-sizing: border-box;"));
        assert!(OBS_HTML.contains("width: 100vw;"));
        assert!(OBS_HTML.contains("max-width: 100vw;"));
    }

    #[test]
    fn setlog_theme_left_aligns_title_and_shows_japan_time_on_black() {
        assert!(OBS_HTML.contains("#wrap[data-theme=\"minimal\"]"));
        assert!(OBS_HTML.contains("id=\"miniTitle\""));
        assert!(OBS_HTML.contains("id=\"miniTime\""));
        assert!(OBS_HTML.contains("background: transparent;\n    box-shadow: none;"));
        assert!(!OBS_HTML.contains("id=\"miniArtist\""));
        assert!(OBS_HTML.contains("border-radius: 18px;"));
        assert!(OBS_HTML.contains("html[data-theme=\"minimal\"] body { background: #000; }"));
        assert!(OBS_HTML.contains("left: 24px;\n    width: calc(100% - 108px);"));
        assert!(OBS_HTML
            .contains("#wrap.has-qr #minimalCopy { left: 116px; width: calc(100% - 200px); }"));
        assert!(OBS_HTML.contains("#miniTitle {\n    max-width: 100%;\n    overflow: hidden;\n    font-size: 26px;\n    font-weight: 500;\n    line-height: 1.3;\n    text-align: left;"));
        assert!(OBS_HTML.contains("timeZone: \"Asia/Tokyo\""));
        assert!(!OBS_HTML.contains("miniTimeZone"));
        assert!(OBS_HTML.contains("right: 24px;"));
        assert!(OBS_HTML.contains("document.documentElement.dataset.theme = wrap.dataset.theme;"));
        let mini_title = OBS_HTML.find("<div id=\"miniTitle\"></div>").unwrap();
        let mini_clock = OBS_HTML.find("<div id=\"miniClock\">").unwrap();
        assert!(mini_title < mini_clock);
        assert!(OBS_HTML.contains("track.theme === \"minimal\""));
        assert!(OBS_HTML.contains("updateJapanClock();"));
    }

    #[test]
    fn title_switch_erases_from_right_then_types_from_left() {
        assert!(OBS_HTML.contains("const TITLE_STEP_MS = 55;"));
        assert!(OBS_HTML.contains("const typeNextCharacter = index =>"));
        assert!(OBS_HTML.contains("const eraseLastCharacter = remaining =>"));
        assert!(OBS_HTML.contains("target.lastElementChild?.classList.add(\"title-char-out\")"));
        assert!(OBS_HTML.contains("typeNextCharacter(0)"));
    }

    #[test]
    fn regular_track_polls_do_not_cancel_title_animation() {
        let unchanged_title_check = OBS_HTML
            .find("if (!animated && text === renderedTitle) return;")
            .unwrap();
        let timer_cancellation = OBS_HTML.find("clearTimeout(titleTimer);").unwrap();

        assert!(unchanged_title_check < timer_cancellation);
    }

    #[test]
    fn track_payload_uses_selected_obs_theme() {
        let state = SharedState(std::sync::Arc::new(std::sync::Mutex::new(
            crate::state::Inner {
                is_syncing: false,
                settings: crate::models::AppSettings {
                    obs_theme: "minimal".to_string(),
                    ..Default::default()
                },
                current_track: None,
                rekordbox_status: String::new(),
                history: Vec::new(),
                last_sent_message: None,
            },
        )));

        assert_eq!(obs_payload(&state).theme, "minimal");
    }

    #[test]
    fn qr_code_is_large_and_high_contrast_for_streaming() {
        let url = qr_code_data_url("https://example.test").expect("qr");
        let svg = String::from_utf8(
            BASE64
                .decode(url.strip_prefix("data:image/svg+xml;base64,").unwrap())
                .unwrap(),
        )
        .unwrap();

        let qr_image_size = QR_IMAGE_SIZE;
        assert!(qr_image_size >= 320);
        assert!(svg.contains("#000"));
        assert!(OBS_HTML.contains("width: 156px;"));
        assert!(OBS_HTML.contains("background: #fff;"));
        assert!(OBS_HTML.contains("image-rendering: pixelated;"));
    }
}
