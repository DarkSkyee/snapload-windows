import { api } from "../shared/browser-api";

let currentMediaType: "video" | "audio" = "video";

const urlInput = document.getElementById("url-input") as HTMLInputElement;
const pasteBtn = document.getElementById("paste-btn") as HTMLButtonElement;
const typeVideoBtn = document.getElementById("type-video") as HTMLButtonElement;
const typeAudioBtn = document.getElementById("type-audio") as HTMLButtonElement;
const formatSelect = document.getElementById("format-select") as HTMLSelectElement;
const qualitySelect = document.getElementById("quality-select") as HTMLSelectElement;
const downloadBtn = document.getElementById("download-btn") as HTMLButtonElement;
const btnText = document.getElementById("btn-text") as HTMLElement;
const statusBadge = document.getElementById("engine-status") as HTMLElement;
const statusLabel = document.getElementById("status-label") as HTMLElement;
const downloadsContainer = document.getElementById("downloads-container") as HTMLElement;
const downloadsList = document.getElementById("downloads-list") as HTMLElement;

const videoFormats = [
  { value: "mp4", label: "MP4" },
  { value: "mkv", label: "MKV" },
  { value: "webm", label: "WebM" },
];

const videoQualities = [
  { value: "best", label: "Best (Original / 4K)" },
  { value: "p1080", label: "1080p FHD" },
  { value: "p720", label: "720p HD" },
  { value: "p480", label: "480p SD" },
];

const audioFormats = [
  { value: "mp3", label: "MP3" },
  { value: "m4a", label: "M4A" },
  { value: "flac", label: "FLAC (Lossless)" },
  { value: "wav", label: "WAV" },
  { value: "opus", label: "OPUS" },
];

const audioQualities = [
  { value: "best", label: "320 kbps (Best)" },
  { value: "high", label: "256 kbps" },
  { value: "medium", label: "192 kbps" },
  { value: "low", label: "128 kbps" },
];

function updateSelectors() {
  formatSelect.innerHTML = "";
  qualitySelect.innerHTML = "";

  const formats = currentMediaType === "video" ? videoFormats : audioFormats;
  for (const fmt of formats) {
    const opt = document.createElement("option");
    opt.value = fmt.value;
    opt.textContent = fmt.label;
    formatSelect.appendChild(opt);
  }

  const qualities = currentMediaType === "video" ? videoQualities : audioQualities;
  for (const q of qualities) {
    const opt = document.createElement("option");
    opt.value = q.value;
    opt.textContent = q.label;
    qualitySelect.appendChild(opt);
  }
}

function setMediaType(type: "video" | "audio") {
  currentMediaType = type;
  if (type === "video") {
    typeVideoBtn.classList.add("active");
    typeAudioBtn.classList.remove("active");
  } else {
    typeAudioBtn.classList.add("active");
    typeVideoBtn.classList.remove("active");
  }
  updateSelectors();
}

typeVideoBtn.addEventListener("click", () => setMediaType("video"));
typeAudioBtn.addEventListener("click", () => setMediaType("audio"));

pasteBtn.addEventListener("click", async () => {
  try {
    const text = await navigator.clipboard.readText();
    if (text) {
      urlInput.value = text.trim();
    }
  } catch (e) {
    console.warn("Clipboard access denied:", e);
  }
});

downloadBtn.addEventListener("click", () => {
  const url = urlInput.value.trim();
  if (!url) {
    urlInput.focus();
    return;
  }

  btnText.textContent = "Starting...";
  downloadBtn.disabled = true;

  const payload = {
    url,
    media_type: currentMediaType,
    video_format: currentMediaType === "video" ? formatSelect.value : undefined,
    video_quality: currentMediaType === "video" ? qualitySelect.value : undefined,
    audio_format: currentMediaType === "audio" ? formatSelect.value : undefined,
    audio_quality: currentMediaType === "audio" ? qualitySelect.value : undefined,
  };

  api.runtime.sendMessage({
    type: "START_DOWNLOAD",
    payload,
  }, (resp: any) => {
    downloadBtn.disabled = false;
    if (resp && resp.success) {
      btnText.textContent = "Download Queued ✓";
      setTimeout(() => {
        btnText.textContent = "Download with SnapLoad";
      }, 2000);
      refreshActiveDownloads();
    } else {
      btnText.textContent = "Connection Error";
      setTimeout(() => {
        btnText.textContent = "Download with SnapLoad";
      }, 3000);
    }
  });
});

function refreshActiveDownloads() {
  api.runtime.sendMessage({ type: "GET_ACTIVE_DOWNLOADS" }, (response: any) => {
    if (!response || !response.downloads || response.downloads.length === 0) {
      downloadsContainer.classList.add("hidden");
      downloadsList.innerHTML = "";
      return;
    }

    downloadsContainer.classList.remove("hidden");
    downloadsList.innerHTML = "";

    for (const dl of response.downloads) {
      const item = document.createElement("div");
      item.className = "download-item";

      const infoRow = document.createElement("div");
      infoRow.className = "download-info-row";
      infoRow.innerHTML = `
        <span class="download-url" title="${dl.url}">${dl.url}</span>
        <span class="download-percent">${dl.percent}%</span>
      `;

      const barBg = document.createElement("div");
      barBg.className = "progress-bar-bg";
      barBg.innerHTML = `<div class="progress-bar-fill" style="width: ${dl.percent}%;"></div>`;

      const metaRow = document.createElement("div");
      metaRow.className = "download-meta-row";
      const statusText = dl.completed ? "Saved ✓" : (dl.error || dl.status || "Downloading");
      metaRow.innerHTML = `
        <span>${statusText} ${dl.speed ? "• " + dl.speed : ""} ${dl.eta ? "• ETA " + dl.eta : ""}</span>
        ${!dl.completed && !dl.error ? `<span class="cancel-link" data-id="${dl.id}">Cancel</span>` : ""}
      `;

      item.appendChild(infoRow);
      item.appendChild(barBg);
      item.appendChild(metaRow);

      const cancelBtn = item.querySelector(".cancel-link");
      if (cancelBtn) {
        cancelBtn.addEventListener("click", () => {
          api.runtime.sendMessage({
            type: "CANCEL_DOWNLOAD",
            payload: { download_id: dl.id },
          });
        });
      }

      downloadsList.appendChild(item);
    }
  });
}

function checkEngineStatus() {
  api.runtime.sendMessage({ type: "PING_ENGINE" }, (response: any) => {
    if (response && response.connected) {
      statusBadge.className = "status-badge connected";
      statusLabel.textContent = "Engine Ready";
    } else {
      statusBadge.className = "status-badge disconnected";
      statusLabel.textContent = "Engine Offline";
    }
  });
}

// Listen for push notifications from background script
api.runtime.onMessage.addListener((message: any) => {
  if (message.type === "ENGINE_EVENT" || message.type === "ENGINE_STATUS") {
    checkEngineStatus();
    refreshActiveDownloads();
  }
});

// Initialize on popup open
document.addEventListener("DOMContentLoaded", () => {
  updateSelectors();

  // Query active tab URL
  api.tabs.query({ active: true, currentWindow: true }, (tabs: any[]) => {
    if (tabs && tabs[0] && tabs[0].url) {
      const currentUrl = tabs[0].url;
      if (currentUrl.startsWith("http://") || currentUrl.startsWith("https://")) {
        urlInput.value = currentUrl;
      }
    }
  });

  checkEngineStatus();
  refreshActiveDownloads();

  // Poll active downloads every second while popup is visible
  setInterval(refreshActiveDownloads, 1000);
});
