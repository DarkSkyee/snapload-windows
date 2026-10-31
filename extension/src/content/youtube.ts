import { api } from "../shared/browser-api";

const BUTTON_ID = "snapload-yt-action-button";
let observer: MutationObserver | null = null;
let lastUrl = "";

function createSnapLoadButton(): HTMLElement {
  const container = document.createElement("div");
  container.id = BUTTON_ID;
  container.className = "snapload-btn-container";
  container.setAttribute("role", "button");
  container.setAttribute("tabindex", "0");
  container.setAttribute("aria-label", "Download with SnapLoad");

  // Modern SnapLoad Monogram & Download Arrow SVG
  container.innerHTML = `
    <button class="snapload-download-btn" type="button" title="Download with SnapLoad">
      <div class="snapload-btn-icon">
        <svg viewBox="0 0 108 108" width="18" height="18" fill="none">
          <defs>
            <linearGradient id="snaploadGrad" x1="0" y1="0" x2="108" y2="108" gradientUnits="userSpaceOnUse">
              <stop offset="0%" stop-color="#00F2FE"/>
              <stop offset="50%" stop-color="#6366F1"/>
              <stop offset="100%" stop-color="#A855F7"/>
            </linearGradient>
          </defs>
          <path d="M66,32 C58,28 44,29 38,36 C34,41 35,49 42,53 L58,60 C64,63 66,67 64,72 C61,77 53,79 46,77 C41,75 37,71 36,67"
                stroke="url(#snaploadGrad)" stroke-width="8" stroke-linecap="round" fill="none"/>
          <path d="M54,40 L54,66 M44,56 L54,66 L64,56" stroke="#FFFFFF" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
      <span class="snapload-btn-label">SnapLoad</span>
    </button>
  `;

  // Apply scoped button styling inline to avoid CSS collisions with YouTube themes
  const style = document.createElement("style");
  style.textContent = `
    #${BUTTON_ID} {
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
  `;
  container.appendChild(style);

  const btn = container.querySelector(".snapload-download-btn") as HTMLButtonElement;
  btn.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();

    const videoUrl = window.location.href;
    console.log("[SnapLoad Content] Triggering download for current URL:", videoUrl);

    btn.classList.add("downloading");
    const label = btn.querySelector(".snapload-btn-label");
    if (label) label.textContent = "Sending...";

    api.runtime.sendMessage({
      type: "START_DOWNLOAD",
      payload: { url: videoUrl },
    }, (response: any) => {
      if (response && response.success) {
        if (label) label.textContent = "Started ✓";
      } else {
        if (label) label.textContent = "Check Engine";
      }
      setTimeout(() => {
        btn.classList.remove("downloading");
        if (label) label.textContent = "SnapLoad";
      }, 3000);
    });
  });

  return container;
}

function injectButton() {
  if (!window.location.pathname.startsWith("/watch")) {
    return;
  }

  if (document.getElementById(BUTTON_ID)) {
    return; // Already present
  }

  // Preferred target: the action buttons container next to Like / Share
  const candidates = [
    "#top-level-buttons-computed",
    "#actions-inner #menu",
    "#actions #top-level-buttons-computed",
    "ytd-watch-metadata #actions #top-level-buttons-computed",
    "#owner #subscribe-button",
  ];

  let targetElement: Element | null = null;
  for (const selector of candidates) {
    const el = document.querySelector(selector);
    if (el) {
      targetElement = el;
      break;
    }
  }

  if (targetElement) {
    const button = createSnapLoadButton();
    targetElement.appendChild(button);
    console.log("[SnapLoad Content] Download button successfully injected into:", targetElement);
  }
}

function initNavigationListener() {
  // YouTube SPA event
  window.addEventListener("yt-navigate-finish", () => {
    if (window.location.href !== lastUrl) {
      lastUrl = window.location.href;
      // Re-evaluate button injection after YouTube DOM settles
      setTimeout(injectButton, 300);
      setTimeout(injectButton, 1000);
    }
  });

  // Fallback MutationObserver to handle dynamically loaded elements
  observer = new MutationObserver(() => {
    if (window.location.pathname.startsWith("/watch") && !document.getElementById(BUTTON_ID)) {
      injectButton();
    }
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });

  // Initial attempt
  injectButton();
  setTimeout(injectButton, 1000);
}

// Start watching after document is interactive
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initNavigationListener);
} else {
  initNavigationListener();
}
