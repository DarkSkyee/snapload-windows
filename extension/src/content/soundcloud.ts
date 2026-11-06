import { api } from "../shared/browser-api";

const SOUNDCLOUD_PLAYER_BTN_ID = "snapload-sc-player-btn";
const SOUNDCLOUD_ACTION_BTN_ID = "snapload-sc-action-btn";
const SOUNDCLOUD_STYLES_ID = "snapload-sc-styles";

function injectStyles() {
  if (document.getElementById(SOUNDCLOUD_STYLES_ID)) return;
  const style = document.createElement("style");
  style.id = SOUNDCLOUD_STYLES_ID;
  style.textContent = `
    /* SoundCloud Player Bar SnapLoad Button */
    .snapload-sc-player-btn {
      display: inline-flex !important;
      align-items: center !important;
      justify-content: center !important;
      width: 26px !important;
      height: 26px !important;
      min-width: 26px !important;
      margin: 0 4px !important;
      background: rgba(25, 25, 25, 0.95) !important;
      border: 1.5px solid #FF5500 !important;
      border-radius: 4px !important;
      cursor: pointer !important;
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.5), 0 0 6px rgba(255, 85, 0, 0.3) !important;
      transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1) !important;
      padding: 0 !important;
      vertical-align: middle !important;
      box-sizing: border-box !important;
    }
    .snapload-sc-player-btn:hover {
      background: rgba(255, 85, 0, 0.25) !important;
      border-color: #FF7733 !important;
      transform: scale(1.1) !important;
      box-shadow: 0 3px 10px rgba(255, 85, 0, 0.6) !important;
    }
    .snapload-sc-player-btn:active {
      transform: scale(0.95) !important;
    }
    .snapload-sc-player-btn.downloading {
      border-color: #10B981 !important;
      pointer-events: none !important;
    }

    /* SoundCloud Action Button (below waveform) */
    .snapload-sc-action-btn {
      display: inline-flex !important;
      align-items: center !important;
      gap: 6px !important;
      height: 26px !important;
      padding: 0 10px !important;
      margin: 0 4px !important;
      background: rgba(25, 25, 25, 0.9) !important;
      border: 1px solid #FF5500 !important;
      border-radius: 3px !important;
      color: #FFFFFF !important;
      font-size: 11px !important;
      font-weight: 600 !important;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif !important;
      cursor: pointer !important;
      box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4) !important;
      transition: all 0.15s ease !important;
      user-select: none !important;
      vertical-align: middle !important;
    }
    .snapload-sc-action-btn:hover {
      background: rgba(255, 85, 0, 0.25) !important;
      border-color: #FF7733 !important;
      color: #FF7733 !important;
      transform: translateY(-1px) !important;
    }
    .snapload-sc-action-btn:active {
      transform: scale(0.97) !important;
    }
    .snapload-sc-action-btn.downloading {
      border-color: #10B981 !important;
      color: #10B981 !important;
      pointer-events: none !important;
    }

    /* SoundCloud Stream/Item Mini Button */
    .snapload-sc-item-btn {
      display: inline-flex !important;
      align-items: center !important;
      justify-content: center !important;
      width: 24px !important;
      height: 24px !important;
      min-width: 24px !important;
      margin: 0 2px !important;
      background: transparent !important;
      border: 1px solid rgba(255, 85, 0, 0.4) !important;
      border-radius: 3px !important;
      cursor: pointer !important;
      color: #999999 !important;
      transition: all 0.15s ease !important;
      padding: 0 !important;
    }
    .snapload-sc-item-btn:hover {
      border-color: #FF5500 !important;
      color: #FF5500 !important;
      background: rgba(255, 85, 0, 0.15) !important;
      transform: scale(1.1) !important;
    }
    .snapload-sc-item-btn.downloading {
      border-color: #10B981 !important;
      color: #10B981 !important;
      pointer-events: none !important;
    }
  `;
  const head = document.head || document.documentElement;
  if (head) head.appendChild(style);
}

function createDownloadIcon(color = "#FF5500", size = 16): string {
  return `
    <svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none">
      <path d="M12 3v10m0 0l-3.5-3.5m3.5 3.5l3.5-3.5" 
            stroke="${color}" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M4 17v1.5a2 2 0 002 2h12a2 2 0 002-2V17" 
            stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  `;
}

function createSuccessIcon(size = 16): string {
  return `
    <svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none">
      <path d="M5 13l4 4L19 7" stroke="#10B981" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  `;
}

/**
 * Returns current playing track URL on SoundCloud.
 */
function getCurrentSoundCloudPlayerUrl(): string {
  try {
    const titleLink = document.querySelector<HTMLAnchorElement>(".playbackSoundBadge__titleLink");
    if (titleLink && titleLink.href) {
      return titleLink.href;
    }
  } catch (_) {}
  return window.location.href;
}

/**
 * Injects SnapLoad button into SoundCloud bottom player controls.
 */
function injectSoundCloudPlayerButton() {
  try {
    const actionsContainer = document.querySelector(".playbackSoundBadge__actions");
    if (!actionsContainer) return;

    if (document.getElementById(SOUNDCLOUD_PLAYER_BTN_ID)) return;

    const btn = document.createElement("button");
    btn.id = SOUNDCLOUD_PLAYER_BTN_ID;
    btn.className = "snapload-sc-player-btn";
    btn.type = "button";
    btn.setAttribute("title", "Download playing track with SnapLoad");
    btn.setAttribute("aria-label", "Download playing track with SnapLoad");
    btn.innerHTML = createDownloadIcon("#FF5500", 16);

    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();

      const trackUrl = getCurrentSoundCloudPlayerUrl();
      console.log("[SnapLoad SoundCloud] Downloading playing track:", trackUrl);

      btn.classList.add("downloading");
      btn.innerHTML = `<span style="font-size: 10px; color: #FF5500;">...</span>`;

      api.runtime.sendMessage(
        {
          type: "START_DOWNLOAD",
          payload: { url: trackUrl, media_type: "audio" },
        },
        (resp: any) => {
          if (resp && resp.success) {
            btn.innerHTML = createSuccessIcon(16);
            btn.setAttribute("title", "Download Queued ✓");
          } else {
            btn.innerHTML = createDownloadIcon("#EF4444", 16);
            btn.setAttribute("title", "SnapLoad Engine Offline");
          }

          setTimeout(() => {
            btn.classList.remove("downloading");
            btn.innerHTML = createDownloadIcon("#FF5500", 16);
            btn.setAttribute("title", "Download playing track with SnapLoad");
          }, 3000);
        }
      );
    });

    actionsContainer.appendChild(btn);
  } catch (_) {}
}

/**
 * Injects SnapLoad action button below waveform on sound/playlist pages.
 */
function injectSoundCloudActionButton() {
  try {
    const btnGroup = document.querySelector(".soundActions .sc-button-group");
    if (!btnGroup) return;

    if (document.getElementById(SOUNDCLOUD_ACTION_BTN_ID)) return;

    const btn = document.createElement("button");
    btn.id = SOUNDCLOUD_ACTION_BTN_ID;
    btn.className = "snapload-sc-action-btn sc-button sc-button-medium";
    btn.type = "button";
    btn.setAttribute("title", "Download track with SnapLoad");
    btn.innerHTML = `
      ${createDownloadIcon("#FF5500", 14)}
      <span class="snapload-sc-label">SnapLoad</span>
    `;

    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();

      btn.classList.add("downloading");
      const labelSpan = btn.querySelector(".snapload-sc-label");

      if (window.location.pathname.includes("/sets/")) {
        const trackLinks = document.querySelectorAll<HTMLAnchorElement>(
          ".trackList__item .trackItem__trackTitle, .trackItem a.trackItem__trackTitle"
        );
        const urls: string[] = [];
        trackLinks.forEach((a) => {
          if (a.href && !urls.includes(a.href)) {
            urls.push(a.href);
          }
        });

        if (urls.length > 0) {
          if (labelSpan) labelSpan.textContent = `Queuing ${urls.length}...`;
          let count = 0;
          urls.forEach((u) => {
            api.runtime.sendMessage(
              { type: "START_DOWNLOAD", payload: { url: u, media_type: "audio" } },
              () => {
                count++;
                if (count >= urls.length) {
                  if (labelSpan) labelSpan.textContent = `Queued ${urls.length} ✓`;
                  setTimeout(() => {
                    btn.classList.remove("downloading");
                    if (labelSpan) labelSpan.textContent = "SnapLoad";
                  }, 3500);
                }
              }
            );
          });
          return;
        }
      }

      if (labelSpan) labelSpan.textContent = "Starting...";

      api.runtime.sendMessage(
        {
          type: "START_DOWNLOAD",
          payload: { url: window.location.href, media_type: "audio" },
        },
        (resp: any) => {
          if (resp && resp.success) {
            if (labelSpan) labelSpan.textContent = "Queued ✓";
          } else {
            if (labelSpan) labelSpan.textContent = "Offline";
          }
          setTimeout(() => {
            btn.classList.remove("downloading");
            if (labelSpan) labelSpan.textContent = "SnapLoad";
          }, 3000);
        }
      );
    });

    btnGroup.appendChild(btn);
  } catch (_) {}
}

/**
 * Injects mini download buttons on feed/stream track items.
 */
function injectSoundCloudItemButtons() {
  try {
    const groups = document.querySelectorAll(
      ".soundList__item .sound__actions .sc-button-group, .trackItem .trackItem__actions .sc-button-group"
    );

    groups.forEach((group) => {
      if (group.querySelector(".snapload-sc-item-btn")) return;

      const parentItem = group.closest(".soundList__item, .trackItem");
      if (!parentItem) return;

      const titleLink = parentItem.querySelector<HTMLAnchorElement>(
        ".soundTitle__title, .trackItem__trackTitle, a[href*='/']"
      );
      if (!titleLink || !titleLink.href) return;

      const itemBtn = document.createElement("button");
      itemBtn.className = "snapload-sc-item-btn sc-button";
      itemBtn.type = "button";
      itemBtn.setAttribute("title", "Download with SnapLoad");
      itemBtn.innerHTML = createDownloadIcon("#FF5500", 14);

      itemBtn.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();

        const trackUrl = titleLink.href;
        itemBtn.classList.add("downloading");
        itemBtn.innerHTML = createSuccessIcon(14);

        api.runtime.sendMessage(
          {
            type: "START_DOWNLOAD",
            payload: { url: trackUrl, media_type: "audio" },
          },
          (resp: any) => {
            if (resp && resp.success) {
              itemBtn.setAttribute("title", "Queued ✓");
            } else {
              itemBtn.innerHTML = createDownloadIcon("#EF4444", 14);
              itemBtn.setAttribute("title", "Error");
            }

            setTimeout(() => {
              itemBtn.classList.remove("downloading");
              itemBtn.innerHTML = createDownloadIcon("#FF5500", 14);
              itemBtn.setAttribute("title", "Download with SnapLoad");
            }, 3000);
          }
        );
      });

      group.appendChild(itemBtn);
    });
  } catch (_) {}
}

function injectAllSoundCloud() {
  injectStyles();
  injectSoundCloudPlayerButton();
  injectSoundCloudActionButton();
  injectSoundCloudItemButtons();
}

let lastSoundCloudUrl = "";
function initSoundCloudObserver() {
  injectStyles();
  injectAllSoundCloud();

  let scheduled = false;
  let lastRun = 0;
  const throttledInject = () => {
    const now = Date.now();
    if (now - lastRun >= 350) {
      lastRun = now;
      injectAllSoundCloud();
    } else if (!scheduled) {
      scheduled = true;
      setTimeout(() => {
        scheduled = false;
        lastRun = Date.now();
        injectAllSoundCloud();
      }, 350 - (now - lastRun));
    }
  };

  const observer = new MutationObserver(() => {
    if (window.location.href !== lastSoundCloudUrl) {
      lastSoundCloudUrl = window.location.href;
      const oldAction = document.getElementById(SOUNDCLOUD_ACTION_BTN_ID);
      if (oldAction) oldAction.remove();
    }
    throttledInject();
  });

  const targetNode = document.body || document.documentElement;
  if (targetNode) {
    observer.observe(targetNode, { childList: true, subtree: true });
  }

  setInterval(injectAllSoundCloud, 1500);
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initSoundCloudObserver);
} else {
  initSoundCloudObserver();
}
