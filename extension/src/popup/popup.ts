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

function setMediaType(type: "video" | "audio", save = true) {
  currentMediaType = type;
  if (type === "video") {
    typeVideoBtn.classList.add("active");
    typeAudioBtn.classList.remove("active");
  } else {
    typeAudioBtn.classList.add("active");
    typeVideoBtn.classList.remove("active");
  }
  updateSelectors();
  if (save) {
    api.storage?.local?.set({ mediaType: type });
  }
}

typeVideoBtn.addEventListener("click", () => setMediaType("video"));
typeAudioBtn.addEventListener("click", () => setMediaType("audio"));

formatSelect.addEventListener("change", () => {
  if (currentMediaType === "video") {
    api.storage?.local?.set({ videoFormat: formatSelect.value });
  } else {
    api.storage?.local?.set({ audioFormat: formatSelect.value });
  }
});

qualitySelect.addEventListener("change", () => {
  if (currentMediaType === "video") {
    api.storage?.local?.set({ videoQuality: qualitySelect.value });
  } else {
    api.storage?.local?.set({ audioQuality: qualitySelect.value });
  }
});

function checkAutoAudioUrl(url: string) {
  const lower = url.toLowerCase();
  if (
    lower.includes("spotify.com") ||
    lower.includes("spotify.link") ||
    lower.startsWith("spotify:") ||
    lower.includes("soundcloud.com")
  ) {
    setMediaType("audio", false);
  }
}

urlInput.addEventListener("input", () => {
  checkAutoAudioUrl(urlInput.value);
});

pasteBtn.addEventListener("click", async () => {
  try {
    const text = await navigator.clipboard.readText();
    if (text) {
      urlInput.value = text.trim();
      checkAutoAudioUrl(urlInput.value);
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
      const isFailed = dl.status === "Failed" || !!dl.error;
      const isCompleted = !!dl.completed;
      item.className = `download-item ${isFailed ? "failed" : ""} ${isCompleted ? "completed" : ""}`.trim();

      const percentDisplay = isFailed ? "Error" : (isCompleted ? "100%" : `${dl.percent}%`);
      const statusText = isCompleted ? "Saved to Downloads ✓" : (isFailed ? `Failed: ${dl.error || "Error"}` : (dl.status || "Downloading"));

      const infoRow = document.createElement("div");
      infoRow.className = "download-info-row";
      infoRow.innerHTML = `
        <span class="download-url" title="${dl.url}">${dl.url}</span>
        <span class="download-percent" style="${isFailed ? 'color: #EF4444;' : (isCompleted ? 'color: #10B981;' : '')}">${percentDisplay}</span>
      `;

      const barBg = document.createElement("div");
      barBg.className = "progress-bar-bg";
      const barFillStyle = isFailed 
        ? "background: #EF4444; width: 100%;" 
        : (isCompleted ? "background: #10B981; width: 100%;" : `width: ${dl.percent}%;`);
      barBg.innerHTML = `<div class="progress-bar-fill" style="${barFillStyle}"></div>`;

      const metaRow = document.createElement("div");
      metaRow.className = "download-meta-row";
      const actionLink = (!isCompleted && !isFailed) 
        ? `<span class="cancel-link" data-id="${dl.id}">Cancel</span>`
        : `<span class="dismiss-link" data-id="${dl.id}" style="color: #64748B; cursor: pointer;">Dismiss</span>`;

      metaRow.innerHTML = `
        <span style="${isFailed ? 'color: #F87171;' : ''}">${statusText} ${dl.speed ? "• " + dl.speed : ""} ${dl.eta ? "• ETA " + dl.eta : ""}</span>
        ${actionLink}
      `;

      item.appendChild(infoRow);
      item.appendChild(barBg);
      item.appendChild(metaRow);

      const cancelBtn = item.querySelector(".cancel-link");
      if (cancelBtn) {
        cancelBtn.addEventListener("click", (e) => {
          e.preventDefault();
          e.stopPropagation();
          (cancelBtn as HTMLElement).textContent = "Cancelling...";
          (cancelBtn as HTMLElement).style.pointerEvents = "none";
          (cancelBtn as HTMLElement).style.opacity = "0.6";

          api.runtime.sendMessage({
            type: "CANCEL_DOWNLOAD",
            payload: { download_id: dl.id },
          }, () => {
            setTimeout(refreshActiveDownloads, 120);
          });
        });
      }

      const dismissBtn = item.querySelector(".dismiss-link");
      if (dismissBtn) {
        dismissBtn.addEventListener("click", (e) => {
          e.preventDefault();
          e.stopPropagation();
          (dismissBtn as HTMLElement).textContent = "Dismissing...";
          (dismissBtn as HTMLElement).style.pointerEvents = "none";

          api.runtime.sendMessage({
            type: "DISMISS_DOWNLOAD",
            payload: { download_id: dl.id },
          }, () => {
            setTimeout(refreshActiveDownloads, 120);
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
  if (message.type === "ENGINE_EVENT") {
    refreshActiveDownloads();
  } else if (message.type === "ENGINE_STATUS") {
    checkEngineStatus();
    refreshActiveDownloads();
  }
});

// Initialize on popup open
document.addEventListener("DOMContentLoaded", () => {
  updateSelectors();

  // Restore saved preferences
  try {
    api.storage?.local?.get(
      ["mediaType", "videoFormat", "videoQuality", "audioFormat", "audioQuality"],
      (stored: any) => {
        if (stored) {
          if (stored.mediaType === "audio" || stored.mediaType === "video") {
            setMediaType(stored.mediaType, false);
          }
          if (currentMediaType === "video") {
            if (stored.videoFormat) formatSelect.value = stored.videoFormat;
            if (stored.videoQuality) qualitySelect.value = stored.videoQuality;
          } else {
            if (stored.audioFormat) formatSelect.value = stored.audioFormat;
            if (stored.audioQuality) qualitySelect.value = stored.audioQuality;
          }
        }
      }
    );
  } catch (_) {}

  // Query active tab URL
  api.tabs.query({ active: true, currentWindow: true }, (tabs: any[]) => {
    if (tabs && tabs[0] && tabs[0].url) {
      const currentUrl = tabs[0].url;
      if (currentUrl.startsWith("http://") || currentUrl.startsWith("https://")) {
        urlInput.value = currentUrl;
        checkAutoAudioUrl(currentUrl);
      }
    }
  });

  checkEngineStatus();
  refreshActiveDownloads();

  // Poll active downloads every second while popup is visible
  setInterval(refreshActiveDownloads, 1000);
});
