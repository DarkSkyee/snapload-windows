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

/**
 * Locates YouTube active player container (movie_player or html5-video-player).
 */
function getPlayerContainer(): HTMLElement | null {
  try {
    const moviePlayer = document.getElementById("movie_player");
    if (moviePlayer) return moviePlayer;

    const player =
      document.querySelector<HTMLElement>(".html5-video-player:not(#inline-preview-player)") ||
      document.querySelector<HTMLElement>(".html5-video-player");
    if (player) return player;
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

function repositionDropdownMenu() {
  const menu = document.getElementById(DROPDOWN_MENU_ID);
  const optBtn = document.getElementById(OPTIONS_BUTTON_ID);
  if (!menu || menu.classList.contains("hidden") || !optBtn) return;
  const player = getPlayerContainer() || document.body;
  if (menu.parentElement !== player) {
    player.appendChild(menu);
  }
  const btnRect = optBtn.getBoundingClientRect();
  const playerRect = player.getBoundingClientRect();
  const menuWidth = menu.offsetWidth || 245;

  if (player === document.body) {
    menu.style.position = "fixed";
    menu.style.top = "auto";
    menu.style.right = "auto";
    const b = btnRect.top > 0 ? (window.innerHeight - btnRect.top + 8) : 60;
    const l = btnRect.left > 0 ? btnRect.left : 100;
    menu.style.bottom = `${Math.max(10, b)}px`;
    menu.style.left = `${Math.max(10, Math.min(window.innerWidth - menuWidth - 10, l))}px`;
  } else {
    try {
      if (window.getComputedStyle(player).position === "static") {
        player.style.position = "relative";
      }
    } catch (_) {}
    menu.style.position = "absolute";
    menu.style.top = "auto";
    menu.style.right = "auto";
    const b = (btnRect.top > 0 && playerRect.bottom >= btnRect.top)
      ? (playerRect.bottom - btnRect.top + 8)
      : 56;
    const l = (btnRect.left > 0 && btnRect.left >= playerRect.left)
      ? (btnRect.left - playerRect.left)
      : 100;
    menu.style.bottom = `${Math.max(10, b)}px`;
    menu.style.left = `${Math.max(8, Math.min(playerRect.width - menuWidth - 8, l))}px`;
  }
}

let isGlobalDismissBound = false;
function ensureGlobalDismiss() {
  if (isGlobalDismissBound) return;
  isGlobalDismissBound = true;

  // Use capture phase to ensure dismissal even if YouTube elements intercept or stop propagation
  document.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;
    const container = document.getElementById(PLAYER_CONTAINER_ID);
    const menu = document.getElementById(DROPDOWN_MENU_ID);
    const optBtn = document.getElementById(OPTIONS_BUTTON_ID);

    if (!menu || menu.classList.contains("hidden")) return;

    if (container?.contains(target) || menu.contains(target)) {
      return;
    }

    menu.classList.add("hidden");
    if (optBtn) optBtn.setAttribute("aria-expanded", "false");
  }, true);

  // Capture phase Escape handler: close menu and prevent exiting fullscreen mode
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      const menu = document.getElementById(DROPDOWN_MENU_ID);
      const optBtn = document.getElementById(OPTIONS_BUTTON_ID);
      if (menu && !menu.classList.contains("hidden")) {
        e.preventDefault();
        e.stopPropagation();
        menu.classList.add("hidden");
        if (optBtn) optBtn.setAttribute("aria-expanded", "false");
      }
    }
  }, true);

  window.addEventListener("resize", repositionDropdownMenu);
  document.addEventListener("fullscreenchange", () => {
    repositionDropdownMenu();
    setTimeout(repositionDropdownMenu, 100);
    setTimeout(repositionDropdownMenu, 300);
  });
}

function updatePlayerLabels() {
  const container = document.getElementById(PLAYER_CONTAINER_ID);
  const menu = document.getElementById(DROPDOWN_MENU_ID);

  const btn = (container ? container.querySelector(`#${PLAYER_BUTTON_ID}`) : document.getElementById(PLAYER_BUTTON_ID)) as HTMLButtonElement | null;
  const activeFmt = currentPrefs.mediaType === "audio" 
    ? currentPrefs.audioFormat.toUpperCase() 
    : currentPrefs.videoFormat.toUpperCase();
  if (btn && !btn.classList.contains("downloading")) {
    btn.setAttribute("title", `Download with SnapLoad (${activeFmt})`);
  }

  if (menu) {
    const videoItem = menu.querySelector('[data-action="download-video"]');
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

    const audioItem = menu.querySelector('[data-action="download-audio"]');
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
}

// ==========================================
// 2. YouTube Player Control Bar Button & 3-Dots Options Menu
// ==========================================
function createPlayerContainer(): HTMLElement {
  ensureGlobalDismiss();
  loadStoredPreferences();

  // Remove any previous menu attached to player or body
  const oldMenu = document.getElementById(DROPDOWN_MENU_ID);
  if (oldMenu) oldMenu.remove();

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
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
      <g class="snapload-ytp-icon-normal">
        <path d="M12 3v10m0 0l-3.5-3.5m3.5 3.5l3.5-3.5" 
              stroke="#00F2FE" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M4 17v1.5a2 2 0 002 2h12a2 2 0 002-2V17" 
              stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
      </g>
      <g class="snapload-ytp-icon-success" style="display: none;">
        <path d="M5 13l4 4L19 7" 
              stroke="#10B981" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>
      </g>
      <g class="snapload-ytp-icon-error" style="display: none;">
        <path d="M6 18L18 6M6 6l12 12" 
              stroke="#EF4444" stroke-width="2.6" stroke-linecap="round"/>
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
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
      <circle cx="12" cy="5" r="2.5" fill="#00F2FE"/>
      <circle cx="12" cy="12" r="2.5" fill="#00F2FE"/>
      <circle cx="12" cy="19" r="2.5" fill="#00F2FE"/>
    </svg>
  `;

  const videoFmtUpper = currentPrefs.videoFormat.toUpperCase();
  const audioFmtUpper = currentPrefs.audioFormat.toUpperCase();
  const isVideoDefault = currentPrefs.mediaType === "video";
  const isAudioDefault = currentPrefs.mediaType === "audio";

  // 3. Dropdown Menu (appended outside left controls to avoid overflow clipping)
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

  // Stop events on menu to prevent accidental YouTube play/pause or fullscreen toggles
  const stopMenuEvent = (e: Event) => {
    e.stopPropagation();
  };
  menu.addEventListener("click", stopMenuEvent);
  menu.addEventListener("mousedown", stopMenuEvent);
  menu.addEventListener("pointerdown", stopMenuEvent);
  menu.addEventListener("dblclick", stopMenuEvent);
  menu.addEventListener("contextmenu", stopMenuEvent);

  const normalIcon = btn.querySelector(".snapload-ytp-icon-normal") as HTMLElement | null;
  const successIcon = btn.querySelector(".snapload-ytp-icon-success") as HTMLElement | null;
  const errorIcon = btn.querySelector(".snapload-ytp-icon-error") as HTMLElement | null;

  const closeMenu = () => {
    menu.classList.add("hidden");
    optBtn.setAttribute("aria-expanded", "false");
  };

  const openMenu = () => {
    updatePlayerLabels();
    repositionDropdownMenu();
    menu.classList.remove("hidden");
    optBtn.setAttribute("aria-expanded", "true");
    // Ensure position is accurate after display change
    requestAnimationFrame(repositionDropdownMenu);
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

  // Initial append to player or body so it is ready and not clipped by controls
  const player = getPlayerContainer() || document.body;
  player.appendChild(menu);

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
      const existingMenu = document.getElementById(DROPDOWN_MENU_ID);
      if (existingMenu) existingMenu.remove();
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
        gap: 6px !important;
        height: 100% !important;
        vertical-align: middle !important;
        margin: 0 8px !important;
        padding: 0 !important;
        box-sizing: border-box !important;
        position: relative !important;
      }

      /* Base styles for both SnapLoad player buttons */
      .snapload-ytp-btn,
      .snapload-ytp-options-btn {
        box-sizing: border-box !important;
        display: inline-flex !important;
        align-items: center !important;
        justify-content: center !important;
        height: 32px !important;
        vertical-align: middle !important;
        text-align: center !important;
        cursor: pointer !important;
        padding: 0 !important;
        margin: 0 !important;
        background: rgba(15, 23, 42, 0.92) !important;
        border: 1.5px solid #00F2FE !important;
        outline: 1.5px solid rgba(0, 242, 254, 0.35) !important;
        outline-offset: 1px !important;
        border-radius: 8px !important;
        box-shadow: 0 2px 10px rgba(0, 0, 0, 0.6), 0 0 8px rgba(0, 242, 254, 0.3) !important;
        opacity: 1 !important;
        transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1) !important;
        user-select: none !important;
        backdrop-filter: blur(8px) !important;
        -webkit-backdrop-filter: blur(8px) !important;
      }

      /* Player Download Button */
      .snapload-ytp-btn {
        width: 36px !important;
        min-width: 36px !important;
      }

      /* 3-Dots Options Button */
      .snapload-ytp-options-btn {
        width: 32px !important;
        min-width: 32px !important;
      }

      .snapload-ytp-btn:hover,
      .snapload-ytp-options-btn:hover {
        background: rgba(30, 41, 59, 0.98) !important;
        border-color: #38BDF8 !important;
        outline-color: rgba(56, 189, 248, 0.7) !important;
        box-shadow: 0 4px 14px rgba(0, 0, 0, 0.7), 0 0 14px rgba(0, 242, 254, 0.6) !important;
        transform: translateY(-1px) scale(1.05) !important;
      }

      .snapload-ytp-options-btn[aria-expanded="true"] {
        background: rgba(30, 41, 59, 0.98) !important;
        border-color: #38BDF8 !important;
        outline-color: rgba(56, 189, 248, 0.8) !important;
        box-shadow: 0 4px 14px rgba(0, 0, 0, 0.7), 0 0 14px rgba(0, 242, 254, 0.6) !important;
      }

      .snapload-ytp-btn:active,
      .snapload-ytp-options-btn:active {
        transform: scale(0.95) !important;
      }

      .snapload-ytp-btn.downloading {
        border-color: #10B981 !important;
        outline-color: rgba(16, 185, 129, 0.5) !important;
        box-shadow: 0 0 12px rgba(16, 185, 129, 0.5) !important;
        pointer-events: none !important;
      }

      /* Dropdown Menu Above Player Controls */
      .snapload-ytp-menu {
        position: absolute !important;
        background: rgba(15, 23, 42, 0.98) !important;
        backdrop-filter: blur(16px) !important;
        -webkit-backdrop-filter: blur(16px) !important;
        border: 1.5px solid #00F2FE !important;
        outline: 1.5px solid rgba(0, 242, 254, 0.35) !important;
        outline-offset: 1px !important;
        border-radius: 12px !important;
        padding: 8px !important;
        box-shadow: 0 12px 36px rgba(0, 0, 0, 0.85), 0 0 16px rgba(0, 242, 254, 0.25) !important;
        z-index: 2147483647 !important;
        display: flex !important;
        flex-direction: column !important;
        gap: 4px !important;
        min-width: 235px !important;
        user-select: none !important;
        pointer-events: auto !important;
        box-sizing: border-box !important;
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
        font-size: 11px !important;
        font-weight: 700 !important;
        color: #94A3B8 !important;
        text-transform: uppercase !important;
        letter-spacing: 0.8px !important;
        padding: 6px 10px 6px 10px !important;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif !important;
        border-bottom: 1px solid rgba(255, 255, 255, 0.1) !important;
        margin-bottom: 4px !important;
        line-height: normal !important;
      }
      .snapload-menu-item {
        display: flex !important;
        align-items: center !important;
        justify-content: space-between !important;
        gap: 10px !important;
        padding: 9px 12px !important;
        border-radius: 8px !important;
        cursor: pointer !important;
        color: #F8FAFC !important;
        font-size: 13px !important;
        font-weight: 500 !important;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif !important;
        line-height: normal !important;
        transition: background 0.15s ease, color 0.15s ease, transform 0.1s ease !important;
      }
      .snapload-menu-item:hover {
        background: rgba(0, 242, 254, 0.15) !important;
        color: #00F2FE !important;
        transform: translateX(2px) !important;
      }
      .snapload-menu-item:active {
        transform: scale(0.98) !important;
      }
      .snapload-menu-item-left {
        display: flex !important;
        align-items: center !important;
        gap: 8px !important;
      }
      .snapload-menu-badge {
        font-size: 10px !important;
        font-weight: 700 !important;
        padding: 2px 7px !important;
        border-radius: 5px !important;
        background: rgba(0, 242, 254, 0.2) !important;
        color: #00F2FE !important;
        border: 1px solid rgba(0, 242, 254, 0.4) !important;
        letter-spacing: 0.3px !important;
      }
      .snapload-menu-badge.audio {
        background: rgba(168, 85, 247, 0.2) !important;
        color: #C084FC !important;
        border: 1px solid rgba(168, 85, 247, 0.4) !important;
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
      const menuEl = document.getElementById(DROPDOWN_MENU_ID);
      if (menuEl) menuEl.remove();
    } else {
      // Non-video page: clean up buttons
      const playerContainer = document.getElementById(PLAYER_CONTAINER_ID);
      if (playerContainer) playerContainer.remove();
      const shortsBtn = document.getElementById(SHORTS_BUTTON_ID);
      if (shortsBtn) shortsBtn.remove();
      const menuEl = document.getElementById(DROPDOWN_MENU_ID);
      if (menuEl) menuEl.remove();
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
