# SnapLoad Windows — Browser Compatibility Matrix

## 1. Supported Browsers

| Browser | Engine | Platform / Manifest | Native Messaging Support | YouTube In-Page Button | Context Menu |
|---|---|---|:---:|:---:|:---:|
| **Google Chrome** | Chromium (V8/Blink) | Manifest V3 (Service Worker) | ✅ Full | ✅ Full | ✅ Full |
| **Microsoft Edge** | Chromium (V8/Blink) | Manifest V3 (Service Worker) | ✅ Full | ✅ Full | ✅ Full |
| **Brave Browser** | Chromium (V8/Blink) | Manifest V3 (Service Worker) | ✅ Full | ✅ Full | ✅ Full |
| **Opera / Opera GX** | Chromium (V8/Blink) | Manifest V3 (Service Worker) | ✅ Full | ✅ Full | ✅ Full |
| **Mozilla Firefox** | Gecko / SpiderMonkey | WebExtensions (Background Script) | ✅ Full | ✅ Full | ✅ Full |

---

## 2. Platform-Specific Architectural Considerations

### Chromium (Manifest V3)
- Background processes execute as **Service Workers**.
- Native Messaging ports disconnect when the service worker becomes idle (typically after ~30 seconds of inactivity).
- SnapLoad background worker automatically establishes on-demand connections and gracefully reconnects whenever a user initiates a download or requests status.

### Mozilla Firefox
- Background processes execute using persistent background pages/scripts.
- Requires explicit `browser_specific_settings` block in `manifest.json`:
  ```json
  "browser_specific_settings": {
    "gecko": {
      "id": "snapload@darkskyee.github.io",
      "strict_min_version": "109.0"
    }
  }
  ```
- Firefox native messaging host manifest uses `"allowed_extensions": ["snapload@darkskyee.github.io"]` instead of Chromium's `"allowed_origins"`.

---

## 3. YouTube Single Page Application (SPA) Integration

YouTube uses an internal dynamic single-page architecture where navigating between video pages does not trigger standard HTTP page reloads.

SnapLoad solves this via a multi-layered approach:
1. **Primary Event**: Listens for the official YouTube SPA navigation event `yt-navigate-finish` dispatched on `window`.
2. **Target Container**: Injects the branded download button into `#top-level-buttons-computed` (adjacent to the Like and Share action buttons).
3. **Duplicate Prevention**: The button carries a unique DOM ID `snapload-yt-action-button`. Injections check for the element's existence before modifying the DOM.
4. **Fallback Observer**: A debounced `MutationObserver` watches `#actions` to re-inject the button if YouTube's polymer elements recreate the player action bar dynamically.
