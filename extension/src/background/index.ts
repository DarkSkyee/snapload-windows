import { NATIVE_HOST_NAME } from "../shared/constants";
import { api } from "../shared/browser-api";
import {
  EngineResponse,
  InternalMessage,
  ProgressData,
  CompletedData,
  StartDownloadPayload,
  UpdateCheckResult,
} from "../shared/types";

interface DownloadState {
  id: string;
  url: string;
  percent: number;
  speed: string;
  eta: string;
  status: string;
  error?: string;
  completed?: boolean;
}

const activeDownloads = new Map<string, DownloadState>();
const cancelledDownloads = new Set<string>();
const notifiedCancelled = new Set<string>();
const MAX_CONCURRENT_DOWNLOADS = 2;
const pendingQueue: { downloadId: string; payload: StartDownloadPayload }[] = [];
const runningDownloadIds = new Set<string>();
let nativePort: chrome.runtime.Port | null = null;
let isConnecting = false;
let isUpdateAvailable = false;
const SNAPLOAD_UPDATE_ALARM = "snapload-periodic-update-check";

function processNextInQueue() {
  while (runningDownloadIds.size < MAX_CONCURRENT_DOWNLOADS && pendingQueue.length > 0) {
    const next = pendingQueue.shift();
    if (!next) break;

    if (cancelledDownloads.has(next.downloadId)) {
      activeDownloads.delete(next.downloadId);
      updateBadge();
      continue;
    }

    const port = getOrCreateNativePort();
    if (!port) {
      const item = activeDownloads.get(next.downloadId);
      if (item) {
        item.status = "Failed";
        item.error = "SnapLoad Engine offline. Launch SnapLoad Setup.";
      }
      updateBadge();
      continue;
    }

    runningDownloadIds.add(next.downloadId);
    const item = activeDownloads.get(next.downloadId);
    if (item && item.status.startsWith("Queued")) {
      item.status = "Starting...";
    }
    updateBadge();

    const engineReq = {
      id: next.downloadId,
      version: "1.0",
      action: "start_download",
      payload: next.payload,
    };

    try {
      port.postMessage(engineReq);
    } catch (err: any) {
      runningDownloadIds.delete(next.downloadId);
      if (item) {
        item.status = "Failed";
        item.error = err?.message || "Failed to communicate with Engine";
      }
      updateBadge();
      // Try next in queue
      processNextInQueue();
    }
  }
}

function getOrCreateNativePort(): chrome.runtime.Port | null {
  if (nativePort) return nativePort;
  if (isConnecting) return null;

  try {
    isConnecting = true;
    console.log(`[SnapLoad Background] Connecting to Native Messaging host: ${NATIVE_HOST_NAME}`);
    const port = api.runtime.connectNative(NATIVE_HOST_NAME);
    if (!port) {
      isConnecting = false;
      return null;
    }

    nativePort = port;
    nativePort.onMessage.addListener(handleNativeMessage);
    nativePort.onDisconnect.addListener(() => {
      const err = api.runtime.lastError ? api.runtime.lastError.message : "Port disconnected";
      console.warn("[SnapLoad Background] Native port disconnected:", err);
      nativePort = null;
      isConnecting = false;

      let friendlyError = err || "SnapLoad Engine disconnected";
      if (friendlyError.toLowerCase().includes("not found")) {
        friendlyError = "SnapLoad Host not registered in Windows Registry. Please run SnapLoad Setup.";
      } else if (friendlyError.toLowerCase().includes("forbidden") || friendlyError.toLowerCase().includes("denied")) {
        friendlyError = "Extension ID not allowed in Native Messaging host. Run SnapLoad Setup.";
      }

      runningDownloadIds.clear();
      pendingQueue.length = 0;

      // Mark all queued or running downloads as failed
      for (const dl of activeDownloads.values()) {
        if (!dl.completed && !dl.error) {
          dl.status = "Failed";
          dl.error = friendlyError;
        }
      }
      updateBadge();

      // Broadcast disconnect to any listening popups
      api.runtime.sendMessage({
        type: "ENGINE_STATUS",
        payload: { connected: false, error: friendlyError },
      }).catch(() => {});
      api.runtime.sendMessage({
        type: "ENGINE_EVENT",
        payload: { type: "download_failed" },
      }).catch(() => {});
    });

    isConnecting = false;
    return nativePort;
  } catch (e: any) {
    console.error("[SnapLoad Background] Failed to connect to Native host:", e);
    isConnecting = false;
    nativePort = null;
    return null;
  }
}

function handleNativeMessage(response: EngineResponse<any>) {
  console.log("[SnapLoad Background] Received from Engine:", response);

  const downloadId = (response.data && (response.data as any).download_id) || response.id;
  const current = activeDownloads.get(downloadId);

  // If this download was marked as cancelled, prevent resurrection by incoming events
  if (downloadId && cancelledDownloads.has(downloadId)) {
    activeDownloads.delete(downloadId);
    updateBadge();
    if (response.type === "download_cancelled") {
      if (!notifiedCancelled.has(downloadId)) {
        notifiedCancelled.add(downloadId);
        showNotification("SnapLoad Download Cancelled", "Download was stopped.");
        setTimeout(() => notifiedCancelled.delete(downloadId), 15000);
      }
      setTimeout(() => cancelledDownloads.delete(downloadId), 15000);
    }
    // Relay event to active popup
    if (response.type !== "pong") {
      api.runtime.sendMessage({
        type: "ENGINE_EVENT",
        payload: response,
      }).catch(() => {});
    }
    return;
  }

  switch (response.type) {
    case "download_started": {
      activeDownloads.set(downloadId, {
        id: downloadId,
        url: response.data?.url || "",
        percent: 0,
        speed: "",
        eta: "",
        status: "Starting...",
      });
      updateBadge();
      showNotification("SnapLoad Download Started", `Downloading: ${response.data?.url || ""}`);
      break;
    }

    case "download_progress": {
      const prog = response.data as ProgressData;
      if (prog) {
        activeDownloads.set(downloadId, {
          id: downloadId,
          url: current?.url || "",
          percent: Math.round(prog.percent),
          speed: prog.speed,
          eta: prog.eta,
          status: prog.status,
        });
        updateBadge();
      }
      break;
    }

    case "download_completed": {
      const completed = response.data as CompletedData;
      if (current) {
        current.completed = true;
        current.percent = 100;
        current.status = "Completed";
      }
      updateBadge();
      const filename = completed?.file_name || "File";
      showNotification("SnapLoad Download Finished", `${filename} successfully saved!`);
      // Keep for 15 seconds in memory so popup can show it, then remove
      setTimeout(() => {
        activeDownloads.delete(downloadId);
        updateBadge();
      }, 15000);
      break;
    }

    case "download_failed": {
      const errMsg = response.error?.message || "Download failed";
      if (current) {
        current.error = errMsg;
        current.status = "Failed";
      }
      updateBadge();
      showNotification("SnapLoad Download Failed", errMsg);
      break;
    }

    case "error": {
      const errMsg = response.error?.message || (response.error as any)?.code || "SnapLoad Engine Error";
      if (current) {
        current.error = errMsg;
        current.status = "Failed";
      } else if (downloadId && downloadId !== "null") {
        activeDownloads.set(downloadId, {
          id: downloadId,
          url: "",
          percent: 0,
          speed: "",
          eta: "",
          status: "Failed",
          error: errMsg,
        });
      }
      updateBadge();
      showNotification("SnapLoad Error", errMsg);
      break;
    }

    case "download_cancelled": {
      if (downloadId) {
        cancelledDownloads.add(downloadId);
        activeDownloads.delete(downloadId);
        setTimeout(() => cancelledDownloads.delete(downloadId), 15000);
      }
      updateBadge();
      if (downloadId && !notifiedCancelled.has(downloadId)) {
        notifiedCancelled.add(downloadId);
        showNotification("SnapLoad Download Cancelled", "Download was stopped.");
        setTimeout(() => notifiedCancelled.delete(downloadId), 15000);
      }
      break;
    }
  }

  if (
    response.type === "download_completed" ||
    response.type === "download_failed" ||
    response.type === "error" ||
    response.type === "download_cancelled"
  ) {
    if (downloadId) {
      runningDownloadIds.delete(downloadId);
    }
    processNextInQueue();
  }

  // Relay event to active popup (skip ping/pong to avoid triggering ping cascades)
  if (response.type !== "pong") {
    api.runtime.sendMessage({
      type: "ENGINE_EVENT",
      payload: response,
    }).catch(() => {
      // Popup might be closed, which is normal
    });
  }
}

function updateBadge() {
  const activeList = Array.from(activeDownloads.values()).filter(d => !d.completed && !d.error);
  const actionApi = api.action || (api as any).browserAction;
  if (!actionApi) return;

  if (activeList.length > 0) {
    const avgPercent = Math.round(activeList.reduce((acc, d) => acc + d.percent, 0) / activeList.length);
    actionApi.setBadgeText({ text: `${avgPercent}%` });
    actionApi.setBadgeBackgroundColor({ color: "#00F2FE" });
    return;
  }

  if (isUpdateAvailable) {
    actionApi.setBadgeText({ text: "NEW" });
    actionApi.setBadgeBackgroundColor({ color: "#EF4444" });
  } else {
    actionApi.setBadgeText({ text: "" });
  }
}

// --- SemVer 2.0.0 Version Comparison & GitHub Release Checker ---

interface ParsedSemVer {
  core: number[];
  prerelease: (number | string)[] | null;
}

function parseSemVer(v: string): ParsedSemVer {
  const clean = v.trim().replace(/^v/i, "");
  const withoutBuild = clean.split("+")[0];
  const [corePart, prereleasePart] = withoutBuild.split("-");

  const core = (corePart || "")
    .split(".")
    .map((p) => {
      const num = parseInt(p.replace(/\D.*$/, ""), 10);
      return isNaN(num) ? 0 : num;
    });

  let prerelease: (number | string)[] | null = null;
  if (prereleasePart !== undefined && prereleasePart !== "") {
    prerelease = prereleasePart
      .split(".")
      .filter((s) => s.length > 0)
      .map((id) => {
        if (/^\d+$/.test(id)) {
          return parseInt(id, 10);
        }
        return id.toLowerCase();
      });
  }

  return { core, prerelease };
}

export function compareSemVer(a: string, b: string): number {
  const va = parseSemVer(a);
  const vb = parseSemVer(b);

  const maxLen = Math.max(va.core.length, vb.core.length);
  for (let i = 0; i < maxLen; i++) {
    const aVal = va.core[i] ?? 0;
    const bVal = vb.core[i] ?? 0;
    if (aVal > bVal) return 1;
    if (aVal < bVal) return -1;
  }

  // SemVer 2.0.0 rule: normal version has higher precedence than pre-release version
  if (va.prerelease === null && vb.prerelease !== null) return 1;
  if (va.prerelease !== null && vb.prerelease === null) return -1;
  if (va.prerelease === null && vb.prerelease === null) return 0;

  const preA = va.prerelease!;
  const preB = vb.prerelease!;
  const minLen = Math.min(preA.length, preB.length);

  for (let i = 0; i < minLen; i++) {
    const idA = preA[i];
    const idB = preB[i];

    if (typeof idA === "number" && typeof idB === "number") {
      if (idA > idB) return 1;
      if (idA < idB) return -1;
    } else if (typeof idA === "number" && typeof idB === "string") {
      return -1;
    } else if (typeof idA === "string" && typeof idB === "number") {
      return 1;
    } else {
      const cmp = (idA as string).localeCompare(idB as string);
      if (cmp !== 0) return cmp > 0 ? 1 : -1;
    }
  }

  if (preA.length > preB.length) return 1;
  if (preA.length < preB.length) return -1;
  return 0;
}

interface GitHubReleaseItem {
  tag_name: string;
  name?: string;
  draft?: boolean;
  prerelease?: boolean;
  html_url?: string;
  body?: string;
}

interface ReleaseCandidate {
  version: string;
  releaseUrl: string;
  releaseNotes: string;
}

async function fetchWithTimeout(url: string, options: RequestInit = {}, timeoutMs = 8000): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(url, { ...options, signal: controller.signal });
    clearTimeout(timer);
    return res;
  } catch (err) {
    clearTimeout(timer);
    throw err;
  }
}

async function fetchLatestReleaseCandidate(): Promise<ReleaseCandidate | null> {
  const releasesApiUrl = "https://api.github.com/repos/DarkSkyee/SnapLoad-Windows/releases?per_page=10";
  const manifestFallbackUrl = "https://raw.githubusercontent.com/DarkSkyee/SnapLoad-Windows/main/release_manifest.json";
  const latestApiUrl = "https://api.github.com/repos/DarkSkyee/SnapLoad-Windows/releases/latest";

  // 1. Primary: GitHub Releases list
  try {
    const res = await fetchWithTimeout(releasesApiUrl, {
      headers: { Accept: "application/vnd.github.v3+json" },
    });
    if (res.ok) {
      const releases: GitHubReleaseItem[] = await res.json();
      if (Array.isArray(releases) && releases.length > 0) {
        const valid = releases.filter((r) => !r.draft);
        if (valid.length > 0) {
          valid.sort((a, b) => compareSemVer(b.tag_name || "0.0.0", a.tag_name || "0.0.0"));
          const best = valid[0];
          return {
            version: (best.tag_name || "").replace(/^v/i, "").trim(),
            releaseUrl: best.html_url || "https://github.com/DarkSkyee/SnapLoad-Windows/releases",
            releaseNotes: best.body || "",
          };
        }
      }
    }
  } catch (err) {
    console.warn("[SnapLoad Update] GitHub releases list fetch failed:", err);
  }

  // 2. Fallback: releases/latest
  try {
    const res = await fetchWithTimeout(latestApiUrl, {
      headers: { Accept: "application/vnd.github.v3+json" },
    });
    if (res.ok) {
      const latest: GitHubReleaseItem = await res.json();
      if (latest && !latest.draft && latest.tag_name) {
        return {
          version: latest.tag_name.replace(/^v/i, "").trim(),
          releaseUrl: latest.html_url || "https://github.com/DarkSkyee/SnapLoad-Windows/releases",
          releaseNotes: latest.body || "",
        };
      }
    }
  } catch (err) {
    console.warn("[SnapLoad Update] GitHub releases/latest fetch failed:", err);
  }

  // 3. Fallback: raw release_manifest.json on main
  try {
    const res = await fetchWithTimeout(manifestFallbackUrl, {
      headers: { Accept: "application/json" },
    });
    if (res.ok) {
      const manifest = await res.json();
      if (manifest && manifest.version) {
        return {
          version: String(manifest.version).replace(/^v/i, "").trim(),
          releaseUrl: "https://github.com/DarkSkyee/SnapLoad-Windows/releases",
          releaseNotes: `SnapLoad Release v${manifest.version}`,
        };
      }
    }
  } catch (err) {
    console.warn("[SnapLoad Update] Raw release_manifest.json fetch failed:", err);
  }

  return null;
}

export async function checkForUpdates(isManual = false): Promise<UpdateCheckResult> {
  const currentVersion = api.runtime.getManifest?.()?.version || "1.0.0";
  console.log(`[SnapLoad Update] Checking for updates (current: ${currentVersion}, manual: ${isManual})...`);

  const candidate = await fetchLatestReleaseCandidate();
  const now = Date.now();

  if (!candidate) {
    console.warn("[SnapLoad Update] Could not reach GitHub releases or release manifest.");
    const cached = await new Promise<{ updateAvailable?: boolean; latestVersion?: string; releaseUrl?: string }>((resolve) => {
      api.storage?.local?.get(["updateAvailable", "latestVersion", "releaseUrl"], (data) => {
        resolve(data || {});
      });
    });

    return {
      updateAvailable: cached.updateAvailable || false,
      currentVersion,
      latestVersion: cached.latestVersion,
      releaseUrl: cached.releaseUrl,
      lastChecked: now,
      error: "Unable to contact GitHub releases. Please check your connection.",
    };
  }

  const isNewer = compareSemVer(candidate.version, currentVersion) > 0;

  if (isNewer) {
    console.log(`[SnapLoad Update] New update available: v${candidate.version} (current: v${currentVersion})`);
    isUpdateAvailable = true;
    updateBadge();

    const updateInfo = {
      updateAvailable: true,
      latestVersion: candidate.version,
      releaseUrl: candidate.releaseUrl,
      releaseNotes: candidate.releaseNotes,
      lastChecked: now,
    };

    await api.storage?.local?.set(updateInfo);

    api.runtime.sendMessage({
      type: "UPDATE_STATUS_CHANGED",
      payload: updateInfo,
    }).catch(() => {});

    return {
      ...updateInfo,
      currentVersion,
    };
  } else {
    console.log(`[SnapLoad Update] Up to date (v${currentVersion})`);
    isUpdateAvailable = false;
    updateBadge();

    const updateInfo = {
      updateAvailable: false,
      lastChecked: now,
    };

    await api.storage?.local?.set(updateInfo);

    api.runtime.sendMessage({
      type: "UPDATE_STATUS_CHANGED",
      payload: updateInfo,
    }).catch(() => {});

    return {
      ...updateInfo,
      currentVersion,
      latestVersion: candidate.version,
    };
  }
}

// Alarms & Periodic checking (every 12 hours = 720 minutes)
if (api.alarms) {
  api.alarms.onAlarm.addListener((alarm) => {
    if (alarm.name === SNAPLOAD_UPDATE_ALARM) {
      console.log("[SnapLoad Update] Periodic 12-hour update alarm triggered.");
      checkForUpdates();
    }
  });
}

function initUpdateChecker() {
  if (api.alarms) {
    api.alarms.get(SNAPLOAD_UPDATE_ALARM, (alarm) => {
      if (!alarm) {
        api.alarms.create(SNAPLOAD_UPDATE_ALARM, { periodInMinutes: 720 });
      }
    });
  }

  // Restore update state from storage and check if overdue (> 12 hours)
  api.storage?.local?.get(["updateAvailable", "lastChecked"], (stored) => {
    if (stored?.updateAvailable) {
      isUpdateAvailable = true;
      updateBadge();
    }
    const lastChecked = stored?.lastChecked || 0;
    const twelveHoursMs = 12 * 60 * 60 * 1000;
    if (Date.now() - lastChecked > twelveHoursMs) {
      checkForUpdates();
    }
  });
}

initUpdateChecker();

function showNotification(title: string, message: string) {
  try {
    api.notifications?.create({
      type: "basic",
      iconUrl: "assets/icon-128.png",
      title: title,
      message: message,
    });
  } catch (e) {
    console.debug("[SnapLoad] Notification skipped:", e);
  }
}

// Send download command to Rust Engine via BatchQueueManager (max 2 concurrent)
export function requestDownload(payload: StartDownloadPayload): string {
  const downloadId = "dl_" + Math.random().toString(36).substring(2, 10);

  const isSlotAvailable = runningDownloadIds.size < MAX_CONCURRENT_DOWNLOADS;
  activeDownloads.set(downloadId, {
    id: downloadId,
    url: payload.url,
    percent: 0,
    speed: "",
    eta: "",
    status: isSlotAvailable ? "Queued" : "Queued (Waiting for slot)",
  });
  updateBadge();

  pendingQueue.push({ downloadId, payload });
  processNextInQueue();

  return downloadId;
}

// Setup context menus, alarms, and lifecycle listeners
api.runtime.onInstalled?.addListener(() => {
  api.contextMenus?.create({
    id: "snapload-download-menu",
    title: "Download with SnapLoad",
    contexts: ["page", "link", "video", "audio"],
  });

  if (api.alarms) {
    api.alarms.create(SNAPLOAD_UPDATE_ALARM, { periodInMinutes: 720 });
  }
  checkForUpdates();
});

api.runtime.onStartup?.addListener(() => {
  checkForUpdates();
});

api.contextMenus?.onClicked.addListener((info: chrome.contextMenus.OnClickData, tab?: chrome.tabs.Tab) => {
  const targetUrl = info.linkUrl || info.srcUrl || info.pageUrl || tab?.url;
  if (targetUrl) {
    console.log("[SnapLoad Context Menu] Initiating download for:", targetUrl);
    requestDownload({ url: targetUrl });
  }
});

// Internal message router
api.runtime.onMessage.addListener((message: InternalMessage, _sender: any, sendResponse: (resp: any) => void) => {
  switch (message.type) {
    case "START_DOWNLOAD": {
      const id = requestDownload(message.payload);
      sendResponse({ success: true, downloadId: id });
      return true;
    }
    case "CANCEL_DOWNLOAD": {
      const dlId = message.payload?.download_id;
      if (dlId) {
        cancelledDownloads.add(dlId);
        const qIdx = pendingQueue.findIndex(q => q.downloadId === dlId);
        if (qIdx !== -1) {
          pendingQueue.splice(qIdx, 1);
        }
        runningDownloadIds.delete(dlId);
        activeDownloads.delete(dlId);
        updateBadge();
        processNextInQueue();

        const port = getOrCreateNativePort();
        if (port) {
          try {
            port.postMessage({
              id: "cancel_" + Date.now(),
              version: "1.0",
              action: "cancel_download",
              payload: { download_id: dlId },
            });
          } catch (err) {
            console.warn("[SnapLoad Background] Failed to send cancel to native port:", err);
          }
        }

        // Notify active popup immediately
        api.runtime.sendMessage({
          type: "ENGINE_EVENT",
          payload: {
            id: dlId,
            type: "download_cancelled",
            data: { download_id: dlId },
          },
        }).catch(() => {});
      }
      sendResponse({ success: true });
      return true;
    }
    case "DISMISS_DOWNLOAD": {
      if (message.payload?.download_id) {
        activeDownloads.delete(message.payload.download_id);
        updateBadge();
      }
      sendResponse({ success: true });
      return true;
    }
    case "GET_ACTIVE_DOWNLOADS": {
      sendResponse({ downloads: Array.from(activeDownloads.values()) });
      return true;
    }
    case "PING_ENGINE": {
      const port = getOrCreateNativePort();
      if (!port) {
        sendResponse({ connected: false });
      } else {
        port.postMessage({
          id: "ping_" + Date.now(),
          version: "1.0",
          action: "ping",
        });
        sendResponse({ connected: true });
      }
      return true;
    }
    case "CHECK_FOR_UPDATES": {
      checkForUpdates(true).then((info) => {
        sendResponse(info);
      }).catch((err) => {
        sendResponse({
          updateAvailable: false,
          error: err?.message || String(err),
          lastChecked: Date.now(),
        });
      });
      return true;
    }
    case "GET_UPDATE_INFO": {
      api.storage?.local?.get(
        ["updateAvailable", "latestVersion", "releaseUrl", "releaseNotes", "lastChecked"],
        (data) => {
          sendResponse(data || { updateAvailable: false, lastChecked: 0 });
        }
      );
      return true;
    }
  }
});
