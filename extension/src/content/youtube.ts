import { api } from "../shared/browser-api";

const PLAYER_BUTTON_ID = "snapload-ytp-btn";
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
 * Locates YouTube active player right-side controls container, filtering out inline preview player.
 */
function getRightControls(): Element | null {
  try {
    const moviePlayer = document.getElementById("movie_player");
    if (moviePlayer) {
      const right = moviePlayer.querySelector(".ytp-right-controls");
      if (right) return right;
    }

    const player =
      document.querySelector(".html5-video-player:not(#inline-preview-player)") ||
      document.querySelector(".html5-video-player");

    if (player) {
      const right = player.querySelector(".ytp-right-controls");
      if (right) return right;
    }

    const all = document.querySelectorAll(".ytp-right-controls");
    for (const el of all) {
      if (!el.closest("#inline-preview-player")) {
        return el;
      }
    }
  } catch (_) {}
  return null;
}

// ==========================================
// 1. YouTube Player Control Bar Button
// ==========================================
function createPlayerButton(): HTMLElement {
  const btn = document.createElement("button");
  btn.id = PLAYER_BUTTON_ID;
  btn.className = "ytp-button snapload-ytp-btn";
  btn.type = "button";
  btn.setAttribute("title", "Download with SnapLoad");
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

  btn.addEventListener("click", (e) => {
    try {
      e.preventDefault();
      e.stopPropagation();

      const normalIcon = btn.querySelector(".snapload-ytp-icon-normal") as HTMLElement | null;
      const successIcon = btn.querySelector(".snapload-ytp-icon-success") as HTMLElement | null;
      const errorIcon = btn.querySelector(".snapload-ytp-icon-error") as HTMLElement | null;

      btn.classList.add("downloading");
      btn.setAttribute("title", "Initiating SnapLoad...");

      const videoUrl = getCanonicalVideoUrl();
      console.log("[SnapLoad Player] Starting download for:", videoUrl);

      api.runtime.sendMessage({
        type: "START_DOWNLOAD",
        payload: { url: videoUrl },
      }, (response: any) => {
        try {
          if (response && response.success) {
            if (normalIcon) normalIcon.style.display = "none";
            if (successIcon) successIcon.style.display = "block";
            btn.setAttribute("title", "Download Started ✓ (Saved to Downloads)");
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
            btn.setAttribute("title", "Download with SnapLoad");
          }, 3500);
        } catch (_) {}
      });
    } catch (_) {}
  });

  return btn;
}

function injectPlayerButton() {
  try {
    const rightControls = getRightControls();
    if (!rightControls) {
      return;
    }

    const existing = document.getElementById(PLAYER_BUTTON_ID);
    if (existing) {
      if (rightControls.contains(existing)) {
        return; // Already present in current controls, do not mutate DOM
      }
      existing.remove();
    }

    const btn = createPlayerButton();

    // Position before settings button or subtitles button if child of rightControls
    const settingsBtn = rightControls.querySelector(".ytp-settings-button");
    const subBtn = rightControls.querySelector(".ytp-subtitles-button");
    const target = (settingsBtn && settingsBtn.parentNode === rightControls)
      ? settingsBtn
      : (subBtn && subBtn.parentNode === rightControls)
        ? subBtn
        : null;

    if (target) {
      rightControls.insertBefore(btn, target);
    } else {
      rightControls.appendChild(btn);
    }

    console.log("[SnapLoad] Injected player control bar button.");
  } catch (err) {
    console.debug("[SnapLoad] Non-fatal error injecting player button:", err);
  }
}

// ==========================================
// 2. Action Bar Pill Button (Below video title)
// ==========================================
function createActionButton(): HTMLElement {
  const container = document.createElement("div");
  container.id = ACTION_BUTTON_ID;
  container.className = "snapload-btn-container";
  container.setAttribute("role", "button");
  container.setAttribute("tabindex", "0");
  container.setAttribute("aria-label", "Download with SnapLoad");

  container.innerHTML = `
    <button class="snapload-download-btn" type="button" title="Download with SnapLoad">
      <div class="snapload-btn-icon">
        <svg viewBox="0 0 108 108" width="18" height="18" fill="none">
          <defs>
            <linearGradient id="snaploadActionGrad" x1="0" y1="0" x2="108" y2="108" gradientUnits="userSpaceOnUse">
              <stop offset="0%" stop-color="#00F2FE"/>
              <stop offset="50%" stop-color="#6366F1"/>
              <stop offset="100%" stop-color="#A855F7"/>
            </linearGradient>
          </defs>
          <path d="M66,32 C58,28 44,29 38,36 C34,41 35,49 42,53 L58,60 C64,63 66,67 64,72 C61,77 53,79 46,77 C41,75 37,71 36,67"
                stroke="url(#snaploadActionGrad)" stroke-width="8" stroke-linecap="round" fill="none"/>
          <path d="M54,40 L54,66 M44,56 L54,66 L64,56" stroke="#FFFFFF" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
      <span class="snapload-btn-label">SnapLoad</span>
    </button>
  `;

  const btn = container.querySelector(".snapload-download-btn") as HTMLButtonElement;
  btn.addEventListener("click", (e) => {
    try {
      e.preventDefault();
      e.stopPropagation();

      const videoUrl = getCanonicalVideoUrl();
      console.log("[SnapLoad Action] Triggering download for:", videoUrl);

      btn.classList.add("downloading");
      const label = btn.querySelector(".snapload-btn-label");
      if (label) label.textContent = "Starting...";

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
            btn.classList.remove("downloading");
            if (label) label.textContent = "SnapLoad";
          }, 3000);
        } catch (_) {}
      });
    } catch (_) {}
  });

  return container;
}

function injectActionButton() {
  try {
    const isWatch = window.location.pathname.startsWith("/watch");
    if (!isWatch) return;

    // Isolate SnapLoad action pill outside ytd-menu-renderer Polymer tree
    // to prevent calculateSize crashes and preserve SponsorBlock/native buttons
    const actionsContainer =
      document.querySelector("ytd-watch-metadata #actions") ||
      document.querySelector("#actions");

    if (!actionsContainer) return;

    const existing = document.getElementById(ACTION_BUTTON_ID);
    if (existing) {
      if (actionsContainer.contains(existing)) {
        return; // Already present in container
      }
      existing.remove();
    }

    const button = createActionButton();
    // Insert before actions-inner (ytd-menu-renderer) so it is an external sibling
    const innerMenu = actionsContainer.querySelector("#actions-inner") || actionsContainer.firstElementChild;
    if (innerMenu) {
      actionsContainer.insertBefore(button, innerMenu);
    } else {
      actionsContainer.appendChild(button);
    }

    console.log("[SnapLoad] Injected action bar button.");
  } catch (err) {
    console.debug("[SnapLoad] Non-fatal error injecting action button:", err);
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
// Scoped Styles for All SnapLoad Buttons
// ==========================================
function injectGlobalStyles() {
  try {
    if (document.getElementById("snapload-injected-styles")) return;

    const style = document.createElement("style");
    style.id = "snapload-injected-styles";
    style.textContent = `
      /* SponsorBlock-style Player Control Button */
      .snapload-ytp-btn {
        display: inline-block !important;
        width: 46px !important;
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
        transform: scale(1.12);
      }
      .snapload-ytp-btn:active {
        transform: scale(0.92);
      }
      .snapload-ytp-btn.downloading {
        pointer-events: none;
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

    const isWatch = window.location.pathname.startsWith("/watch");
    const isShorts = window.location.pathname.startsWith("/shorts");

    if (isWatch) {
      injectPlayerButton();
      injectActionButton();
      const shortsBtn = document.getElementById(SHORTS_BUTTON_ID);
      if (shortsBtn) shortsBtn.remove();
    } else if (isShorts) {
      injectShortsButton();
      const playerBtn = document.getElementById(PLAYER_BUTTON_ID);
      if (playerBtn) playerBtn.remove();
      const actionBtn = document.getElementById(ACTION_BUTTON_ID);
      if (actionBtn) actionBtn.remove();
    } else {
      // Non-video page: clean up buttons
      const playerBtn = document.getElementById(PLAYER_BUTTON_ID);
      if (playerBtn) playerBtn.remove();
      const actionBtn = document.getElementById(ACTION_BUTTON_ID);
      if (actionBtn) actionBtn.remove();
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
          const rightControls = getRightControls();
          const existing = document.getElementById(PLAYER_BUTTON_ID);
          if (!rightControls || !existing || !rightControls.contains(existing)) {
            injectPlayerButton();
          }
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
