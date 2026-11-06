import { api } from "../shared/browser-api";

const SPOTIFY_PLAYER_BTN_ID = "snapload-sp-player-btn";
const SPOTIFY_ACTION_BTN_ID = "snapload-sp-action-btn";
const SPOTIFY_STYLES_ID = "snapload-sp-styles";

function injectStyles() {
  if (document.getElementById(SPOTIFY_STYLES_ID)) return;
  const style = document.createElement("style");
  style.id = SPOTIFY_STYLES_ID;
  style.textContent = `
    /* SnapLoad Player Bar Button */
    .snapload-sp-player-btn {
      display: inline-flex !important;
      align-items: center !important;
      justify-content: center !important;
      width: 32px !important;
      height: 32px !important;
      min-width: 32px !important;
      margin: 0 6px !important;
      background: rgba(15, 23, 42, 0.9) !important;
      border: 1.5px solid #1DB954 !important;
      border-radius: 50% !important;
      cursor: pointer !important;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5), 0 0 8px rgba(29, 185, 84, 0.3) !important;
      transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1) !important;
      padding: 0 !important;
      vertical-align: middle !important;
      box-sizing: border-box !important;
    }
    .snapload-sp-player-btn:hover {
      background: rgba(29, 185, 84, 0.2) !important;
      transform: scale(1.1) !important;
      box-shadow: 0 4px 12px rgba(29, 185, 84, 0.6) !important;
    }
    .snapload-sp-player-btn:active {
      transform: scale(0.95) !important;
    }
    .snapload-sp-player-btn.downloading {
      border-color: #10B981 !important;
      pointer-events: none !important;
    }

    /* SnapLoad Action Bar Button */
    .snapload-sp-action-btn {
      display: inline-flex !important;
      align-items: center !important;
      gap: 8px !important;
      height: 42px !important;
      padding: 0 18px !important;
      margin-left: 12px !important;
      background: linear-gradient(135deg, rgba(15, 23, 42, 0.95), rgba(20, 40, 25, 0.95)) !important;
      border: 1.5px solid #1DB954 !important;
      border-radius: 24px !important;
      color: #FFFFFF !important;
      font-size: 14px !important;
      font-weight: 700 !important;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif !important;
      cursor: pointer !important;
      box-shadow: 0 4px 14px rgba(0, 0, 0, 0.6), 0 0 10px rgba(29, 185, 84, 0.3) !important;
      transition: all 0.2s ease !important;
      user-select: none !important;
      vertical-align: middle !important;
    }
    .snapload-sp-action-btn:hover {
      background: linear-gradient(135deg, rgba(29, 185, 84, 0.25), rgba(15, 23, 42, 0.98)) !important;
      transform: translateY(-1px) scale(1.03) !important;
      box-shadow: 0 6px 18px rgba(29, 185, 84, 0.5) !important;
    }
    .snapload-sp-action-btn:active {
      transform: scale(0.97) !important;
    }
    .snapload-sp-action-btn.downloading {
      border-color: #10B981 !important;
      color: #10B981 !important;
      pointer-events: none !important;
    }

    /* SnapLoad Row Button */
    .snapload-sp-row-btn {
      display: inline-flex !important;
      align-items: center !important;
      justify-content: center !important;
      width: 28px !important;
      height: 28px !important;
      min-width: 28px !important;
      margin: 0 4px !important;
      background: transparent !important;
      border: none !important;
      border-radius: 50% !important;
      cursor: pointer !important;
      color: #B3B3B3 !important;
      opacity: 0 !important;
      transition: opacity 0.15s ease, transform 0.15s ease, color 0.15s ease !important;
      padding: 0 !important;
    }
    [data-testid="tracklist-row"]:hover .snapload-sp-row-btn,
    .snapload-sp-row-btn:focus,
    .snapload-sp-row-btn.downloading {
      opacity: 1 !important;
    }
    .snapload-sp-row-btn:hover {
      color: #1DB954 !important;
      transform: scale(1.15) !important;
    }
    .snapload-sp-row-btn:active {
      transform: scale(0.95) !important;
    }
    .snapload-sp-row-btn.downloading {
      color: #10B981 !important;
      pointer-events: none !important;
    }
  `;
  const head = document.head || document.documentElement;
  if (head) head.appendChild(style);
}

/**
 * Returns current playing track URL dynamically from DOM (handling Spotify SPA track changes).
 */
function getCurrentPlayingTrackUrl(): string {
  try {
    const nowPlayingWidget = document.querySelector('[data-testid="now-playing-widget"]');
    if (nowPlayingWidget) {
      const trackLink =
        nowPlayingWidget.querySelector<HTMLAnchorElement>('a[data-testid="context-item-link"]') ||
        nowPlayingWidget.querySelector<HTMLAnchorElement>('a[href*="/track/"]');
      if (trackLink && trackLink.href) {
        return trackLink.href;
      }
    }
  } catch (_) {}
  return window.location.href;
}

function createDownloadIcon(color = "#1DB954", size = 18): string {
  return `
    <svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none">
      <path d="M12 3v10m0 0l-3.5-3.5m3.5 3.5l3.5-3.5" 
            stroke="${color}" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M4 17v1.5a2 2 0 002 2h12a2 2 0 002-2V17" 
            stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  `;
}

function createSuccessIcon(size = 18): string {
  return `
    <svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none">
      <path d="M5 13l4 4L19 7" stroke="#10B981" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  `;
}

/**
 * Injects SnapLoad download button into Spotify Now Playing bottom bar.
 */
function injectPlayerBarButton() {
  try {
    const nowPlayingWidget = document.querySelector('[data-testid="now-playing-widget"]');
    if (!nowPlayingWidget) return;

    if (document.getElementById(SPOTIFY_PLAYER_BTN_ID)) return;

    const btn = document.createElement("button");
    btn.id = SPOTIFY_PLAYER_BTN_ID;
    btn.className = "snapload-sp-player-btn";
    btn.type = "button";
    btn.setAttribute("title", "Download playing track with SnapLoad");
    btn.setAttribute("aria-label", "Download playing track with SnapLoad");
    btn.innerHTML = createDownloadIcon("#1DB954", 18);

    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();

      const trackUrl = getCurrentPlayingTrackUrl();
      console.log("[SnapLoad Spotify] Downloading current track:", trackUrl);

      btn.classList.add("downloading");
      btn.innerHTML = `<span style="font-size: 11px; color: #1DB954;">...</span>`;

      api.runtime.sendMessage(
        {
          type: "START_DOWNLOAD",
          payload: { url: trackUrl, media_type: "audio" },
        },
        (resp: any) => {
          if (resp && resp.success) {
            btn.innerHTML = createSuccessIcon(18);
            btn.setAttribute("title", "Download Queued ✓");
          } else {
            btn.innerHTML = createDownloadIcon("#EF4444", 18);
            btn.setAttribute("title", "SnapLoad Engine Offline");
          }

          setTimeout(() => {
            btn.classList.remove("downloading");
            btn.innerHTML = createDownloadIcon("#1DB954", 18);
            btn.setAttribute("title", "Download playing track with SnapLoad");
          }, 3000);
        }
      );
    });

    // Place next to heart/library button or at the end of the widget
    const likeBtn =
      nowPlayingWidget.querySelector('[data-testid="add-button"]') ||
      nowPlayingWidget.querySelector('[data-testid="control-button-heart"]');
    if (likeBtn && likeBtn.parentNode) {
      likeBtn.parentNode.insertBefore(btn, likeBtn.nextSibling);
    } else {
      nowPlayingWidget.appendChild(btn);
    }
  } catch (_) {}
}

/**
 * Injects SnapLoad button into Playlist / Album / Track action bar row.
 */
function injectActionBarButton() {
  try {
    const actionBar = document.querySelector('[data-testid="action-bar-row"]');
    if (!actionBar) return;

    if (document.getElementById(SPOTIFY_ACTION_BTN_ID)) return;

    const isPlaylist = window.location.pathname.startsWith("/playlist");
    const isAlbum = window.location.pathname.startsWith("/album");
    const isTrack = window.location.pathname.startsWith("/track");

    if (!isPlaylist && !isAlbum && !isTrack) return;

    const label = isPlaylist ? "Download Playlist" : (isAlbum ? "Download Album" : "Download Audio");

    const btn = document.createElement("button");
    btn.id = SPOTIFY_ACTION_BTN_ID;
    btn.className = "snapload-sp-action-btn";
    btn.type = "button";
    btn.setAttribute("title", `${label} with SnapLoad`);
    btn.innerHTML = `
      ${createDownloadIcon("#1DB954", 20)}
      <span class="snapload-sp-label">${label}</span>
    `;

    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();

      btn.classList.add("downloading");
      const labelSpan = btn.querySelector(".snapload-sp-label");

      if (isPlaylist || isAlbum) {
        // Scrape track links from DOM rows
        const rows = document.querySelectorAll('[data-testid="tracklist-row"]');
        const trackUrls: string[] = [];

        rows.forEach((row) => {
          const a = row.querySelector<HTMLAnchorElement>('a[href*="/track/"]');
          if (a && a.href && !trackUrls.includes(a.href)) {
            trackUrls.push(a.href);
          }
        });

        if (trackUrls.length > 0) {
          if (labelSpan) labelSpan.textContent = `Queuing ${trackUrls.length} tracks...`;
          let queuedCount = 0;

          trackUrls.forEach((tUrl) => {
            api.runtime.sendMessage({
              type: "START_DOWNLOAD",
              payload: { url: tUrl, media_type: "audio" },
            }, () => {
              queuedCount++;
              if (queuedCount >= trackUrls.length) {
                if (labelSpan) labelSpan.textContent = `Queued ${trackUrls.length} tracks ✓`;
                setTimeout(() => {
                  btn.classList.remove("downloading");
                  if (labelSpan) labelSpan.textContent = label;
                }, 3500);
              }
            });
          });
          return;
        }
      }

      // Fallback for single track page or empty DOM rows: send page URL
      if (labelSpan) labelSpan.textContent = "Starting...";
      api.runtime.sendMessage({
        type: "START_DOWNLOAD",
        payload: { url: window.location.href, media_type: "audio" },
      }, (resp: any) => {
        if (resp && resp.success) {
          if (labelSpan) labelSpan.textContent = "Queued ✓";
        } else {
          if (labelSpan) labelSpan.textContent = "Offline";
        }
        setTimeout(() => {
          btn.classList.remove("downloading");
          if (labelSpan) labelSpan.textContent = label;
        }, 3000);
      });
    });

    actionBar.appendChild(btn);
  } catch (_) {}
}

/**
 * Injects mini download button on hover inside tracklist rows.
 */
function injectTracklistRowButtons() {
  try {
    const rows = document.querySelectorAll('[data-testid="tracklist-row"]');
    rows.forEach((row) => {
      if (row.querySelector(".snapload-sp-row-btn")) return;

      const trackLink = row.querySelector<HTMLAnchorElement>('a[href*="/track/"]');
      if (!trackLink || !trackLink.href) return;

      const rowBtn = document.createElement("button");
      rowBtn.className = "snapload-sp-row-btn";
      rowBtn.type = "button";
      rowBtn.setAttribute("title", "Download track with SnapLoad");
      rowBtn.innerHTML = createDownloadIcon("#B3B3B3", 16);

      rowBtn.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();

        const url = trackLink.href;
        rowBtn.classList.add("downloading");
        rowBtn.innerHTML = createSuccessIcon(16);

        api.runtime.sendMessage({
          type: "START_DOWNLOAD",
          payload: { url, media_type: "audio" },
        }, (resp: any) => {
          if (resp && resp.success) {
            rowBtn.setAttribute("title", "Queued ✓");
          } else {
            rowBtn.innerHTML = createDownloadIcon("#EF4444", 16);
            rowBtn.setAttribute("title", "Error");
          }
          setTimeout(() => {
            rowBtn.classList.remove("downloading");
            rowBtn.innerHTML = createDownloadIcon("#B3B3B3", 16);
            rowBtn.setAttribute("title", "Download track with SnapLoad");
          }, 3000);
        });
      });

      // Target row action column (often last column with duration and more-options)
      const lastCell = row.lastElementChild;
      if (lastCell) {
        lastCell.insertBefore(rowBtn, lastCell.firstChild);
      } else {
        row.appendChild(rowBtn);
      }
    });
  } catch (_) {}
}

function injectAllSpotify() {
  injectStyles();
  injectPlayerBarButton();
  injectActionBarButton();
  injectTracklistRowButtons();
}

let lastSpotifyUrl = "";
function initSpotifyObserver() {
  injectStyles();
  injectAllSpotify();

  let scheduled = false;
  let lastRun = 0;
  const throttledInject = () => {
    const now = Date.now();
    if (now - lastRun >= 350) {
      lastRun = now;
      injectAllSpotify();
    } else if (!scheduled) {
      scheduled = true;
      setTimeout(() => {
        scheduled = false;
        lastRun = Date.now();
        injectAllSpotify();
      }, 350 - (now - lastRun));
    }
  };

  const observer = new MutationObserver(() => {
    if (window.location.href !== lastSpotifyUrl) {
      lastSpotifyUrl = window.location.href;
      const oldAction = document.getElementById(SPOTIFY_ACTION_BTN_ID);
      if (oldAction) oldAction.remove();
    }
    throttledInject();
  });

  const targetNode = document.body || document.documentElement;
  if (targetNode) {
    observer.observe(targetNode, { childList: true, subtree: true });
  }

  // Periodic heartbeat
  setInterval(injectAllSpotify, 1500);
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initSpotifyObserver);
} else {
  initSpotifyObserver();
}
