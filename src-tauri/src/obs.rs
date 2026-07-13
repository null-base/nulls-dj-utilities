use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use qrcode::{render::svg, QrCode};

use crate::{models::ObsPayload, state::SharedState};

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
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            album_art_data_url: track.album_art_data_url.clone(),
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
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            album_art_data_url: None,
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

    let code = QrCode::new(url).ok()?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(128, 128)
        .dark_color(svg::Color("#111"))
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
    font-family: Inter, system-ui, -apple-system, "Segoe UI", sans-serif;
  }
  #wrap {
    display: flex;
    align-items: center;
    gap: 28px;
    width: 100%;
    max-width: 100%;
    padding: 20px 28px 20px 20px;
    border-radius: 24px;
    background: linear-gradient(135deg, rgba(9, 11, 13, 0.86), rgba(25, 29, 31, 0.72));
    box-shadow: 0 18px 52px rgba(0,0,0,.38);
    color: white;
    text-shadow: 0 2px 6px rgba(0,0,0,.45);
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
    background:
      radial-gradient(circle at center, transparent 0 33%, rgba(255,255,255,.12) 34% 35%, transparent 36%),
      repeating-radial-gradient(circle at center, #111 0 7px, #1a1b1c 8px 10px),
      radial-gradient(circle at 34% 30%, rgba(255,255,255,.18), transparent 28%),
      #111;
    box-shadow: inset 0 0 0 2px rgba(255,255,255,.08), inset 0 0 34px rgba(0,0,0,.9), 0 12px 32px rgba(0,0,0,.45);
    animation: spin 5s linear infinite;
  }
  #record::after,
  .record-ghost::after {
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
    animation: titleCharIn .78s cubic-bezier(.18,.8,.18,1) both;
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
    width: 116px;
    height: 116px;
    padding: 8px;
    border-radius: 16px;
    background: rgba(255,255,255,.94);
    box-shadow: 0 12px 28px rgba(0,0,0,.28);
  }
  #qr {
    display: block;
    width: 100%;
    height: 100%;
  }
  #wrap.changed {
    animation: wrapChange .6s ease both;
  }
  #wrap.changed #record {
    animation: spin 5s linear infinite, recordIn .76s cubic-bezier(.18,.8,.18,1) both;
  }
  .record-ghost {
    animation: spin 5s linear infinite, recordOut .76s cubic-bezier(.18,.8,.18,1) both;
  }
  #wrap.changed #label {
    animation: textKicker .42s ease both;
  }
  #title.title-out {
    animation: titleWipeOut .55s ease both;
  }
  #wrap.changed #artist {
    animation: textLine .54s cubic-bezier(.18,.8,.18,1) .08s both;
  }
  #wrap.changed #meta {
    animation: textLine .54s cubic-bezier(.18,.8,.18,1) .14s both;
  }
  #wrap.changed #qrSlot {
    animation: qrChange .5s ease .18s both;
  }
  .hidden { display: none !important; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes pageLoad {
    0% { opacity: 0; transform: translateY(18px) scale(.985); filter: blur(8px); }
    100% { opacity: 1; transform: translateY(0) scale(1); filter: blur(0); }
  }
  @keyframes wrapChange {
    0% { opacity: .94; filter: brightness(1.08); }
    100% { opacity: 1; filter: brightness(1); }
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
  @keyframes titleWipeOut {
    0% { opacity: 1; transform: translateY(0); clip-path: inset(0 0 0 0); }
    100% { opacity: .35; transform: translateY(12px); clip-path: inset(100% 0 0 0); }
  }
  @keyframes titleCharIn {
    0% { opacity: 0; transform: translateY(-18px) scale(.96); filter: blur(3px); }
    100% { opacity: 1; transform: translateY(0) scale(1); filter: blur(0); }
  }
  @keyframes textLine {
    0% { opacity: 0; transform: translateX(18px); }
    100% { opacity: 1; transform: translateY(0); }
  }
  @keyframes qrChange {
    0% { opacity: 0; transform: translateX(12px) scale(.96); }
    100% { opacity: 1; transform: translateX(0) scale(1); }
  }
</style>
<div id="wrap" class="hidden">
  <div id="recordSlot">
    <div id="record">
      <img id="art" alt="">
      <div id="artFallback">♫</div>
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
let lastTrackKey = "";
let renderedTitle = "";
let titleTimer = 0;
async function tick() {
  try {
    const track = await fetch("/track", { cache: "no-store" }).then(r => r.json());
    const trackKey = track.isPlaying ? [track.title, track.artist, track.album].join("\n") : "";
    const changed = trackKey && trackKey !== lastTrackKey;
    const firstVisible = track.isPlaying && !wrap.classList.contains("loaded");
    if (changed) cloneOutgoingRecord();
    wrap.classList.toggle("hidden", !track.isPlaying);
    if (firstVisible) wrap.classList.add("loaded");
    renderTitle(track.title || track.message || "", changed);
    artist.textContent = track.artist || "";
    meta.textContent = track.album || "";
    art.classList.toggle("hidden", !track.albumArtDataUrl);
    artFallback.classList.toggle("hidden", !!track.albumArtDataUrl);
    if (track.albumArtDataUrl && art.src !== track.albumArtDataUrl) art.src = track.albumArtDataUrl;
    qrSlot.classList.toggle("hidden", !track.qrCodeDataUrl);
    if (track.qrCodeDataUrl && qr.src !== track.qrCodeDataUrl) qr.src = track.qrCodeDataUrl;
    if (changed && !firstVisible) restartAnimation();
    lastTrackKey = trackKey;
  } catch (_) {}
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
function renderTitle(text, animated) {
  clearTimeout(titleTimer);
  if (!animated) {
    if (text !== renderedTitle) title.textContent = text;
    renderedTitle = text;
    return;
  }
  title.classList.add("title-out");
  titleTimer = setTimeout(() => {
    title.classList.remove("title-out");
    title.textContent = "";
    const chars = [...text];
    const order = chars.map((_, index) => index).sort(() => Math.random() - .5);
    const delays = new Map(order.map((index, rank) => [index, rank * 32]));
    chars.forEach((char, index) => {
      const span = document.createElement("span");
      span.className = "title-char";
      span.style.animationDelay = `${delays.get(index)}ms`;
      span.textContent = char === " " ? "\u00a0" : char;
      title.append(span);
    });
    renderedTitle = text;
  }, 550);
}
function restartAnimation() {
  wrap.classList.remove("changed");
  void wrap.offsetWidth;
  wrap.classList.add("changed");
  setTimeout(() => wrap.classList.remove("changed"), 900);
}
tick();
setInterval(tick, 1000);
</script>
</html>"##;
