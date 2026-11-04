import { NATIVE_HOST_NAME } from "../shared/constants";
import { api } from "../shared/browser-api";
import { EngineResponse, InternalMessage, ProgressData, CompletedData, StartDownloadPayload } from "../shared/types";

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
let nativePort: chrome.runtime.Port | null = null;
let isConnecting = false;

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
      showNotification("SnapLoad Download Cancelled", "Download was stopped.");
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
      showNotification("SnapLoad Download Cancelled", "Download was stopped.");
      break;
    }
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

  if (activeList.length === 0) {
    actionApi.setBadgeText({ text: "" });
    return;
  }
  const avgPercent = Math.round(activeList.reduce((acc, d) => acc + d.percent, 0) / activeList.length);
  actionApi.setBadgeText({ text: `${avgPercent}%` });
  actionApi.setBadgeBackgroundColor({ color: "#00F2FE" });
}

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

// Send download command to Rust Engine
export function requestDownload(payload: StartDownloadPayload): string {
  const downloadId = "dl_" + Math.random().toString(36).substring(2, 10);

  // Immediately track as Queued so popup UI displays active download item instantly
  activeDownloads.set(downloadId, {
    id: downloadId,
    url: payload.url,
    percent: 0,
    speed: "",
    eta: "",
    status: "Queued",
  });
  updateBadge();

  const port = getOrCreateNativePort();

  if (!port) {
    const item = activeDownloads.get(downloadId);
    if (item) {
      item.status = "Failed";
      item.error = "SnapLoad Engine offline. Launch SnapLoad Setup.";
    }
    updateBadge();
    showNotification("SnapLoad Engine Not Connected", "Please ensure SnapLoad Windows Setup has been executed.");
    return downloadId;
  }

  const engineReq = {
    id: downloadId,
    version: "1.0",
    action: "start_download",
    payload: payload,
  };

  try {
    port.postMessage(engineReq);
  } catch (err: any) {
    const item = activeDownloads.get(downloadId);
    if (item) {
      item.status = "Failed";
      item.error = err?.message || "Failed to communicate with Engine";
    }
    updateBadge();
  }

  return downloadId;
}

// Setup context menus
api.runtime.onInstalled?.addListener(() => {
  api.contextMenus?.create({
    id: "snapload-download-menu",
    title: "Download with SnapLoad",
    contexts: ["page", "link", "video", "audio"],
  });
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
        activeDownloads.delete(dlId);
        updateBadge();

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
  }
});
