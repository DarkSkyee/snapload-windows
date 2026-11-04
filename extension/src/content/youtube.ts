import { api } from "../shared/browser-api";

const PLAYER_CONTAINER_ID = "snapload-ytp-container";
const PLAYER_BUTTON_ID = "snapload-ytp-btn";
const OPTIONS_BUTTON_ID = "snapload-ytp-options-btn";
const DROPDOWN_MENU_ID = "snapload-ytp-dropdown";
const ACTION_BUTTON_ID = "snapload-yt-action-button";
const SHORTS_BUTTON_ID = "snapload-shorts-btn";

let observer: MutationObserver | null = null;
let lastUrl = "";
let debounceTimer: any = null;

/**
 * Returns a canonical, clean YouTube URL free of playlist index, timestamps, and tracking tags.
 */
function getCanonicalVideoUrl(): string {
  try {
    const url = new URL(window.location.href);
    if (url.pathname.startsWith("/watch")) {
      const v = url.searchParams.get("v");
      if (v) {
        return `https://www.youtube.com/watch?v=${encodeURIComponent(v)}`;
      }
    } else if (url.pathname.startsWith("/shorts/")) {
      const shortId = url.pathname.replace(/^\/shorts\//, "").split("/")[0].split("?")[0];
      if (shortId) {
        return `https://www.youtube.com/shorts/${encodeURIComponent(shortId)}`;
      }
    }
  } catch (_) {}
  return window.location.href;
}

/**
 * Locates YouTube active player left-side controls container, filtering out inline preview player.
 */
function getLeftControls(): Element | null {
  try {
    const moviePlayer = document.getElementById("movie_player");
    if (moviePlayer) {
      const left = moviePlayer.querySelector(".ytp-left-controls");
      if (left) return left;
    }

    const player =
      document.querySelector(".html5-video-player:not(#inline-preview-player)") ||
      document.querySelector(".html5-video-player");

    if (player) {
      const left = player.querySelector(".ytp-left-controls");
      if (left) return left;
    }

    const all = document.querySelectorAll(".ytp-left-controls");
    for (const el of all) {
      if (!el.closest("#inline-preview-player")) {
        return el;
      }
    }
  } catch (_) {}
  return null;
}

// ==========================================
// 1. User Preferences & Storage Sync
// ==========================================
interface ExtensionPrefs {
  mediaType: "video" | "audio";
  videoFormat: string;
  videoQuality: string;
  audioFormat: string;
  audioQuality: string;
}

const currentPrefs: ExtensionPrefs = {
  mediaType: "video",
  videoFormat: "mp4",
  videoQuality: "best",
  audioFormat: "mp3",
  audioQuality: "best",
};

function loadStoredPreferences() {
  try {
    api.storage?.local?.get(
      ["mediaType", "videoFormat", "videoQuality", "audioFormat", "audioQuality"],
      (stored: any) => {
        if (stored) {
          if (stored.mediaType) currentPrefs.mediaType = stored.mediaType;
          if (stored.videoFormat) currentPrefs.videoFormat = stored.videoFormat;
          if (stored.videoQuality) currentPrefs.videoQuality = stored.videoQuality;
          if (stored.audioFormat) currentPrefs.audioFormat = stored.audioFormat;
          if (stored.audioQuality) currentPrefs.audioQuality = stored.audioQuality;
          updatePlayerLabels();
        }
      }
    );
  } catch (_) {}
}

try {
  api.storage?.onChanged?.addListener((changes, area) => {
    if (area === "local") {
      if (changes.mediaType) currentPrefs.mediaType = changes.mediaType.newValue;
      if (changes.videoFormat) currentPrefs.videoFormat = changes.videoFormat.newValue;
      if (changes.videoQuality) currentPrefs.videoQuality = changes.videoQuality.newValue;
      if (changes.audioFormat) currentPrefs.audioFormat = changes.audioFormat.newValue;
      if (changes.audioQuality) currentPrefs.audioQuality = changes.audioQuality.newValue;
      updatePlayerLabels();
    }
  });
} catch (_) {}

let isGlobalDismissBound = false;
function ensureGlobalDismiss() {
  if (isGlobalDismissBound) return;
  isGlobalDismissBound = true;
  document.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;
    const container = document.getElementById(PLAYER_CONTAINER_ID);
    if (container && !container.contains(target)) {
      const menu = container.querySelector(`#${DROPDOWN_MENU_ID}`);
      const optBtn = container.querySelector(`#${OPTIONS_BUTTON_ID}`);
      if (menu) menu.classList.add("hidden");
      if (optBtn) optBtn.setAttribute("aria-expanded", "false");
    }
  });
}

function updatePlayerLabels() {
  const container = document.getElementById(PLAYER_CONTAINER_ID);
  if (!container) return;

  const btn = container.querySelector(`#${PLAYER_BUTTON_ID}`) as HTMLButtonElement | null;
  const activeFmt = currentPrefs.mediaType === "audio" 
    ? currentPrefs.audioFormat.toUpperCase() 
    : currentPrefs.videoFormat.toUpperCase();
  if (btn && !btn.classList.contains("downloading")) {
    btn.setAttribute("title", `Download with SnapLoad (${activeFmt})`);
  }

  const videoItem = container.querySelector('[data-action="download-video"]');
  if (videoItem) {
    const titleSpan = videoItem.querySelector(".snapload-menu-title");
    const badgeSpan = videoItem.querySelector(".snapload-menu-badge");
    if (titleSpan) {
      titleSpan.textContent = `Download ${currentPrefs.videoFormat.toUpperCase()} (Video)`;
    }
    if (badgeSpan) {
      badgeSpan.textContent = currentPrefs.videoFormat.toUpperCase() + (currentPrefs.mediaType === "video" ? " • Default" : "");
    }
  }

  const audioItem = container.querySelector('[data-action="download-audio"]');
  if (audioItem) {
    const titleSpan = audioItem.querySelector(".snapload-menu-title");
    const badgeSpan = audioItem.querySelector(".snapload-menu-badge");
    if (titleSpan) {
      titleSpan.textContent = `Download ${currentPrefs.audioFormat.toUpperCase()} (Audio)`;
    }
    if (badgeSpan) {
      badgeSpan.textContent = currentPrefs.audioFormat.toUpperCase() + (currentPrefs.mediaType === "audio" ? " • Default" : "");
    }
  }
}

// ==========================================
// 2. YouTube Player Control Bar Button & 3-Dots Options Menu
// ==========================================
function createPlayerContainer(): HTMLElement {
  ensureGlobalDismiss();
  loadStoredPreferences();

  const container = document.createElement("div");
  container.id = PLAYER_CONTAINER_ID;
  container.className = "snapload-ytp-container";

  const defaultFmt = currentPrefs.mediaType === "audio"
    ? currentPrefs.audioFormat.toUpperCase()
    : currentPrefs.videoFormat.toUpperCase();

  // 1. Main Download Button
  const btn = document.createElement("button");
  btn.id = PLAYER_BUTTON_ID;
  btn.className = "ytp-button snapload-ytp-btn";
  btn.type = "button";
  btn.setAttribute("title", `Download with SnapLoad (${defaultFmt})`);
  btn.setAttribute("aria-label", "Download with SnapLoad");
  btn.innerHTML = `
    <svg height="100%" version="1.1" viewBox="0 0 36 36" width="100%">
      <defs>
        <linearGradient id="snapYtpGrad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#00F2FE"/>
          <stop offset="100%" stop-color="#818CF8"/>
        </linearGradient>
      </defs>
      <g class="snapload-ytp-icon-normal">
        <path d="M 10 24 L 10 26 C 10 26.5 10.5 27 11 27 L 25 27 C 25.5 27 26 26.5 26 26 L 26 24" 
              stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
        <path d="M 18 9 L 18 21 M 13 16 L 18 21 L 23 16" 
              stroke="url(#snapYtpGrad)" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
      </g>
      <g class="snapload-ytp-icon-success" style="display: none;">
        <path d="M 12 18 L 16 22 L 24 13" 
              stroke="#10B981" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
      </g>
      <g class="snapload-ytp-icon-error" style="display: none;">
        <path d="M 13 13 L 23 23 M 23 13 L 13 23" 
              stroke="#EF4444" stroke-width="2.5" stroke-linecap="round" fill="none"/>
      </g>
    </svg>
  `;

  // 2. Options 3-dots Button
  const optBtn = document.createElement("button");
  optBtn.id = OPTIONS_BUTTON_ID;
  optBtn.className = "ytp-button snapload-ytp-options-btn";
  optBtn.type = "button";
  optBtn.setAttribute("title", "Download Options");
  optBtn.setAttribute("aria-label", "Download Options");
  optBtn.setAttribute("aria-haspopup", "true");
  optBtn.setAttribute("aria-expanded", "false");
  optBtn.innerHTML = `
    <svg height="100%" version="1.1" viewBox="0 0 36 36" width="100%">
      <circle cx="18" cy="11" r="2.2" fill="#FFFFFF"/>
      <circle cx="18" cy="18" r="2.2" fill="#FFFFFF"/>
      <circle cx="18" cy="25" r="2.2" fill="#FFFFFF"/>
    </svg>
  `;

  const videoFmtUpper = currentPrefs.videoFormat.toUpperCase();
  const audioFmtUpper = currentPrefs.audioFormat.toUpperCase();
  const isVideoDefault = currentPrefs.mediaType === "video";
  const isAudioDefault = currentPrefs.mediaType === "audio";

  // 3. Dropdown Menu (English options)
  const menu = document.createElement("div");
  menu.id = DROPDOWN_MENU_ID;
  menu.className = "snapload-ytp-menu hidden";
  menu.innerHTML = `
    <div class="snapload-menu-header">Download Options</div>
    <div class="snapload-menu-item" data-action="download-video">
      <div class="snapload-menu-item-left">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
          <path d="M4 6.5C4 5.12 5.12 4 6.5 4h11C18.88 4 20 5.12 20 6.5v11c0 1.38-1.12 2.5-2.5 2.5h-11C5.12 20 4 18.88 4 17.5v-11zM10 8.5v7l6-3.5-6-3.5z"/>
        </svg>
        <span class="snapload-menu-title">Download ${videoFmtUpper} (Video)</span>
      </div>
      <span class="snapload-menu-badge">${videoFmtUpper}${isVideoDefault ? " • Default" : ""}</span>
    </div>
    <div class="snapload-menu-item" data-action="download-audio">
      <div class="snapload-menu-item-left">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
          <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/>
        </svg>
        <span class="snapload-menu-title">Download ${audioFmtUpper} (Audio)</span>
      </div>
      <span class="snapload-menu-badge audio">${audioFmtUpper}${isAudioDefault ? " • Default" : ""}</span>
    </div>
  `;

  const normalIcon = btn.querySelector(".snapload-ytp-icon-normal") as HTMLElement | null;
  const successIcon = btn.querySelector(".snapload-ytp-icon-success") as HTMLElement | null;
  const errorIcon = btn.querySelector(".snapload-ytp-icon-error") as HTMLElement | null;

  const closeMenu = () => {
    menu.classList.add("hidden");
    optBtn.setAttribute("aria-expanded", "false");
  };

  const openMenu = () => {
    updatePlayerLabels();
    menu.classList.remove("hidden");
    optBtn.setAttribute("aria-expanded", "true");
  };

  const triggerDownload = (type: "video" | "audio") => {
    closeMenu();
    btn.classList.add("downloading");

    const isAudio = type === "audio";
    const selectedFormat = isAudio ? (currentPrefs.audioFormat || "mp3") : (currentPrefs.videoFormat || "mp4");
    const selectedQuality = isAudio ? (currentPrefs.audioQuality || "best") : (currentPrefs.videoQuality || "best");

    btn.setAttribute("title", `Initiating ${selectedFormat.toUpperCase()} download...`);

    const videoUrl = getCanonicalVideoUrl();
    console.log(`[SnapLoad Player] Starting ${selectedFormat.toUpperCase()} (${type}) download for:`, videoUrl);

    const payload: any = {
      url: videoUrl,
      media_type: type,
    };
    if (isAudio) {
      payload.audio_format = selectedFormat;
      payload.audio_quality = selectedQuality;
    } else {
      payload.video_format = selectedFormat;
      payload.video_quality = selectedQuality;
    }

    api.runtime.sendMessage({
      type: "START_DOWNLOAD",
      payload,
    }, (response: any) => {
      try {
        if (response && response.success) {
          if (normalIcon) normalIcon.style.display = "none";
          if (successIcon) successIcon.style.display = "block";
          btn.setAttribute("title", `${selectedFormat.toUpperCase()} Download Started ✓ (Saved to Downloads)`);
        } else {
          if (normalIcon) normalIcon.style.display = "none";
          if (errorIcon) errorIcon.style.display = "block";
          btn.setAttribute("title", "SnapLoad Engine Offline - Run Setup");
        }

        setTimeout(() => {
          btn.classList.remove("downloading");
          if (normalIcon) normalIcon.style.display = "block";
          if (successIcon) successIcon.style.display = "none";
          if (errorIcon) errorIcon.style.display = "none";
          updatePlayerLabels();
        }, 3500);
      } catch (_) {}
    });
  };

  btn.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    triggerDownload(currentPrefs.mediaType);
  });

  optBtn.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (menu.classList.contains("hidden")) {
      openMenu();
    } else {
      closeMenu();
    }
  });

  const videoOption = menu.querySelector('[data-action="download-video"]');
  if (videoOption) {
    videoOption.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      triggerDownload("video");
    });
  }

  const audioOption = menu.querySelector('[data-action="download-audio"]');
  if (audioOption) {
    audioOption.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      triggerDownload("audio");
    });
  }

  container.appendChild(btn);
  container.appendChild(optBtn);
  container.appendChild(menu);

  return container;
}

function injectPlayerButton() {
  try {
    const leftControls = getLeftControls();
    if (!leftControls) {
      return;
    }

    const existing = document.getElementById(PLAYER_CONTAINER_ID);
    if (existing) {
      if (leftControls.contains(existing)) {
        return; // Already present in current controls, do not mutate DOM
      }
      existing.remove();
    }

    const container = createPlayerContainer();
    // In left-controls, append to the end (right after time display / chapters)
    leftControls.appendChild(container);

    console.log("[SnapLoad] Injected player control bar container in left controls.");
  } catch (err) {
    console.debug("[SnapLoad] Non-fatal error injecting player button:", err);
  }
}


// ==========================================
// 3. YouTube Shorts Button
// ==========================================
function createShortsButton(): HTMLElement {
  const container = document.createElement("div");
  container.id = SHORTS_BUTTON_ID;
  container.className = "snapload-shorts-container";
  container.innerHTML = `
    <button class="snapload-shorts-btn" title="Download Short with SnapLoad" type="button">
      <div class="snapload-shorts-icon">
        <svg viewBox="0 0 36 36" width="28" height="28" fill="none">
          <path d="M 10 24 L 10 26 C 10 26.5 10.5 27 11 27 L 25 27 C 25.5 27 26 26.5 26 26 L 26 24" 
                stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M 18 9 L 18 21 M 13 16 L 18 21 L 23 16" 
                stroke="#00F2FE" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
      <span class="snapload-shorts-label">SnapLoad</span>
    </button>
  `;

  const btn = container.querySelector(".snapload-shorts-btn") as HTMLButtonElement;
  btn.addEventListener("click", (e) => {
    try {
      e.preventDefault();
      e.stopPropagation();

      const videoUrl = getCanonicalVideoUrl();
      const label = container.querySelector(".snapload-shorts-label");
      if (label) label.textContent = "Sending...";

      api.runtime.sendMessage({
        type: "START_DOWNLOAD",
        payload: { url: videoUrl },
      }, (response: any) => {
        try {
          if (response && response.success) {
            if (label) label.textContent = "Started ✓";
          } else {
            if (label) label.textContent = "Offline";
          }
          setTimeout(() => {
            if (label) label.textContent = "SnapLoad";
          }, 3000);
        } catch (_) {}
      });
    } catch (_) {}
  });

  return container;
}

function injectShortsButton() {
  try {
    const isShorts = window.location.pathname.startsWith("/shorts");
    if (!isShorts) return;

    const activeReel =
      document.querySelector("ytd-reel-video-renderer[is-active] #actions") ||
      document.querySelector("ytd-reel-player-overlay-renderer[is-active] #actions") ||
      document.querySelector("ytd-reel-video-renderer[is-active] #actions-inner") ||
      document.querySelector(".ytd-reel-player-overlay-renderer #actions");

    if (!activeReel) {
      return;
    }

    const existing = document.getElementById(SHORTS_BUTTON_ID);
    if (existing) {
      if (activeReel.contains(existing)) {
        return;
      }
      existing.remove();
    }

    const container = createShortsButton();
    activeReel.insertBefore(container, activeReel.firstChild);
    console.log("[SnapLoad] Injected Shorts action button.");
  } catch (err) {
    console.debug("[SnapLoad] Non-fatal error injecting Shorts button:", err);
  }
}

// ==========================================
// Scoped Styles for All SnapLoad Buttons & Menus
// ==========================================
function injectGlobalStyles() {
  try {
    if (document.getElementById("snapload-injected-styles")) return;

    const style = document.createElement("style");
    style.id = "snapload-injected-styles";
    style.textContent = `
      /* Video Player Container */
      .snapload-ytp-container {
        display: inline-flex !important;
        align-items: center !important;
        position: relative !important;
        height: 100% !important;
        vertical-align: top !important;
        margin-left: 6px !important;
      }

      /* Player Download Button */
      .snapload-ytp-btn {
        display: inline-flex !important;
        align-items: center !important;
        justify-content: center !important;
        width: 38px !important;
        height: 100% !important;
        vertical-align: top !important;
        text-align: center !important;
        cursor: pointer !important;
        border: none !important;
        background: transparent !important;
        padding: 0 !important;
        outline: none !important;
        position: relative !important;
        opacity: 0.9;
        transition: opacity 0.2s ease, transform 0.15s ease;
      }
      .snapload-ytp-btn:hover {
        opacity: 1 !important;
        transform: scale(1.1);
      }
      .snapload-ytp-btn:active {
        transform: scale(0.92);
      }
      .snapload-ytp-btn.downloading {
        pointer-events: none;
      }

      /* 3-Dots Options Button */
      .snapload-ytp-options-btn {
        display: inline-flex !important;
        align-items: center !important;
        justify-content: center !important;
        width: 26px !important;
        height: 100% !important;
        vertical-align: top !important;
        cursor: pointer !important;
        border: none !important;
        background: transparent !important;
        padding: 0 !important;
        outline: none !important;
        position: relative !important;
        opacity: 0.8;
        transition: opacity 0.2s ease, transform 0.15s ease;
      }
      .snapload-ytp-options-btn:hover {
        opacity: 1 !important;
        transform: scale(1.15);
      }
      .snapload-ytp-options-btn:active {
        transform: scale(0.92);
      }

      /* Dropdown Menu Above Player Controls */
      .snapload-ytp-menu {
        position: absolute !important;
        bottom: calc(100% + 8px) !important;
        left: 0 !important;
        background: rgba(15, 23, 42, 0.97) !important;
        backdrop-filter: blur(16px) !important;
        -webkit-backdrop-filter: blur(16px) !important;
        border: 1px solid rgba(255, 255, 255, 0.15) !important;
        border-radius: 10px !important;
        padding: 6px !important;
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.75), 0 0 1px rgba(255, 255, 255, 0.2) !important;
        z-index: 999999 !important;
        display: flex !important;
        flex-direction: column !important;
        gap: 3px !important;
        min-width: 215px !important;
        user-select: none !important;
        animation: snaploadFadeIn 0.18s ease-out !important;
      }
      .snapload-ytp-menu.hidden {
        display: none !important;
      }
      @keyframes snaploadFadeIn {
        from { opacity: 0; transform: translateY(6px); }
        to { opacity: 1; transform: translateY(0); }
      }
      .snapload-menu-header {
        font-size: 10px !important;
        font-weight: 700 !important;
        color: #94A3B8 !important;
        text-transform: uppercase !important;
        letter-spacing: 0.6px !important;
        padding: 6px 10px 4px 10px !important;
        font-family: "Roboto", -apple-system, sans-serif !important;
        border-bottom: 1px solid rgba(255, 255, 255, 0.08) !important;
        margin-bottom: 3px !important;
      }
      .snapload-menu-item {
        display: flex !important;
        align-items: center !important;
        justify-content: space-between !important;
        gap: 8px !important;
        padding: 8px 10px !important;
        border-radius: 6px !important;
        cursor: pointer !important;
        color: #F8FAFC !important;
        font-size: 12px !important;
        font-weight: 500 !important;
        font-family: "Roboto", -apple-system, sans-serif !important;
        transition: background 0.15s ease, color 0.15s ease !important;
      }
      .snapload-menu-item:hover {
        background: rgba(255, 255, 255, 0.12) !important;
        color: #00F2FE !important;
      }
      .snapload-menu-item-left {
        display: flex !important;
        align-items: center !important;
        gap: 8px !important;
      }
      .snapload-menu-badge {
        font-size: 10px !important;
        font-weight: 700 !important;
        padding: 2px 6px !important;
        border-radius: 4px !important;
        background: rgba(0, 242, 254, 0.2) !important;
        color: #00F2FE !important;
      }
      .snapload-menu-badge.audio {
        background: rgba(168, 85, 247, 0.2) !important;
        color: #C084FC !important;
      }

      /* Action Bar Pill Button */
      #${ACTION_BUTTON_ID} {
        display: inline-flex;
        align-items: center;
        margin-left: 8px;
        vertical-align: middle;
        font-family: "Roboto", "Segoe UI", sans-serif;
      }
      .snapload-download-btn {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        height: 36px;
        padding: 0 14px;
        border: 1px solid rgba(99, 102, 241, 0.4);
        border-radius: 18px;
        background: linear-gradient(135deg, rgba(15, 23, 42, 0.95), rgba(30, 20, 59, 0.95));
        color: #F8FAFC;
        font-size: 14px;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s ease;
        box-shadow: 0 2px 8px rgba(0, 242, 254, 0.15);
        user-select: none;
      }
      .snapload-download-btn:hover {
        background: linear-gradient(135deg, rgba(30, 41, 59, 0.98), rgba(49, 36, 90, 0.98));
        border-color: #00F2FE;
        box-shadow: 0 0 12px rgba(0, 242, 254, 0.35);
        transform: translateY(-1px);
      }
      .snapload-download-btn:active {
        transform: scale(0.97);
      }
      .snapload-download-btn.downloading {
        border-color: #10B981;
        color: #10B981;
        pointer-events: none;
      }

      /* YouTube Shorts Button */
      .snapload-shorts-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-bottom: 12px;
      }
      .snapload-shorts-btn {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 4px;
        background: transparent;
        border: none;
        color: #FFFFFF;
        cursor: pointer;
        outline: none;
        font-size: 11px;
        font-weight: 500;
        transition: transform 0.2s;
      }
      .snapload-shorts-btn:hover {
        transform: scale(1.1);
      }
      .snapload-shorts-icon {
        width: 48px;
        height: 48px;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.1);
        display: flex;
        align-items: center;
        justify-content: center;
        backdrop-filter: blur(8px);
      }
      .snapload-shorts-label {
        text-shadow: 0 1px 3px rgba(0,0,0,0.8);
      }
    `;

    const targetHead = document.head || document.documentElement || document.body;
    if (targetHead) {
      targetHead.appendChild(style);
    }
  } catch (_) {}
}

// ==========================================
// Master Injection Controller
// ==========================================
function injectAll() {
  try {
    injectGlobalStyles();

    // Strictly remove any buttons next to subscribe button or metadata actions per user request
    const staleActionElements = document.querySelectorAll(
      `#${ACTION_BUTTON_ID}, .snapload-btn-container, .snapload-download-btn`
    );
    staleActionElements.forEach((el) => el.remove());

    const isWatch = window.location.pathname.startsWith("/watch");
    const isShorts = window.location.pathname.startsWith("/shorts");

    if (isWatch) {
      injectPlayerButton();
      const shortsBtn = document.getElementById(SHORTS_BUTTON_ID);
      if (shortsBtn) shortsBtn.remove();
    } else if (isShorts) {
      injectShortsButton();
      const playerContainer = document.getElementById(PLAYER_CONTAINER_ID);
      if (playerContainer) playerContainer.remove();
    } else {
      // Non-video page: clean up buttons
      const playerContainer = document.getElementById(PLAYER_CONTAINER_ID);
      if (playerContainer) playerContainer.remove();
      const shortsBtn = document.getElementById(SHORTS_BUTTON_ID);
      if (shortsBtn) shortsBtn.remove();
    }
  } catch (_) {}
}

function handleNavigation() {
  try {
    if (window.location.href !== lastUrl) {
      lastUrl = window.location.href;
    }
    injectAll();
    setTimeout(injectAll, 300);
    setTimeout(injectAll, 800);
    setTimeout(injectAll, 1800);
  } catch (_) {}
}

function initNavigationListener() {
  try {
    injectGlobalStyles();

    window.addEventListener("yt-navigate-finish", handleNavigation);
    window.addEventListener("yt-page-data-updated", handleNavigation);
    window.addEventListener("yt-player-updated", handleNavigation);
    window.addEventListener("spfdone", handleNavigation);

    const targetNode = document.body || document.documentElement;
    if (targetNode) {
      let lastRun = 0;
      let scheduled = false;
      const throttledInject = () => {
        const now = Date.now();
        if (now - lastRun >= 350) {
          lastRun = now;
          injectAll();
        } else if (!scheduled) {
          scheduled = true;
          setTimeout(() => {
            scheduled = false;
            lastRun = Date.now();
            injectAll();
          }, 350 - (now - lastRun));
        }
      };

      observer = new MutationObserver(() => {
        throttledInject();
      });

      observer.observe(targetNode, {
        childList: true,
        subtree: true,
      });
    }

    // Heartbeat periodic timer: ensures buttons re-appear if YouTube reconstructs elements
    setInterval(() => {
      try {
        const isWatch = window.location.pathname.startsWith("/watch");
        const isShorts = window.location.pathname.startsWith("/shorts");

        if (isWatch) {
          const leftControls = getLeftControls();
          const existing = document.getElementById(PLAYER_CONTAINER_ID);
          if (!leftControls || !existing || !leftControls.contains(existing)) {
            injectPlayerButton();
          }
          const staleActionElements = document.querySelectorAll(
            `#${ACTION_BUTTON_ID}, .snapload-btn-container, .snapload-download-btn`
          );
          staleActionElements.forEach((el) => el.remove());
        } else if (isShorts) {
          injectShortsButton();
        }
      } catch (_) {}
    }, 1500);

    handleNavigation();
  } catch (_) {}
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initNavigationListener);
} else {
  initNavigationListener();
}
