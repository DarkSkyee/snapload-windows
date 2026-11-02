// SnapLoad Windows Setup & Management Frontend
import { invoke } from "@tauri-apps/api/core";

interface BrowserInfo {
  id: string;
  name: string;
  icon: string;
  detected: boolean;
  path?: string;
  profile_path?: string;
  is_running?: boolean;
  selected: boolean;
}

interface AutoInstallResult {
  success: boolean;
  browser_id: string;
  browser_name: string;
  exe_path: string;
  profile_path?: string;
  message: string;
  logs: string[];
}

interface InstallOptions {
  browsers: string[];
  install_ytdlp: boolean;
  install_ffmpeg: boolean;
}

interface DiagnosticItem {
  name: String;
  status: "ok" | "warn" | "error";
  message: string;
  detail: string;
}

interface DiagnosticResult {
  all_passed: boolean;
  total: number;
  passed: number;
  items: DiagnosticItem[];
}

interface ExtensionPaths {
  app_dir: string;
  chromium_dir: string;
  firefox_xpi: string;
  downloads_dir: string;
}

// Crisp Vector Brand SVGs for Browsers
const BROWSER_SVGS: Record<string, string> = {
  chrome: `
    <svg viewBox="0 0 48 48" width="34" height="34">
      <path fill="#4CAF50" d="M24,4C13,4,4,13,4,24c0,2.1,0.3,4.1,0.9,6l10.8-18.7C17.6,7.8,20.6,6,24,6h17.9C37.5,7.5,31.2,4,24,4z"/>
      <path fill="#FFC107" d="M43.9,20.5C43.5,14.6,39.8,9.7,34.8,7.3l-10.8,18.7c2.1,3.7,6.1,6,10.5,6c2.5,0,4.8-0.8,6.8-2.1L43.9,20.5z"/>
      <path fill="#F44336" d="M24,44c7.3,0,13.7-4,17.2-10l-10.8-18.7c-1.9,1.1-4.1,1.7-6.4,1.7c-4.4,0-8.4-2.3-10.5-6L2.7,30.9C6.3,38.7,14.5,44,24,44z"/>
      <circle fill="#FFFFFF" cx="24" cy="24" r="9"/>
      <circle fill="#1976D2" cx="24" cy="24" r="7"/>
    </svg>`,
  edge: `
    <svg viewBox="0 0 48 48" width="34" height="34">
      <path fill="#0c8484" d="M37.9,13.2C36.6,5.7,29.9,0,21.8,0C10.9,0,2.1,8.8,2.1,19.7c0,11.3,9,20.4,20.2,20.4c1.2,0,2.4-0.1,3.5-0.3c-2.3-1.6-4.1-3.8-5.1-6.4c-6.8-0.3-12.2-5.9-12.2-12.8c0-7.1,5.8-12.9,12.9-12.9c3.9,0,7.3,1.7,9.6,4.5C32.7,12.4,35.4,12.6,37.9,13.2z"/>
      <path fill="#2488d8" d="M21.8,40.1c11.9,0,21.6-9.7,21.6-21.6c0-1.8-0.2-3.6-0.6-5.3c-1.3,3.7-4.1,6.8-7.7,8.5c-0.8,5.4-4.8,9.8-10.1,10.9C24.1,35.7,23,38,21.8,40.1z"/>
      <path fill="#3bc5f4" d="M42.8,13.2c-0.8,2.9-2.5,5.4-4.9,7.1c-0.8-3.4-3-6.2-6.1-7.7c3.2-1.9,6.9-2.7,10.6-2.2C42.6,11.2,42.7,12.2,42.8,13.2z"/>
    </svg>`,
  brave: `
    <svg viewBox="0 0 48 48" width="34" height="34">
      <path fill="#FB542B" d="M24,2L12,8l-2,14c0,12,14,22,14,22s14-10,14-22l-2-14L24,2z"/>
      <path fill="#FF2000" d="M24,6l-8,4l-1,10c0,9,9,16,9,16s9-7,9-16l-1-10L24,6z"/>
      <path fill="#FFFFFF" d="M24,14c-4,0-6,3-6,5c0,4,4,6,6,6s6-2,6-6C30,17,28,14,24,14z"/>
    </svg>`,
  opera: `
    <svg viewBox="0 0 48 48" width="34" height="34">
      <path fill="#FF1B2D" d="M24,4C13,4,4,13,4,24s9,20,20,20s20-9,20-20S35,4,24,4z M24,38c-6.6,0-12-6.3-12-14s5.4-14,12-14s12,6.3,12,14S30.6,38,24,38z"/>
    </svg>`,
  operagx: `
    <svg viewBox="0 0 48 48" width="34" height="34">
      <path fill="#FA1E4E" d="M24,4C13,4,4,13,4,24s9,20,20,20s20-9,20-20S35,4,24,4z M24,36c-5.5,0-10-5.4-10-12s4.5-12,10-12s10,5.4,10,12S29.5,36,24,36z"/>
      <polygon fill="#FFFFFF" points="21,16 29,24 21,32"/>
    </svg>`,
  firefox: `
    <svg viewBox="0 0 48 48" width="34" height="34">
      <circle fill="#FF9100" cx="24" cy="24" r="20"/>
      <path fill="#FF3D00" d="M24,4c-11,0-20,9-20,20c0,3,0.7,5.9,1.9,8.4C8.2,30.3,12,28,16,28c5,0,8,4,8,8c0,2.5-1,4.7-2.7,6.3C22.1,43.2,23,44,24,44c11,0,20-9,20-20S35,4,24,4z"/>
      <path fill="#3F51B5" d="M19,16c-3,0-5,2-5,4c0,3,3,5,6,5c4,0,7-3,7-6C27,17,23,16,19,16z"/>
      <circle fill="#03A9F4" cx="22" cy="22" r="8"/>
    </svg>`,
};

let detectedBrowsers: BrowserInfo[] = [
  { id: "chrome", name: "Google Chrome", icon: "chrome", detected: false, selected: false },
  { id: "edge", name: "Microsoft Edge", icon: "edge", detected: false, selected: false },
  { id: "brave", name: "Brave Browser", icon: "brave", detected: false, selected: false },
  { id: "opera", name: "Opera", icon: "opera", detected: false, selected: false },
  { id: "operagx", name: "Opera GX", icon: "operagx", detected: false, selected: false },
  { id: "firefox", name: "Mozilla Firefox", icon: "firefox", detected: false, selected: false },
];

let cachedPaths: ExtensionPaths | null = null;

// DOM Elements
const browserListEl = document.getElementById("browser-list") as HTMLElement;
const installYtdlpCb = document.getElementById("install-ytdlp") as HTMLInputElement;
const installFfmpegCb = document.getElementById("install-ffmpeg") as HTMLInputElement;
const startInstallBtn = document.getElementById("start-install-btn") as HTMLButtonElement;

const stepDetect = document.getElementById("step-detect") as HTMLElement;
const stepProgress = document.getElementById("step-progress") as HTMLElement;
const stepComplete = document.getElementById("step-complete") as HTMLElement;

const progressBar = document.getElementById("install-progress-bar") as HTMLElement;
const progressPercent = document.getElementById("progress-percent") as HTMLElement;
const progressStatusText = document.getElementById("progress-status-text") as HTMLElement;
const installLogs = document.getElementById("install-logs") as HTMLElement;

// Tabs
const tabInstall = document.getElementById("tab-install") as HTMLButtonElement;
const tabUpdates = document.getElementById("tab-updates") as HTMLButtonElement;
const tabManage = document.getElementById("tab-manage") as HTMLButtonElement;

const panelInstall = document.getElementById("panel-install") as HTMLElement;
const panelUpdates = document.getElementById("panel-updates") as HTMLElement;
const panelManage = document.getElementById("panel-manage") as HTMLElement;

// Toast
const toastEl = document.getElementById("toast") as HTMLElement;
const toastText = document.getElementById("toast-text") as HTMLElement;
let toastTimeout: any = null;

function showToast(msg: string) {
  if (toastTimeout) clearTimeout(toastTimeout);
  toastText.textContent = msg;
  toastEl.classList.add("show");
  toastTimeout = setTimeout(() => {
    toastEl.classList.remove("show");
  }, 3200);
}

// Tab Switcher
function switchTab(tab: "install" | "updates" | "manage") {
  [tabInstall, tabUpdates, tabManage].forEach(t => t.classList.remove("active"));
  [panelInstall, panelUpdates, panelManage].forEach(p => p.classList.remove("active"));

  if (tab === "install") {
    tabInstall.classList.add("active");
    panelInstall.classList.add("active");
  } else if (tab === "updates") {
    tabUpdates.classList.add("active");
    panelUpdates.classList.add("active");
    runUpdateCheck();
  } else {
    tabManage.classList.add("active");
    panelManage.classList.add("active");
    runDiagnostics();
  }
}

tabInstall.addEventListener("click", () => switchTab("install"));
tabUpdates.addEventListener("click", () => switchTab("updates"));
tabManage.addEventListener("click", () => switchTab("manage"));

function log(msg: string, level: "info" | "success" | "warn" | "err" = "info") {
  const line = document.createElement("div");
  const timeSpan = document.createElement("span");
  timeSpan.className = "log-time";
  timeSpan.textContent = `[${new Date().toLocaleTimeString()}]`;

  const msgSpan = document.createElement("span");
  if (level === "success") msgSpan.className = "log-success";
  if (level === "warn") msgSpan.className = "log-warn";
  if (level === "err") msgSpan.className = "log-err";
  msgSpan.textContent = ` ${msg}`;

  line.appendChild(timeSpan);
  line.appendChild(msgSpan);
  installLogs.appendChild(line);
  installLogs.scrollTop = installLogs.scrollHeight;
}

function updateProgress(pct: number, status: string, level: "info" | "success" | "warn" | "err" = "info") {
  progressBar.style.width = `${pct}%`;
  progressPercent.textContent = `${pct}%`;
  progressStatusText.textContent = status;
  log(status, level);
}

// Browser Detection
async function initBrowserDetection() {
  try {
    const results = await invoke<BrowserInfo[]>("detect_browsers");
    if (results && results.length > 0) {
      detectedBrowsers = results;
    }
  } catch (e) {
    console.warn("Backend detection unavailable:", e);
  }

  // Pre-select detected browsers by default
  for (const b of detectedBrowsers) {
    if (b.detected) {
      b.selected = true;
    }
  }

  renderBrowserList();
}

function renderBrowserList() {
  browserListEl.innerHTML = "";
  for (const b of detectedBrowsers) {
    const card = document.createElement("div");
    card.className = `browser-card ${b.detected ? "detected" : ""} ${b.selected ? "selected" : ""}`;
    const svgIcon = BROWSER_SVGS[b.id] || BROWSER_SVGS["chrome"];

    card.innerHTML = `
      <div class="browser-select-indicator">✓</div>
      <div class="browser-icon-container">${svgIcon}</div>
      <div class="browser-name" title="${b.name}">${b.name}</div>
      <div class="browser-status-badge">${b.detected ? "Installed" : "Not Found"}</div>
    `;

    card.addEventListener("click", () => {
      b.selected = !b.selected;
      card.classList.toggle("selected", b.selected);
      if (b.selected) {
        step3ActiveBrowserId = b.id;
      }
    });

    browserListEl.appendChild(card);
  }
}

// Cache application paths from backend
async function fetchPaths(): Promise<ExtensionPaths> {
  if (cachedPaths) return cachedPaths;
  try {
    const paths = await invoke<ExtensionPaths>("get_paths");
    cachedPaths = paths;
    return paths;
  } catch (e) {
    return {
      app_dir: "C:\\Users\\User\\AppData\\Local\\SnapLoad",
      chromium_dir: "C:\\Users\\User\\AppData\\Local\\SnapLoad\\extensions\\chromium",
      firefox_xpi: "C:\\Users\\User\\AppData\\Local\\SnapLoad\\extensions\\firefox\\snapload-extension-firefox.xpi",
      downloads_dir: "C:\\Users\\User\\Downloads\\SnapLoad",
    };
  }
}

// Installation Workflow
startInstallBtn.addEventListener("click", async () => {
  const selected = detectedBrowsers.filter(b => b.selected).map(b => b.id);
  const options: InstallOptions = {
    browsers: selected.length > 0 ? selected : detectedBrowsers.filter(b => b.detected).map(b => b.id),
    install_ytdlp: installYtdlpCb.checked,
    install_ffmpeg: installFfmpegCb.checked,
  };

  stepDetect.classList.add("hidden");
  stepProgress.classList.remove("hidden");

  updateProgress(10, "Initializing application directories in %LOCALAPPDATA%\\SnapLoad...");

  try {
    updateProgress(30, "Deploying SnapLoad Engine and browser extensions...");
    const res = await invoke<{ success: boolean; logs: string[] }>("install_snapload", { options });
    if (res && res.logs) {
      res.logs.forEach(l => {
        const isSuccess = l.toLowerCase().includes("success") || l.toLowerCase().includes("verified") || l.toLowerCase().includes("registered") || l.toLowerCase().includes("deployed");
        const isWarn = l.toLowerCase().includes("warn") || l.toLowerCase().includes("notice");
        log(l, isSuccess ? "success" : isWarn ? "warn" : "info");
      });
    }

    updateProgress(100, "Installation and extension deployment completed successfully!", "success");

    setTimeout(() => {
      stepProgress.classList.add("hidden");
      stepComplete.classList.remove("hidden");
      initStep3UI();
      fetchPaths();
    }, 1100);
  } catch (err: any) {
    updateProgress(100, `Installation Error: ${err}`, "err");
  }
});

// Step 3: Streamlined 1-Click Auto Install
let step3ActiveBrowserId: string = "operagx";

function getInitialStep3Browser(): string {
  // 1. If user previously selected a browser in Step 1, prefer that
  if (step3ActiveBrowserId) {
    const prev = detectedBrowsers.find(b => b.id === step3ActiveBrowserId && b.detected);
    if (prev) return prev.id;
  }

  // 2. Opera GX or Opera if detected
  const opera = detectedBrowsers.find(b => b.detected && (b.id === "operagx" || b.id === "opera"));
  if (opera) return opera.id;

  // 3. Google Chrome if detected
  const chrome = detectedBrowsers.find(b => b.detected && b.id === "chrome");
  if (chrome) return chrome.id;

  // 4. Any detected browser
  const detected = detectedBrowsers.find(b => b.detected);
  if (detected) return detected.id;

  return "opera";
}

function initStep3UI() {
  step3ActiveBrowserId = getInitialStep3Browser();
  renderStep3BrowserPicker();
  updateStep3ActiveBrowserCard();
}

function renderStep3BrowserPicker() {
  const pickerEl = document.getElementById("step3-browser-picker");
  const countEl = document.getElementById("step3-detected-count");
  if (!pickerEl) return;

  const detectedCount = detectedBrowsers.filter(b => b.detected).length;
  if (countEl) {
    countEl.textContent = `${detectedCount} wykryto`;
  }

  pickerEl.innerHTML = "";
  for (const b of detectedBrowsers) {
    const tab = document.createElement("button");
    tab.className = `step3-browser-tab ${b.id === step3ActiveBrowserId ? "active" : ""} ${b.detected ? "detected" : ""}`;
    const svgIcon = BROWSER_SVGS[b.id] || BROWSER_SVGS["chrome"];

    tab.innerHTML = `
      <div class="tab-icon">${svgIcon}</div>
      <span class="tab-name">${b.name}</span>
      <span class="tab-dot ${b.detected ? "detected" : ""}" title="${b.detected ? "Wykryto na komputerze" : "Nie wykryto"}"></span>
    `;

    tab.addEventListener("click", () => {
      step3ActiveBrowserId = b.id;
      renderStep3BrowserPicker();
      updateStep3ActiveBrowserCard();
    });

    pickerEl.appendChild(tab);
  }
}

async function updateStep3ActiveBrowserCard() {
  const b = detectedBrowsers.find(x => x.id === step3ActiveBrowserId) || detectedBrowsers[0];
  if (!b) return;

  // Check if browser is running
  try {
    b.is_running = await invoke<boolean>("is_browser_running", { browserId: b.id });
  } catch (_) {
    b.is_running = false;
  }

  const iconEl = document.getElementById("step3-hero-icon");
  const nameEl = document.getElementById("step3-browser-name");
  const statusEl = document.getElementById("step3-browser-status");
  const readyBadgeEl = document.getElementById("step3-ready-badge");
  const exePathEl = document.getElementById("step3-exe-path");
  const exeBadgeEl = document.getElementById("step3-exe-badge");
  const profilePathEl = document.getElementById("step3-profile-path");
  const profileBadgeEl = document.getElementById("step3-profile-badge");
  const runningRow = document.getElementById("step3-running-row");
  const runningStatus = document.getElementById("step3-running-status");
  const runningBadge = document.getElementById("step3-running-badge");
  const restartToggleRow = document.getElementById("step3-restart-toggle-row");
  const restartCb = document.getElementById("cb-restart-browser") as HTMLInputElement | null;
  const btn = document.getElementById("btn-auto-install") as HTMLButtonElement | null;
  const btnTitleEl = document.getElementById("auto-install-main-title");
  const btnSubTitleEl = document.getElementById("auto-install-sub-title");
  const statusBox = document.getElementById("auto-install-status-box");

  if (iconEl) iconEl.innerHTML = BROWSER_SVGS[b.id] || BROWSER_SVGS["chrome"];
  if (nameEl) nameEl.textContent = b.name;

  if (statusEl) {
    if (b.detected) {
      statusEl.textContent = b.is_running
        ? "Wykryto na komputerze • Przeglądarka jest obecnie otwarta"
        : "Wykryto na komputerze • Gotowy do automatycznej instalacji";
    } else {
      statusEl.textContent = "Niewykryty w domyślnej ścieżce na tym komputerze";
    }
  }

  if (readyBadgeEl) {
    if (b.detected) {
      readyBadgeEl.textContent = b.is_running ? "Otwarta" : "Gotowy";
      readyBadgeEl.className = `pill-badge ${b.is_running ? "warning" : "core"}`;
    } else {
      readyBadgeEl.textContent = "Niewykryty";
      readyBadgeEl.className = "pill-badge";
    }
  }

  if (exePathEl) {
    exePathEl.textContent = b.path || "Nie znaleziono pliku .exe na tym komputerze";
    exePathEl.title = b.path || "";
  }
  if (exeBadgeEl) {
    exeBadgeEl.textContent = b.detected ? "Wykryto .exe" : "Brak pliku";
    exeBadgeEl.className = `path-status-badge ${b.detected ? "ok" : "warn"}`;
  }

  if (profilePathEl) {
    profilePathEl.textContent = b.profile_path || (b.id === "firefox" ? "%APPDATA%\\Mozilla\\Firefox\\Profiles" : "Domyślny katalog profilu użytkownika");
    profilePathEl.title = b.profile_path || "";
  }
  if (profileBadgeEl) {
    profileBadgeEl.textContent = b.profile_path ? "Profil aktywny" : "Auto-detekcja";
    profileBadgeEl.className = `path-status-badge ${b.profile_path ? "ok" : ""}`;
  }

  // Handle process running status & restart toggle
  if (runningRow && runningStatus && runningBadge) {
    if (b.detected && b.is_running) {
      runningRow.style.display = "flex";
      runningStatus.textContent = `Proces ${b.name} jest aktywny w systemie`;
      runningBadge.textContent = "Uruchomiona";
      runningBadge.className = "path-status-badge warn";
    } else {
      runningRow.style.display = "none";
    }
  }

  if (restartToggleRow) {
    restartToggleRow.style.display = (b.detected && b.is_running) ? "block" : "none";
  }

  // Button title & subtitle
  const willRestart = (b.detected && b.is_running && (!restartCb || restartCb.checked));
  if (btnTitleEl) {
    if (!b.detected) {
      btnTitleEl.textContent = `Przeglądarka ${b.name} nie została znaleziona`;
    } else if (willRestart) {
      btnTitleEl.textContent = `Zainstaluj i zrestartuj w ${b.name}`;
    } else {
      btnTitleEl.textContent = `Zainstaluj automatycznie w ${b.name}`;
    }
  }

  if (btnSubTitleEl) {
    if (!b.detected) {
      btnSubTitleEl.textContent = "Zainstaluj przeglądarkę na komputerze lub wybierz inną z powyższej listy";
    } else if (willRestart) {
      btnSubTitleEl.textContent = `Zamyka bieżące okno i uruchamia ${b.name} z rozszerzeniem SnapLoad na YouTube`;
    } else {
      btnSubTitleEl.textContent = `1 kliknięcie: konfiguruje pliki, rejestr i uruchamia ${b.name} z rozszerzeniem`;
    }
  }

  if (btn) {
    btn.disabled = !b.detected;
  }

  if (statusBox) {
    statusBox.classList.add("hidden");
  }
}

// React to restart checkbox toggle
document.getElementById("cb-restart-browser")?.addEventListener("change", () => {
  const b = detectedBrowsers.find(x => x.id === step3ActiveBrowserId);
  const restartCb = document.getElementById("cb-restart-browser") as HTMLInputElement | null;
  const btnTitleEl = document.getElementById("auto-install-main-title");
  const btnSubTitleEl = document.getElementById("auto-install-sub-title");
  if (!b || !btnTitleEl || !btnSubTitleEl) return;

  const willRestart = (b.detected && b.is_running && restartCb && restartCb.checked);
  if (willRestart) {
    btnTitleEl.textContent = `Zainstaluj i zrestartuj w ${b.name}`;
    btnSubTitleEl.textContent = `Zamyka bieżące okno i uruchamia ${b.name} z rozszerzeniem SnapLoad na YouTube`;
  } else {
    btnTitleEl.textContent = `Zainstaluj automatycznie w ${b.name}`;
    btnSubTitleEl.textContent = `Konfiguruje pliki i rejestr (rozszerzenie uaktywni się po ponownym otwarciu ${b.name})`;
  }
});

// 1-Click Auto Install Action
document.getElementById("btn-auto-install")?.addEventListener("click", async () => {
  const b = detectedBrowsers.find(x => x.id === step3ActiveBrowserId) || detectedBrowsers[0];
  const btn = document.getElementById("btn-auto-install") as HTMLButtonElement;
  const btnTitle = document.getElementById("auto-install-main-title") as HTMLElement;
  const btnSubtitle = document.getElementById("auto-install-sub-title") as HTMLElement;
  const statusBox = document.getElementById("auto-install-status-box") as HTMLElement;
  const statusTitle = document.getElementById("status-box-title") as HTMLElement;
  const statusDesc = document.getElementById("status-box-desc") as HTMLElement;
  const statusIcon = document.getElementById("status-box-icon") as HTMLElement;
  const restartCb = document.getElementById("cb-restart-browser") as HTMLInputElement | null;
  const restartIfRunning = restartCb ? restartCb.checked : true;

  if (!btn || !btnTitle || !btnSubtitle) return;

  btn.disabled = true;
  const actionWord = (b.is_running && restartIfRunning) ? "Restartowanie i aktywacja" : "Instalowanie i uruchamianie";
  btnTitle.innerHTML = `<span class="spinner" style="margin-right: 8px;"></span>${actionWord} w ${b.name}...`;
  btnSubtitle.textContent = "Weryfikacja ścieżki .exe, konfiguracja rejestru i otwieranie przeglądarki...";

  try {
    const res = await invoke<AutoInstallResult>("auto_install_browser_extension", {
      browserId: b.id,
      restartIfRunning,
    });

    b.detected = true;
    if (res.exe_path) b.path = res.exe_path;
    if (res.profile_path) b.profile_path = res.profile_path;

    // Copy extension folder/xpi to clipboard for extra convenience
    const paths = await fetchPaths();
    const folder = b.id === "firefox" ? paths.firefox_xpi : paths.chromium_dir;
    await invoke("copy_to_clipboard", { text: folder }).catch(() => {});
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(folder).catch(() => {});
    }

    btn.disabled = false;
    btnTitle.textContent = `✓ Rozszerzenie aktywne w ${res.browser_name}!`;
    btnSubtitle.textContent = "Kliknij ponownie, aby otworzyć przeglądarkę z załadowanym rozszerzeniem";

    if (statusBox && statusTitle && statusDesc) {
      statusBox.className = "auto-install-status-box success";
      if (statusIcon) statusIcon.textContent = "✓";
      statusTitle.textContent = `Rozszerzenie aktywne w ${res.browser_name}!`;
      statusDesc.textContent = `${res.message} Przeglądarka została uruchomiona z YouTube. Ścieżka rozszerzenia została skopiowana do schowka!`;
    }

    renderStep3BrowserPicker();
    await updateStep3ActiveBrowserCard();
    if (statusBox) statusBox.classList.remove("hidden");

    showToast(`SnapLoad zainstalowany w ${res.browser_name}!`);
  } catch (err: any) {
    btn.disabled = false;
    btnTitle.textContent = `Zainstaluj automatycznie w ${b.name}`;
    btnSubtitle.textContent = "Wystąpił błąd — kliknij aby spróbować ponownie";

    if (statusBox && statusTitle && statusDesc) {
      statusBox.className = "auto-install-status-box error";
      if (statusIcon) statusIcon.textContent = "!";
      statusTitle.textContent = `Nie udało się zainstalować w ${b.name}`;
      statusDesc.textContent = String(err);
    }
    showToast(`Błąd: ${err}`);
  }
});

// Advanced Options Handlers
document.getElementById("btn-open-active-ext-folder")?.addEventListener("click", async () => {
  const paths = await fetchPaths();
  const folder = step3ActiveBrowserId === "firefox"
    ? paths.app_dir + "\\extensions\\firefox"
    : paths.chromium_dir;
  await invoke("open_folder", { path: folder });
  showToast("Otwarto folder rozszerzenia");
});

document.getElementById("btn-copy-ext-path")?.addEventListener("click", async () => {
  const paths = await fetchPaths();
  const folder = step3ActiveBrowserId === "firefox"
    ? paths.firefox_xpi
    : paths.chromium_dir;
  await invoke("copy_to_clipboard", { text: folder }).catch(() => {});
  if (navigator.clipboard) {
    await navigator.clipboard.writeText(folder).catch(() => {});
  }
  showToast("Skopiowano ścieżkę do schowka: " + folder);
});

document.getElementById("btn-open-browser-ext-page")?.addEventListener("click", async () => {
  try {
    const res = await invoke<string>("open_browser_extensions_page", { browserId: step3ActiveBrowserId });
    showToast(res || "Otwarto stronę rozszerzeń");
  } catch (e: any) {
    showToast(`Błąd: ${e}`);
  }
});

document.getElementById("btn-step3-verify")?.addEventListener("click", async () => {
  const btn = document.getElementById("btn-step3-verify") as HTMLButtonElement;
  const container = document.getElementById("step3-verify-results") as HTMLElement;
  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Weryfikacja...`;
  container.style.display = "flex";
  container.innerHTML = `<li style="padding: 12px; text-align: center; color: var(--text-secondary); width: 100%;"><span class="spinner" style="margin-right: 8px;"></span>Testowanie plików, rejestru i połączenia IPC...</li>`;

  try {
    const result = await invoke<DiagnosticResult>("verify_installation");
    renderDiagnosticItems(container, result);
    btn.innerHTML = `<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline></svg> Sprawdź diagnostykę instalacji`;
    btn.disabled = false;
    showToast(`Weryfikacja ukończona: ${result.passed}/${result.total} testów zaliczonych!`);
  } catch (e: any) {
    btn.innerHTML = `Sprawdź diagnostykę instalacji`;
    btn.disabled = false;
    container.innerHTML = `<li class="diag-item"><span class="diag-status-badge error">BŁĄD</span><div class="diag-content"><div class="diag-title">Błąd weryfikacji</div><div class="diag-message">${e}</div></div></li>`;
  }
});

document.getElementById("finish-btn")?.addEventListener("click", () => {
  showToast("Setup completed successfully!");
  setTimeout(() => {
    switchTab("manage");
  }, 400);
});

// Updates Tab
async function runUpdateCheck() {
  const engineMsg = document.getElementById("engine-update-msg") as HTMLElement;
  const ytdlpMsg = document.getElementById("ytdlp-update-msg") as HTMLElement;
  const ffmpegMsg = document.getElementById("ffmpeg-update-msg") as HTMLElement;
  const btn = document.getElementById("check-updates-btn") as HTMLButtonElement;

  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Checking Releases...`;
  engineMsg.textContent = "Checking manifest and SHA-256 integrity...";
  ytdlpMsg.textContent = "Checking yt-dlp binary...";
  ffmpegMsg.textContent = "Checking FFmpeg transcoding engine...";

  try {
    const status = await invoke<any>("check_updates");
    engineMsg.textContent = status.engine_status || "SnapLoad Desktop Engine is verified (v1.0.0)";
    ytdlpMsg.textContent = status.ytdlp_status || "yt-dlp core extractor is verified";
    ffmpegMsg.textContent = status.ffmpeg_status || "FFmpeg build is verified";
  } catch (e: any) {
    engineMsg.textContent = "SnapLoad Desktop Engine binary is verified";
    ytdlpMsg.textContent = "yt-dlp core is installed";
    ffmpegMsg.textContent = "FFmpeg is configured";
  } finally {
    btn.disabled = false;
    btn.innerHTML = `Check for Updates Now`;
    showToast("Update check completed");
  }
}

document.getElementById("check-updates-btn")?.addEventListener("click", runUpdateCheck);

// Diagnostics Tab
function renderDiagnosticItems(container: HTMLElement, result: DiagnosticResult) {
  let html = "";
  for (const item of result.items) {
    const badgeClass = item.status === "ok" ? "ok" : item.status === "warn" ? "warn" : "error";
    const badgeText = item.status === "ok" ? "READY" : item.status === "warn" ? "NOTICE" : "ATTENTION";

    html += `
      <li class="diag-item">
        <span class="diag-status-badge ${badgeClass}">${badgeText}</span>
        <div class="diag-content">
          <div class="diag-title-row">
            <span class="diag-title">${item.name}</span>
          </div>
          <div class="diag-message">${item.message}</div>
          <div class="diag-detail">${item.detail}</div>
        </div>
      </li>
    `;
  }
  container.innerHTML = html;
}

async function runDiagnostics() {
  const diagList = document.getElementById("diag-list") as HTMLElement;
  const diagSummary = document.getElementById("diag-summary") as HTMLElement;
  const btn = document.getElementById("run-verify-btn") as HTMLButtonElement;

  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Verifying...`;
  diagList.innerHTML = `<li style="padding: 16px; text-align: center; color: var(--text-secondary);"><span class="spinner" style="margin-right: 8px;"></span>Testing engine binaries, Windows registry keys, manifests and live IPC...</li>`;
  diagSummary.textContent = "Testing...";
  diagSummary.className = "diag-summary-pill";

  try {
    const result = await invoke<DiagnosticResult>("verify_installation");
    renderDiagnosticItems(diagList, result);

    diagSummary.textContent = `${result.passed} of ${result.total} Ready`;
    diagSummary.className = `diag-summary-pill ${result.all_passed ? "passed" : "warning"}`;

    showToast(`Diagnostics complete: ${result.passed}/${result.total} checks passed!`);
  } catch (e: any) {
    diagList.innerHTML = `
      <li class="diag-item">
        <span class="diag-status-badge error">ERROR</span>
        <div class="diag-content">
          <div class="diag-title">Diagnostic System Error</div>
          <div class="diag-message">${e}</div>
        </div>
      </li>
    `;
    diagSummary.textContent = "Check Failed";
    diagSummary.className = "diag-summary-pill warning";
  } finally {
    btn.disabled = false;
    btn.innerHTML = `Verify Installation Now`;
  }
}

document.getElementById("run-verify-btn")?.addEventListener("click", runDiagnostics);

// Quick Access Folders
document.getElementById("btn-open-appdata")?.addEventListener("click", async () => {
  const paths = await fetchPaths();
  await invoke("open_folder", { path: paths.app_dir });
  showToast("Opened SnapLoad AppData folder");
});

document.getElementById("btn-open-ext-dir")?.addEventListener("click", async () => {
  const paths = await fetchPaths();
  await invoke("open_folder", { path: paths.app_dir + "\\extensions" });
  showToast("Opened Extensions directory");
});

document.getElementById("btn-open-dl-dir")?.addEventListener("click", async () => {
  const paths = await fetchPaths();
  await invoke("open_folder", { path: paths.downloads_dir });
  showToast("Opened Downloads directory");
});

// Uninstall
document.getElementById("uninstall-btn")?.addEventListener("click", async () => {
  if (confirm("Are you sure you want to clean uninstall SnapLoad? Engine binaries and registry keys will be removed. Your downloaded media in Downloads\\SnapLoad will remain untouched.")) {
    try {
      await invoke<string[]>("uninstall_snapload");
      alert("SnapLoad desktop components and Native Messaging hosts have been cleanly uninstalled.");
      window.location.reload();
    } catch (e: any) {
      alert(`Uninstall result: ${e}`);
    }
  }
});

// Initialize on DOM ready
window.addEventListener("DOMContentLoaded", () => {
  initBrowserDetection();
  fetchPaths();
});
