const { invoke } = window.__TAURI__.core;

const $ = (selector) => document.querySelector(selector);
const views = [...document.querySelectorAll(".view")];
const navButtons = [...document.querySelectorAll(".nav")];

const el = {
  nowCard: $(".now-card"),
  albumArt: $("#album-art"),
  title: $("#track-title"),
  artist: $("#track-artist"),
  album: $("#track-album"),
  metrics: $("#track-metrics"),
  rekordboxStatus: $("#rekordbox-status"),
  qrUrl: $("#track-qr-url"),
  syncDot: $("#sync-dot"),
  syncLabel: $("#sync-label"),
  destination: $("#osc-destination"),
  obsUrl: $("#obs-url"),
  start: $("#start-button"),
  stop: $("#stop-button"),
  historyList: $("#history-list"),
  historyEmpty: $("#history-empty"),
  clearHistory: $("#clear-history"),
  form: $("#settings-form"),
  saveState: $("#save-state"),
  host: $("#osc-host"),
  port: $("#osc-port"),
  delay: $("#osc-delay"),
  format: $("#message-format"),
  preview: $("#format-preview"),
  interval: $("#polling-interval"),
  notification: $("#notification-sound"),
  obsPort: $("#obs-port"),
  obsTheme: $("#obs-theme"),
  obsPreview: $("#obs-preview"),
  obsShowQr: $("#obs-show-qr"),
  obsHtml: $("#obs-html"),
};

let snapshot = null;
let editingSettings = false;
let saveTimer = 0;
let autoSaveTimer = 0;
let lastTrackKey = null;
let editingQrUrl = false;

for (const button of navButtons) {
  button.addEventListener("click", () => {
    for (const nav of navButtons) nav.classList.toggle("active", nav === button);
    for (const view of views) view.classList.toggle("active", view.id === button.dataset.view);
  });
}

el.start.addEventListener("click", async () => {
  if (await saveSettings(false)) render(await invoke("start_sync"));
});
el.stop.addEventListener("click", async () => render(await invoke("stop_sync")));
el.clearHistory.addEventListener("click", async () => render(await invoke("clear_history")));

el.qrUrl.addEventListener("focusin", () => {
  editingQrUrl = true;
});

el.qrUrl.addEventListener("focusout", () => {
  editingQrUrl = false;
  renderDashboard();
});

el.qrUrl.addEventListener("input", () => {
  editingQrUrl = true;
  scheduleAutoSave();
});

el.form.addEventListener("focusin", () => {
  editingSettings = true;
});

el.form.addEventListener("focusout", (event) => {
  if (!event.currentTarget.contains(event.relatedTarget)) {
    editingSettings = false;
  }
});

el.form.addEventListener("submit", async (event) => {
  event.preventDefault();
  editingSettings = false;
  await saveSettings(true);
});

for (const input of [
  el.host,
  el.port,
  el.delay,
  el.format,
  el.interval,
  el.notification,
  el.obsPort,
  el.obsTheme,
  el.obsShowQr,
  el.obsHtml,
]) {
  input.addEventListener(input.type === "checkbox" || input.tagName === "SELECT" ? "change" : "input", () => {
    editingSettings = true;
    updateSettingsPreview();
    scheduleAutoSave();
  });
}

render(await invoke("get_snapshot"));
setInterval(async () => {
  if (!editingSettings) render(await invoke("get_snapshot"));
}, 1000);

function render(next) {
  snapshot = next;
  renderDashboard();
  renderHistory();
  renderSettings();
}

function renderDashboard() {
  const track = snapshot.currentTrack;
  el.syncDot.classList.toggle("active", snapshot.isSyncing);
  el.syncLabel.textContent = snapshot.isSyncing ? "Sync Active" : "Disconnected";
  el.destination.textContent = `${snapshot.settings.oscHost}:${snapshot.settings.oscPort}`;
  el.obsUrl.textContent = `0.0.0.0:${snapshot.settings.obsPort}`;
  el.start.disabled = snapshot.isSyncing;
  el.stop.disabled = !snapshot.isSyncing;

  el.title.textContent = track?.title || "Waiting for track...";
  el.artist.textContent = track ? track.artist || "Unknown artist" : "No track detected.";
  el.album.textContent = track?.album || "";
  el.metrics.textContent = track
    ? [formatBpm(track.bpm), formatTime(track.positionSeconds, track.durationSeconds), track.source]
        .filter(Boolean)
        .join(" · ")
    : "";
  el.rekordboxStatus.textContent = snapshot.rekordboxStatus || "";
  el.albumArt.hidden = !track?.albumArtDataUrl;
  el.albumArt.src = track?.albumArtDataUrl || "";
  renderQrUrl(track);

  const key = trackKey(track);
  if (key && key !== lastTrackKey) restartAnimation(el.nowCard, "track-changed");
  lastTrackKey = key;
}

function renderQrUrl(track) {
  const key = trackKey(track);
  el.qrUrl.disabled = !key;
  el.qrUrl.placeholder = key ? "https://..." : "Play a track to set a URL";
  if (!editingQrUrl) {
    el.qrUrl.value = key ? snapshot.settings.obsQrUrls?.[key] || "" : "";
  }
}

function renderHistory() {
  const history = snapshot.history ?? [];
  el.historyEmpty.hidden = history.length > 0;
  el.historyList.innerHTML = "";

  for (const item of history) {
    const row = document.createElement("div");
    row.className = "history-row";

    const time = document.createElement("div");
    time.className = "history-time";
    time.textContent = new Date(Number(item.playedAtMs)).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

    const copy = document.createElement("div");
    const title = document.createElement("div");
    title.className = "history-title";
    title.textContent = item.title || "Untitled";

    const meta = document.createElement("div");
    meta.className = "history-meta";
    meta.textContent = [item.artist, item.album].filter(Boolean).join(" · ");

    copy.append(title, meta);
    row.append(time, copy);
    el.historyList.append(row);
  }
}

function renderSettings() {
  if (editingSettings) return;
  const settings = snapshot.settings;
  el.host.value = settings.oscHost;
  el.port.value = settings.oscPort;
  el.delay.value = settings.oscDelayMs;
  el.format.value = settings.messageFormat;
  el.interval.value = settings.pollingIntervalMs;
  el.notification.checked = settings.sendNotificationSound;
  el.obsPort.value = settings.obsPort;
  el.obsTheme.value = settings.obsTheme;
  el.obsShowQr.checked = settings.obsShowQr;
  el.obsHtml.value = settings.obsHtml;
  updateSettingsPreview();
}

function readSettings() {
  return {
    oscHost: el.host.value.trim() || "127.0.0.1",
    oscPort: clamp(safeNumber(el.port.value, 9000), 1, 65535),
    oscDelayMs: clamp(safeNumber(el.delay.value, 0), 0, 30000),
    pollingIntervalMs: clamp(safeNumber(el.interval.value, 3000), 500, 30000),
    messageFormat: el.format.value,
    sendNotificationSound: el.notification.checked,
    obsPort: clamp(safeNumber(el.obsPort.value, 17777), 1, 65535),
    obsTheme: el.obsTheme.value,
    obsShowQr: el.obsShowQr.checked,
    obsQrUrls: readQrUrls(),
    obsHtml: el.obsHtml.value,
  };
}

function readQrUrls() {
  const urls = { ...(snapshot?.settings?.obsQrUrls ?? {}) };
  const key = trackKey(snapshot?.currentTrack);
  if (!key) return urls;

  const url = el.qrUrl.value.trim();
  if (url) urls[key] = url;
  else delete urls[key];
  return urls;
}

function trackKey(track) {
  if (!track) return "";
  return track.filePath?.trim() || [track.title, track.artist, track.album].join("|");
}

function updatePreview() {
  const sample = {
    title: "How Many Boogie",
    artist: "SKRYU; WAZGOGG; Fuma no KTR",
    album: "わすれもの",
    bpm: 128,
    positionSeconds: 65,
    durationSeconds: 214,
  };
  el.preview.textContent = formatMessage(el.format.value, sample);
}

function updateSettingsPreview() {
  updatePreview();
  el.obsPreview.textContent = `http://0.0.0.0:${safeNumber(el.obsPort.value, 17777)}/`;
}

function formatMessage(format, track) {
  let result = format || "";
  result = result.replaceAll("{title}", track.title);
  result = result.replaceAll("{artist}", track.artist);
  result = result.replaceAll("{album}", track.album);
  result = result.replaceAll("{bpm}", formatBpm(track.bpm));
  result = result.replaceAll("{time}", formatTime(track.positionSeconds, track.durationSeconds));
  return result.trim() || "(empty)";
}

function formatBpm(value) {
  return Number.isFinite(value) && value > 0 ? `${value.toFixed(1)} BPM` : "";
}

function formatTime(position, duration) {
  if (!Number.isFinite(position)) return "";
  const current = mmss(position);
  return Number.isFinite(duration) && duration > 0 ? `${current} / ${mmss(duration)}` : current;
}

function mmss(value) {
  const seconds = Math.max(0, Math.round(value));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function safeNumber(value, fallback) {
  const number = Number(value);
  return Number.isFinite(number) ? Math.round(number) : fallback;
}

function flashSaved(message = "Settings saved") {
  clearTimeout(saveTimer);
  el.saveState.textContent = message;
  saveTimer = setTimeout(() => {
    el.saveState.textContent = "";
  }, 1800);
}

function scheduleAutoSave() {
  clearTimeout(autoSaveTimer);
  el.saveState.textContent = "Saving...";
  autoSaveTimer = setTimeout(() => saveSettings(false), 600);
}

async function saveSettings(showSaved) {
  clearTimeout(autoSaveTimer);
  try {
    render(await invoke("save_settings", { settings: readSettings() }));
    flashSaved(showSaved ? "Settings saved" : "Saved");
    return true;
  } catch (error) {
    el.saveState.textContent = `Save failed: ${String(error)}`;
    return false;
  }
}

function restartAnimation(element, className) {
  element.classList.remove(className);
  void element.offsetWidth;
  element.classList.add(className);
  setTimeout(() => element.classList.remove(className), 900);
}
