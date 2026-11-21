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

interface SetupUpdateInfo {
  current_version: string;
  latest_version: string;
  update_available: boolean;
  release_name: string;
  release_notes: string;
  download_url?: string;
  asset_url?: string;
  published_at?: string;
  file_size_bytes?: number;
  error?: string;
}

// Crisp Authentic Vector Brand SVGs for Browsers (Official Assets)
const BROWSER_SVGS: Record<string, string> = {
  chrome: `<svg viewBox="0 0 48 48" width="34" height="34" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="chrome-grad-a" x1="3.2173" y1="15" x2="44.7812" y2="15" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#d93025" />
      <stop offset="1" stop-color="#ea4335" />
    </linearGradient>
    <linearGradient id="chrome-grad-b" x1="20.7219" y1="47.6791" x2="41.5039" y2="11.6837" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#fcc934" />
      <stop offset="1" stop-color="#fbbc04" />
    </linearGradient>
    <linearGradient id="chrome-grad-c" x1="26.5981" y1="46.5015" x2="5.8161" y2="10.506" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#1e8e3e" />
      <stop offset="1" stop-color="#34a853" />
    </linearGradient>
  </defs>
  <circle cx="24" cy="23.9947" r="12" fill="#fff" />
  <path d="M24,12H44.7812a23.9939,23.9939,0,0,0-41.5639.0029L13.6079,30l.0093-.0024A11.9852,11.9852,0,0,1,24,12Z" fill="url(#chrome-grad-a)" />
  <circle cx="24" cy="24" r="9.5" fill="#1a73e8" />
  <path d="M34.3913,30.0029,24.0007,48A23.994,23.994,0,0,0,44.78,12.0031H23.9989l-.0025.0093A11.985,11.985,0,0,1,34.3913,30.0029Z" fill="url(#chrome-grad-b)" />
  <path d="M13.6086,30.0031,3.218,12.006A23.994,23.994,0,0,0,24.0025,48L34.3931,30.0029l-.0067-.0068a11.9852,11.9852,0,0,1-20.7778.007Z" fill="url(#chrome-grad-c)" />
</svg>`,
  edge: `<svg width="34" height="34" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><defs><radialGradient id="edge-grad-b" cx="161.8" cy="68.9" r="95.4" gradientTransform="matrix(1 0 0 -.95 0 248.8)" gradientUnits="userSpaceOnUse"><stop offset=".7" stop-opacity="0"/><stop offset=".9" stop-opacity=".5"/><stop offset="1"/></radialGradient><radialGradient id="edge-grad-d" cx="-340.3" cy="63" r="143.2" gradientTransform="matrix(.15 -.99 -.8 -.12 176.6 -125.4)" gradientUnits="userSpaceOnUse"><stop offset=".8" stop-opacity="0"/><stop offset=".9" stop-opacity=".5"/><stop offset="1"/></radialGradient><radialGradient id="edge-grad-e" cx="113.4" cy="570.2" r="202.4" gradientTransform="matrix(-.04 1 2.13 .08 -1179.5 -106.7)" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#35c1f1"/><stop offset=".1" stop-color="#34c1ed"/><stop offset=".2" stop-color="#2fc2df"/><stop offset=".3" stop-color="#2bc3d2"/><stop offset=".7" stop-color="#36c752"/></radialGradient><radialGradient id="edge-grad-f" cx="376.5" cy="568" r="97.3" gradientTransform="matrix(.28 .96 .78 -.23 -303.8 -148.5)" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#66eb6e"/><stop offset="1" stop-color="#66eb6e" stop-opacity="0"/></radialGradient><linearGradient id="edge-grad-a" x1="63.3" y1="84" x2="241.7" y2="84" gradientTransform="matrix(1 0 0 -1 0 266)" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#0c59a4"/><stop offset="1" stop-color="#114a8b"/></linearGradient><linearGradient id="edge-grad-c" x1="157.3" y1="161.4" x2="46" y2="40.1" gradientTransform="matrix(1 0 0 -1 0 266)" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1b9de2"/><stop offset=".2" stop-color="#1595df"/><stop offset=".7" stop-color="#0680d7"/><stop offset="1" stop-color="#0078d4"/></linearGradient></defs><path d="M235.7 195.5a93.7 93.7 0 0 1-10.6 4.7 101.9 101.9 0 0 1-35.9 6.4c-47.3 0-88.5-32.5-88.5-74.3a31.5 31.5 0 0 1 16.4-27.3c-42.8 1.8-53.8 46.4-53.8 72.5 0 74 68.1 81.4 82.8 81.4 7.9 0 19.8-2.3 27-4.6l1.3-.4a128.3 128.3 0 0 0 66.6-52.8 4 4 0 0 0-5.3-5.6Z" transform="translate(-4.6 -5)" style="fill:url(#edge-grad-a)"/><path d="M235.7 195.5a93.7 93.7 0 0 1-10.6 4.7 101.9 101.9 0 0 1-35.9 6.4c-47.3 0-88.5-32.5-88.5-74.3a31.5 31.5 0 0 1 16.4-27.3c-42.8 1.8-53.8 46.4-53.8 72.5 0 74 68.1 81.4 82.8 81.4 7.9 0 19.8-2.3 27-4.6l1.3-.4a128.3 128.3 0 0 0 66.6-52.8 4 4 0 0 0-5.3-5.6Z" transform="translate(-4.6 -5)" style="isolation:isolate;opacity:.35;fill:url(#edge-grad-b)"/><path d="M110.3 246.3A79.2 79.2 0 0 1 87.6 225a80.7 80.7 0 0 1 29.5-120c3.2-1.5 8.5-4.1 15.6-4a32.4 32.4 0 0 1 25.7 13 31.9 31.9 0 0 1 6.3 18.7c0-.2 24.5-79.6-80-79.6-43.9 0-80 41.6-80 78.2a130.2 130.2 0 0 0 12.1 56 128 128 0 0 0 156.4 67 75.5 75.5 0 0 1-62.8-8Z" transform="translate(-4.6 -5)" style="fill:url(#edge-grad-c)"/><path d="M110.3 246.3A79.2 79.2 0 0 1 87.6 225a80.7 80.7 0 0 1 29.5-120c3.2-1.5 8.5-4.1 15.6-4a32.4 32.4 0 0 1 25.7 13 31.9 31.9 0 0 1 6.3 18.7c0-.2 24.5-79.6-80-79.6-43.9 0-80 41.6-80 78.2a130.2 130.2 0 0 0 12.1 56 128 128 0 0 0 156.4 67 75.5 75.5 0 0 1-62.8-8Z" transform="translate(-4.6 -5)" style="opacity:.41;fill:url(#edge-grad-d);isolation:isolate"/><path d="M157 153.8c-.9 1-3.4 2.5-3.4 5.6 0 2.6 1.7 5.2 4.8 7.3 14.3 10 41.4 8.6 41.5 8.6a59.6 59.6 0 0 0 30.3-8.3 61.4 61.4 0 0 0 30.4-52.9c.3-22.4-8-37.3-11.3-43.9C228 28.8 182.3 5 132.6 5a128 128 0 0 0-128 126.2c.5-36.5 36.8-66 80-66 3.5 0 23.5.3 42 10a72.6 72.6 0 0 1 30.9 29.3c6.1 10.6 7.2 24.1 7.2 29.5s-2.7 13.3-7.8 19.9Z" transform="translate(-4.6 -5)" style="fill:url(#edge-grad-e)"/><path d="M157 153.8c-.9 1-3.4 2.5-3.4 5.6 0 2.6 1.7 5.2 4.8 7.3 14.3 10 41.4 8.6 41.5 8.6a59.6 59.6 0 0 0 30.3-8.3 61.4 61.4 0 0 0 30.4-52.9c.3-22.4-8-37.3-11.3-43.9C228 28.8 182.3 5 132.6 5a128 128 0 0 0-128 126.2c.5-36.5 36.8-66 80-66 3.5 0 23.5.3 42 10a72.6 72.6 0 0 1 30.9 29.3c6.1 10.6 7.2 24.1 7.2 29.5s-2.7 13.3-7.8 19.9Z" transform="translate(-4.6 -5)" style="fill:url(#edge-grad-f)"/></svg>`,
  brave: `<svg viewBox="0 0 24 24" width="34" height="34" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="brave-grad-a" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FB542B" />
      <stop offset="100%" stop-color="#FF2000" />
    </linearGradient>
  </defs>
  <path fill="url(#brave-grad-a)" d="M15.68 0l2.096 2.38s1.84-.512 2.709.358c.868.87 1.584 1.638 1.584 1.638l-.562 1.381.715 2.047s-2.104 7.98-2.35 8.955c-.486 1.919-.818 2.66-2.198 3.633-1.38.972-3.884 2.66-4.293 2.916-.409.256-.92.692-1.38.692-.46 0-.97-.436-1.38-.692a185.796 185.796 0 01-4.293-2.916c-1.38-.973-1.712-1.714-2.197-3.633-.247-.975-2.351-8.955-2.351-8.955l.715-2.047-.562-1.381s.716-.768 1.585-1.638c.868-.87 2.708-.358 2.708-.358L8.321 0h7.36zm-3.679 14.936c-.14 0-1.038.317-1.758.69-.72.373-1.242.637-1.409.742-.167.104-.065.301.087.409.152.107 2.194 1.69 2.393 1.866.198.175.489.464.687.464.198 0 .49-.29.688-.464.198-.175 2.24-1.759 2.392-1.866.152-.108.254-.305.087-.41-.167-.104-.689-.368-1.41-.741-.72-.373-1.617-.69-1.757-.69zm0-11.278s-.409.001-1.022.206-1.278.46-1.584.46c-.307 0-2.581-.434-2.581-.434S4.119 7.152 4.119 7.849c0 .697.339.881.68 1.243l2.02 2.149c.192.203.59.511.356 1.066-.235.555-.58 1.26-.196 1.977.384.716 1.042 1.194 1.464 1.115.421-.08 1.412-.598 1.776-.834.364-.237 1.518-1.19 1.518-1.554 0-.365-1.193-1.02-1.413-1.168-.22-.15-1.226-.725-1.247-.95-.02-.227-.012-.293.284-.851.297-.559.831-1.304.742-1.8-.089-.495-.95-.753-1.565-.986-.615-.232-1.799-.671-1.947-.74-.148-.068-.11-.133.339-.175.448-.043 1.719-.212 2.292-.052.573.16 1.552.403 1.632.532.079.13.149.134.067.579-.081.445-.5 2.581-.541 2.96-.04.38-.12.63.288.724.409.094 1.097.256 1.333.256s.924-.162 1.333-.256c.408-.093.329-.344.288-.723-.04-.38-.46-2.516-.541-2.961-.082-.445-.012-.45.067-.579.08-.129 1.059-.372 1.632-.532.573-.16 1.845.009 2.292.052.449.042.487.107.339.175-.148.069-1.332.508-1.947.74-.615.233-1.476.49-1.565.986-.09.496.445 1.241.742 1.8.297.558.304.624.284.85-.02.226-1.026.802-1.247.95-.22.15-1.413.804-1.413 1.169 0 .364 1.154 1.317 1.518 1.554.364.236 1.355.755 1.776.834.422.079 1.08-.4 1.464-1.115.384-.716.039-1.422-.195-1.977-.235-.555.163-.863.355-1.066l2.02-2.149c.341-.362.68-.546.68-1.243 0-.697-2.695-3.96-2.695-3.96s-2.274.436-2.58.436c-.307 0-.972-.256-1.585-.461-.613-.205-1.022-.206-1.022-.206z"/>
</svg>`,
  opera: `<svg viewBox="0 0 128 128" width="34" height="34" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="opera-grad-a" gradientUnits="userSpaceOnUse" x1="53.327" y1="2.095" x2="53.327" y2="126.143">
      <stop offset="0" stop-color="#FF1B2D"/>
      <stop offset=".614" stop-color="#FF1B2D"/>
      <stop offset="1" stop-color="#A70014"/>
    </linearGradient>
    <linearGradient id="opera-grad-b" gradientUnits="userSpaceOnUse" x1="85.463" y1="9.408" x2="85.463" y2="119.121">
      <stop offset="0" stop-color="#9C0000"/>
      <stop offset=".7" stop-color="#FF4B4B"/>
    </linearGradient>
  </defs>
  <path d="M63.996.008C28.652.008 0 28.66 0 64.008c0 34.32 27.02 62.332 60.949 63.922 1.012.047 2.027.074 3.047.074a63.77 63.77 0 0042.652-16.285c-7.5 4.973-16.273 7.836-25.645 7.836-15.242 0-28.891-7.562-38.07-19.484-7.078-8.352-11.66-20.699-11.973-34.559V62.5c.313-13.859 4.895-26.207 11.973-34.559C52.113 16.016 65.762 8.457 81 8.457c9.375 0 18.148 2.863 25.652 7.84C95.383 6.219 80.531.07 64.238.008h-.242zm0 0" fill="url(#opera-grad-a)"/>
  <path d="M42.934 27.945c5.871-6.934 13.457-11.117 21.742-11.117 18.633 0 33.734 21.125 33.734 47.18s-15.102 47.18-33.734 47.18c-8.285 0-15.871-4.18-21.742-11.113 9.18 11.926 22.828 19.484 38.07 19.484 9.375 0 18.145-2.863 25.645-7.836 13.102-11.719 21.348-28.754 21.348-47.715s-8.246-35.988-21.344-47.707c-7.5-4.977-16.273-7.84-25.648-7.84-15.242 0-28.891 7.562-38.07 19.484" fill="url(#opera-grad-b)"/>
</svg>`,
  operagx: `<svg viewBox="0 0 24 24" width="34" height="34" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="operagx-grad-a" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FF1A4E"/>
      <stop offset="100%" stop-color="#A50028"/>
    </linearGradient>
  </defs>
  <path fill="url(#operagx-grad-a)" d="M24 12.01c0 3.412-1.462 6.674-4.013 8.943A12 12 0 0 1 11.992 24c-.178 0-.385-.019-.563-.019C5.016 23.681 0 18.403 0 11.991 0 5.37 5.372 0 11.99 0h.038c2.944 0 5.775 1.087 7.96 3.047A12.04 12.04 0 0 1 24 12.009zm-9.14 10.443c-2.729-.056-5.279-1.397-7.041-3.684-1.388-1.64-2.203-4.013-2.269-6.516v-.544c.047-2.521.881-4.893 2.269-6.534 1.744-2.269 4.294-3.61 7.012-3.666a10.67 10.67 0 0 0-2.784-.384h-.038c-5.99 0-10.865 4.894-10.865 10.866 0 5.812 4.528 10.584 10.34 10.847a10.087 10.087 0 0 0 3.375-.385zm7.996-10.444c0-3.065-1.303-5.99-3.59-8.053a7.69 7.69 0 0 0-4.228-1.284c-2.447 0-4.754 1.181-6.357 3.244-1.247 1.462-1.969 3.59-2.025 5.83v.526c.056 2.25.778 4.369 2.006 5.812l.02.02c1.602 2.071 3.909 3.252 6.355 3.252a7.69 7.69 0 0 0 4.229-1.284 10.894 10.894 0 0 0 3.59-8.063zm-3.75 0c0 4.05-2.184 7.341-4.912 7.585 2.128-1.388 3.61-4.257 3.61-7.585 0-3.328-1.463-6.197-3.61-7.575 2.728.244 4.912 3.525 4.912 7.575z"/>
</svg>`,
  firefox: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="34" height="34"><defs><radialGradient id="ff-grad-g" cx="210%" cy="-100%" r="290%"><stop offset=".1" stop-color="#ffe226"/><stop offset=".79" stop-color="#ff7139"/></radialGradient><radialGradient id="ff-grad-c" cx="49%" cy="40%" r="128%" gradientTransform="matrix(.82 0 0 1 .088 0)"><stop offset=".3" stop-color="#960e18"/><stop offset=".35" stop-color="#b11927" stop-opacity=".74"/><stop offset=".43" stop-color="#db293d" stop-opacity=".34"/><stop offset=".5" stop-color="#f5334b" stop-opacity=".09"/><stop offset=".53" stop-color="#ff3750" stop-opacity="0"/></radialGradient><radialGradient id="ff-grad-d" cx="48%" cy="-12%" r="140%"><stop offset=".13" stop-color="#fff44f"/><stop offset=".53" stop-color="#ff980e"/></radialGradient><radialGradient id="ff-grad-e" cx="22.76%" cy="110.11%" r="100%"><stop offset=".35" stop-color="#3a8ee6"/><stop offset=".67" stop-color="#9059ff"/><stop offset="1" stop-color="#c139e6"/></radialGradient><radialGradient id="ff-grad-f" cx="52%" cy="33%" r="59%" gradientTransform="scale(.9 1)"><stop offset=".21" stop-color="#9059ff" stop-opacity="0"/><stop offset=".97" stop-color="#6e008b" stop-opacity=".6"/></radialGradient><radialGradient id="ff-grad-b" cx="87.4%" cy="-12.9%" r="128%" gradientTransform="matrix(.8 0 0 1 .178 .129)"><stop offset=".13" stop-color="#ffbd4f"/><stop offset=".28" stop-color="#ff980e"/><stop offset=".47" stop-color="#ff3750"/><stop offset=".78" stop-color="#eb0878"/><stop offset=".86" stop-color="#e50080"/></radialGradient><radialGradient id="ff-grad-h" cx="84%" cy="-41%" r="180%"><stop offset=".11" stop-color="#fff44f"/><stop offset=".46" stop-color="#ff980e"/><stop offset=".72" stop-color="#ff3647"/><stop offset=".9" stop-color="#e31587"/></radialGradient><radialGradient id="ff-grad-i" cx="16.1%" cy="-18.6%" r="348.8%" gradientTransform="scale(1 .47) rotate(84 .279 -.297)"><stop offset="0" stop-color="#fff44f"/><stop offset=".3" stop-color="#ff980e"/><stop offset=".57" stop-color="#ff3647"/><stop offset=".74" stop-color="#e31587"/></radialGradient><radialGradient id="ff-grad-j" cx="18.9%" cy="-42.5%" r="238.4%"><stop offset=".14" stop-color="#fff44f"/><stop offset=".48" stop-color="#ff980e"/><stop offset=".66" stop-color="#ff3647"/><stop offset=".9" stop-color="#e31587"/></radialGradient><radialGradient id="ff-grad-k" cx="159.3%" cy="-44.72%" r="313.1%"><stop offset=".09" stop-color="#fff44f"/><stop offset=".63" stop-color="#ff980e"/></radialGradient><linearGradient id="ff-grad-a" x1="87.25%" y1="15.5%" x2="9.4%" y2="93.1%"><stop offset=".05" stop-color="#fff44f"/><stop offset=".37" stop-color="#ff980e"/><stop offset=".53" stop-color="#ff3647"/><stop offset=".7" stop-color="#e31587"/></linearGradient><linearGradient id="ff-grad-l" x1="80%" y1="14%" x2="18%" y2="84%"><stop offset=".17" stop-color="#fff44f" stop-opacity=".8"/><stop offset=".6" stop-color="#fff44f" stop-opacity="0"/></linearGradient></defs><path d="M478.711 166.353c-10.445-25.124-31.6-52.248-48.212-60.821 13.52 26.505 21.345 53.093 24.335 72.936 0 .039.015.136.047.4C427.706 111.135 381.627 83.823 344 24.355c-1.9-3.007-3.805-6.022-5.661-9.2a73.716 73.716 0 01-2.646-4.972A43.7 43.7 0 01332.1.677a.626.626 0 00-.546-.644.818.818 0 00-.451 0c-.034.012-.084.051-.12.065-.053.021-.12.069-.176.1.027-.036.083-.117.1-.136-60.37 35.356-80.85 100.761-82.732 133.484a120.249 120.249 0 00-66.142 25.488 71.355 71.355 0 00-6.225-4.7 111.338 111.338 0 01-.674-58.732c-24.688 11.241-43.89 29.01-57.85 44.7h-.111c-9.527-12.067-8.855-51.873-8.312-60.184-.114-.515-7.107 3.63-8.023 4.255a175.073 175.073 0 00-23.486 20.12 210.478 210.478 0 00-22.442 26.913c0 .012-.007.026-.011.038 0-.013.007-.026.011-.038a202.838 202.838 0 00-32.247 72.805c-.115.521-.212 1.061-.324 1.586-.452 2.116-2.08 12.7-2.365 15-.022.177-.032.347-.053.524a229.066 229.066 0 00-3.9 33.157c0 .41-.025.816-.025 1.227C16 388.418 123.6 496 256.324 496c118.865 0 217.56-86.288 236.882-199.63.407-3.076.733-6.168 1.092-9.271 4.777-41.21-.53-84.525-15.587-120.746zM201.716 354.447c1.124.537 2.18 1.124 3.334 1.639.048.033.114.07.163.1a126.191 126.191 0 01-3.497-1.739zm55.053-144.93zm198.131-30.59l-.032-.233c.012.085.027.174.04.259z" fill="url(#ff-grad-a)"/><path d="M478.711 166.353c-10.445-25.124-31.6-52.248-48.212-60.821 13.52 26.505 21.345 53.093 24.335 72.936 0-.058.011.048.036.226.012.085.027.174.04.259 22.675 61.47 10.322 123.978-7.479 162.175-27.539 59.1-94.215 119.67-198.576 116.716C136.1 454.651 36.766 370.988 18.223 261.41c-3.379-17.28 0-26.054 1.7-40.084-2.071 10.816-2.86 13.94-3.9 33.157 0 .41-.025.816-.025 1.227C16 388.418 123.6 496 256.324 496c118.865 0 217.56-86.288 236.882-199.63.407-3.076.733-6.168 1.092-9.271 4.777-41.21-.53-84.525-15.587-120.746z" fill="url(#ff-grad-b)"/><path d="M478.711 166.353c-10.445-25.124-31.6-52.248-48.212-60.821 13.52 26.505 21.345 53.093 24.335 72.936 0-.058.011.048.036.226.012.085.027.174.04.259 22.675 61.47 10.322 123.978-7.479 162.175-27.539 59.1-94.215 119.67-198.576 116.716C136.1 454.651 36.766 370.988 18.223 261.41c-3.379-17.28 0-26.054 1.7-40.084-2.071 10.816-2.86 13.94-3.9 33.157 0 .41-.025.816-.025 1.227C16 388.418 123.6 496 256.324 496c118.865 0 217.56-86.288 236.882-199.63.407-3.076.733-6.168 1.092-9.271 4.777-41.21-.53-84.525-15.587-120.746z" fill="url(#ff-grad-c)"/><path d="M361.922 194.6c.524.368 1 .734 1.493 1.1a130.706 130.706 0 00-22.31-29.112C266.4 91.892 321.516 4.626 330.811.194c.027-.036.083-.117.1-.136-60.37 35.356-80.85 100.761-82.732 133.484 2.8-.194 5.592-.429 8.442-.429 45.051 0 84.289 24.77 105.301 61.487z" fill="url(#ff-grad-d)"/><path d="M256.772 209.514c-.393 5.978-21.514 26.593-28.9 26.593-68.339 0-79.432 41.335-79.432 41.335 3.027 34.81 27.261 63.475 56.611 78.643 1.339.692 2.694 1.317 4.05 1.935a132.768 132.768 0 007.059 2.886 106.743 106.743 0 0031.271 6.031c119.78 5.618 142.986-143.194 56.545-186.408 22.137-3.85 45.115 5.053 57.947 14.067-21.012-36.714-60.25-61.484-105.3-61.484-2.85 0-5.641.235-8.442.429a120.249 120.249 0 00-66.142 25.488c3.664 3.1 7.8 7.244 16.514 15.828 16.302 16.067 58.13 32.705 58.219 34.657z" fill="url(#ff-grad-e)"/><path d="M256.772 209.514c-.393 5.978-21.514 26.593-28.9 26.593-68.339 0-79.432 41.335-79.432 41.335 3.027 34.81 27.261 63.475 56.611 78.643 1.339.692 2.694 1.317 4.05 1.935a132.768 132.768 0 007.059 2.886 106.743 106.743 0 0031.271 6.031c119.78 5.618 142.986-143.194 56.545-186.408 22.137-3.85 45.115 5.053 57.947 14.067-21.012-36.714-60.25-61.484-105.3-61.484-2.85 0-5.641.235-8.442.429a120.249 120.249 0 00-66.142 25.488c3.664 3.1 7.8 7.244 16.514 15.828 16.302 16.067 58.13 32.705 58.219 34.657z" fill="url(#ff-grad-f)"/><path d="M170.829 151.036a244.042 244.042 0 014.981 3.3 111.338 111.338 0 01-.674-58.732c-24.688 11.241-43.89 29.01-57.85 44.7 1.155-.033 36.014-.66 53.543 10.732z" fill="url(#ff-grad-g)"/><path d="M18.223 261.41C36.766 370.988 136.1 454.651 248.855 457.844c104.361 2.954 171.037-57.62 198.576-116.716 17.8-38.2 30.154-100.7 7.479-162.175l-.008-.026-.032-.233c-.025-.178-.04-.284-.036-.226 0 .039.015.136.047.4 8.524 55.661-19.79 109.584-64.051 146.044l-.133.313c-86.245 70.223-168.774 42.368-185.484 30.966a144.108 144.108 0 01-3.5-1.743c-50.282-24.029-71.054-69.838-66.6-109.124-42.457 0-56.934-35.809-56.934-35.809s38.119-27.179 88.358-3.541c46.53 21.893 90.228 3.543 90.233 3.541-.089-1.952-41.917-18.59-58.223-34.656-8.713-8.584-12.85-12.723-16.514-15.828a71.355 71.355 0 00-6.225-4.7 282.929 282.929 0 00-4.981-3.3c-17.528-11.392-52.388-10.765-53.543-10.735h-.111c-9.527-12.067-8.855-51.873-8.312-60.184-.114-.515-7.107 3.63-8.023 4.255a175.073 175.073 0 00-23.486 20.12 210.478 210.478 0 00-22.442 26.919c0 .012-.007.026-.011.038 0-.013.007-.026.011-.038a202.838 202.838 0 00-32.247 72.805c-.115.521-8.65 37.842-4.44 57.199z" fill="url(#ff-grad-h)"/><path d="M341.105 166.587a130.706 130.706 0 0122.31 29.112c1.323.994 2.559 1.985 3.608 2.952 54.482 50.2 25.936 121.2 23.807 126.26 44.261-36.46 72.575-90.383 64.051-146.044C427.706 111.135 381.627 83.823 344 24.355c-1.9-3.007-3.805-6.022-5.661-9.2a73.716 73.716 0 01-2.646-4.972A43.7 43.7 0 01332.1.677a.626.626 0 00-.546-.644.818.818 0 00-.451 0c-.034.012-.084.051-.12.065-.053.021-.12.069-.176.1-9.291 4.428-64.407 91.694 10.298 166.389z" fill="url(#ff-grad-i)"/><path d="M367.023 198.651c-1.049-.967-2.285-1.958-3.608-2.952-.489-.368-.969-.734-1.493-1.1-12.832-9.014-35.81-17.917-57.947-14.067 86.441 43.214 63.235 192.026-56.545 186.408a106.743 106.743 0 01-31.271-6.031 134.51 134.51 0 01-7.059-2.886c-1.356-.618-2.711-1.243-4.05-1.935.048.033.114.07.163.1 16.71 11.4 99.239 39.257 185.484-30.966l.133-.313c2.129-5.054 30.675-76.057-23.807-126.258z" fill="url(#ff-grad-j)"/><path d="M148.439 277.443s11.093-41.335 79.432-41.335c7.388 0 28.509-20.615 28.9-26.593s-43.7 18.352-90.233-3.541c-50.239-23.638-88.358 3.541-88.358 3.541s14.477 35.809 56.934 35.809c-4.453 39.286 16.319 85.1 66.6 109.124 1.124.537 2.18 1.124 3.334 1.639-29.348-15.169-53.582-43.834-56.609-78.644z" fill="url(#ff-grad-k)"/><path d="M478.711 166.353c-10.445-25.124-31.6-52.248-48.212-60.821 13.52 26.505 21.345 53.093 24.335 72.936 0 .039.015.136.047.4C427.706 111.135 381.627 83.823 344 24.355c-1.9-3.007-3.805-6.022-5.661-9.2a73.716 73.716 0 01-2.646-4.972A43.7 43.7 0 01332.1.677a.626.626 0 00-.546-.644.818.818 0 00-.451 0c-.034.012-.084.051-.12.065-.053.021-.12.069-.176.1.027-.036.083-.117.1-.136-60.37 35.356-80.85 100.761-82.732 133.484 2.8-.194 5.592-.429 8.442-.429 45.053 0 84.291 24.77 105.3 61.484-12.832-9.014-35.81-17.917-57.947-14.067 86.441 43.214 63.235 192.026-56.545 186.408a106.743 106.743 0 01-31.271-6.031 134.51 134.51 0 01-7.059-2.886c-1.356-.618-2.711-1.243-4.05-1.935.048.033.114.07.163.1a144.108 144.108 0 01-3.5-1.743c1.124.537 2.18 1.124 3.334 1.639-29.35-15.168-53.584-43.833-56.611-78.643 0 0 11.093-41.335 79.432-41.335 7.388 0 28.509-20.615 28.9-26.593-.089-1.952-41.917-18.59-58.223-34.656-8.713-8.584-12.85-12.723-16.514-15.828a71.355 71.355 0 00-6.225-4.7 111.338 111.338 0 01-.674-58.732c-24.688 11.241-43.89 29.01-57.85 44.7h-.111c-9.527-12.067-8.855-51.873-8.312-60.184-.114-.515-7.107 3.63-8.023 4.255a175.073 175.073 0 00-23.486 20.12 210.478 210.478 0 00-22.435 26.916c0 .012-.007.026-.011.038 0-.013.007-.026.011-.038a202.838 202.838 0 00-32.247 72.805c-.115.521-.212 1.061-.324 1.586-.452 2.116-2.486 12.853-2.77 15.156-.022.177.021-.176 0 0a279.565 279.565 0 00-3.544 33.53c0 .41-.025.816-.025 1.227C16 388.418 123.6 496 256.324 496c118.865 0 217.56-86.288 236.882-199.63.407-3.076.733-6.168 1.092-9.271 4.777-41.21-.53-84.525-15.587-120.746zm-23.841 12.341c.012.085.027.174.04.259l-.008-.026-.032-.233z" fill="url(#ff-grad-l)"/></svg>`,
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
      downloads_dir: "C:\\Users\\User\\Downloads",
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
    countEl.textContent = `${detectedCount} detected`;
  }

  pickerEl.innerHTML = "";
  for (const b of detectedBrowsers) {
    const tab = document.createElement("button");
    tab.className = `step3-browser-tab ${b.id === step3ActiveBrowserId ? "active" : ""} ${b.detected ? "detected" : ""}`;
    const svgIcon = BROWSER_SVGS[b.id] || BROWSER_SVGS["chrome"];

    tab.innerHTML = `
      <div class="tab-icon">${svgIcon}</div>
      <span class="tab-name">${b.name}</span>
      <span class="tab-dot ${b.detected ? "detected" : ""}" title="${b.detected ? "Detected on system" : "Not detected"}"></span>
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
        ? "Detected on system • Browser is currently open"
        : "Detected on system • Ready for auto-install";
    } else {
      statusEl.textContent = "Not detected at default path on this computer";
    }
  }

  if (readyBadgeEl) {
    if (b.detected) {
      readyBadgeEl.textContent = b.is_running ? "Running" : "Ready";
      readyBadgeEl.className = `pill-badge ${b.is_running ? "warning" : "core"}`;
    } else {
      readyBadgeEl.textContent = "Not Found";
      readyBadgeEl.className = "pill-badge";
    }
  }

  if (exePathEl) {
    exePathEl.textContent = b.path || "Executable (.exe) not found on this computer";
    exePathEl.title = b.path || "";
  }
  if (exeBadgeEl) {
    exeBadgeEl.textContent = b.detected ? ".exe Detected" : "Missing";
    exeBadgeEl.className = `path-status-badge ${b.detected ? "ok" : "warn"}`;
  }

  if (profilePathEl) {
    profilePathEl.textContent = b.profile_path || (b.id === "firefox" ? "%APPDATA%\\Mozilla\\Firefox\\Profiles" : "Default user profile directory");
    profilePathEl.title = b.profile_path || "";
  }
  if (profileBadgeEl) {
    profileBadgeEl.textContent = b.profile_path ? "Profile active" : "Auto-detect";
    profileBadgeEl.className = `path-status-badge ${b.profile_path ? "ok" : ""}`;
  }

  // Handle process running status & restart toggle
  if (runningRow && runningStatus && runningBadge) {
    if (b.detected && b.is_running) {
      runningRow.style.display = "flex";
      runningStatus.textContent = `${b.name} process is currently active`;
      runningBadge.textContent = "Running";
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
      btnTitleEl.textContent = `${b.name} was not detected`;
    } else if (willRestart) {
      btnTitleEl.textContent = `Install & Restart in ${b.name}`;
    } else {
      btnTitleEl.textContent = `Auto-Install into ${b.name}`;
    }
  }

  if (btnSubTitleEl) {
    if (!b.detected) {
      btnSubTitleEl.textContent = "Install browser on this computer or select another from the list above";
    } else if (willRestart) {
      btnSubTitleEl.textContent = `Closes active window and launches ${b.name} with SnapLoad`;
    } else {
      btnSubTitleEl.textContent = `1-Click: deploys files, registers native host, and launches ${b.name}`;
    }
  }

  if (btn) {
    btn.disabled = !b.detected;
  }

  if (statusBox) {
    statusBox.classList.add("hidden");
  }

  // Guide card dynamic updates
  const guideBrowserTag = document.getElementById("guide-browser-tag");
  const guideBrowserDesc = document.getElementById("guide-browser-desc");
  const guideExtUrlCode = document.getElementById("guide-ext-url-code");
  const step1Title = document.getElementById("step1-title");
  const step1Desc = document.getElementById("step1-desc");
  const step2Title = document.getElementById("step2-title");
  const step2Desc = document.getElementById("step2-desc");
  const step3Title = document.getElementById("step3-title");
  const step3Desc = document.getElementById("step3-desc");

  const extUrl = b.id === "firefox"
    ? "about:addons"
    : (b.id === "opera" || b.id === "operagx")
    ? "opera://extensions"
    : b.id === "edge"
    ? "edge://extensions"
    : b.id === "brave"
    ? "brave://extensions"
    : "chrome://extensions";

  if (guideBrowserTag) guideBrowserTag.textContent = b.name;
  if (guideExtUrlCode) guideExtUrlCode.textContent = extUrl;

  if (b.id === "firefox") {
    if (guideBrowserDesc) guideBrowserDesc.textContent = "Firefox prompts to add the SnapLoad extension automatically:";
    if (step1Title) step1Title.textContent = "Click Auto-Install";
    if (step1Desc) step1Desc.textContent = "Firefox opens automatically with the extension installation prompt.";
    if (step2Title) step2Title.textContent = "Click 'Add'";
    if (step2Desc) step2Desc.textContent = "Click 'Add' in the Firefox pop-up to authorize SnapLoad.";
    if (step3Title) step3Title.textContent = "Browser Ready";
    if (step3Desc) step3Desc.textContent = "SnapLoad extension is installed and ready for media downloads.";
  } else {
    if (guideBrowserDesc) guideBrowserDesc.innerHTML = `<strong>Important:</strong> To ensure the extension stays permanently in ${b.name} (even after restarting the browser), follow these 3 simple steps once (the path is already copied to your clipboard!):`;
    if (step1Title) step1Title.textContent = "1. Extensions Tab";
    if (step1Desc) step1Desc.innerHTML = `Open the extensions page in your browser (<code>${extUrl}</code>).`;
    if (step2Title) step2Title.textContent = "2. Developer Mode";
    if (step2Desc) step2Desc.innerHTML = `Switch the <em>Developer mode</em> toggle in the top-right corner to <strong>ON</strong>.`;
    if (step3Title) step3Title.textContent = "3. Load Unpacked & Paste";
    if (step3Desc) step3Desc.innerHTML = `Click <em>Load unpacked</em>, press <kbd>Ctrl+V</kbd> in the folder selection dialog, and confirm. Done!`;
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
    btnTitleEl.textContent = `Install & Restart in ${b.name}`;
    btnSubTitleEl.textContent = `Closes active window and launches ${b.name} with SnapLoad`;
  } else {
    btnTitleEl.textContent = `Auto-Install into ${b.name}`;
    btnSubTitleEl.textContent = `Registers host and deploys files (active upon relaunching ${b.name})`;
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
  const actionWord = (b.is_running && restartIfRunning) ? "Restarting & Launching" : "Configuring & Launching";
  btnTitle.innerHTML = `<span class="spinner" style="margin-right: 8px;"></span>${actionWord} in ${b.name}...`;
  btnSubtitle.textContent = "Verifying binary path, registering Native Host, and launching browser...";

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
    btnTitle.textContent = `✓ Extension Configured for ${res.browser_name}!`;
    btnSubtitle.textContent = "Integration ready • Profile preferences configured";

    if (statusBox && statusTitle && statusDesc) {
      statusBox.className = "auto-install-status-box success";
      if (statusIcon) statusIcon.textContent = "✓";
      statusTitle.textContent = `Configured ${res.browser_name} for SnapLoad!`;
      statusDesc.innerHTML = `${res.message}<br><br>💡 <strong>Permanent Setup Tip:</strong> In your browser's extensions tab (use the button below to open), ensure <em>Developer mode</em> is enabled and click <em>Load unpacked</em> (paste path with <kbd>Ctrl+V</kbd>) so the extension remains active permanently after restarts!`;
    }

    renderStep3BrowserPicker();
    await updateStep3ActiveBrowserCard();
    if (statusBox) statusBox.classList.remove("hidden");

    showToast(`SnapLoad configured for ${res.browser_name}!`);
  } catch (err: any) {
    btn.disabled = false;
    btnTitle.textContent = `Auto-Install into ${b.name}`;
    btnSubtitle.textContent = "An error occurred — click to retry";

    if (statusBox && statusTitle && statusDesc) {
      statusBox.className = "auto-install-status-box error";
      if (statusIcon) statusIcon.textContent = "!";
      statusTitle.textContent = `Failed to configure ${b.name}`;
      statusDesc.textContent = String(err);
    }
    showToast(`Error: ${err}`);
  }
});

// Advanced Options Handlers
document.getElementById("btn-open-active-ext-folder")?.addEventListener("click", async () => {
  const paths = await fetchPaths();
  const folder = step3ActiveBrowserId === "firefox"
    ? paths.app_dir + "\\extensions\\firefox"
    : paths.chromium_dir;
  await invoke("open_folder", { path: folder });
  showToast("Opened extension directory");
});

document.getElementById("btn-copy-ext-path")?.addEventListener("click", async (e) => {
  const target = e.currentTarget as HTMLButtonElement | null;
  const paths = await fetchPaths();
  const folder = step3ActiveBrowserId === "firefox"
    ? paths.firefox_xpi
    : paths.chromium_dir;
  await invoke("copy_to_clipboard", { text: folder }).catch(() => {});
  if (navigator.clipboard) {
    await navigator.clipboard.writeText(folder).catch(() => {});
  }
  if (target) {
    const orig = target.innerHTML;
    target.innerHTML = "✓ Copied!";
    setTimeout(() => { target.innerHTML = orig; }, 2000);
  }
  showToast("Copied extension path to clipboard");
});

document.getElementById("btn-open-browser-ext-page")?.addEventListener("click", async () => {
  try {
    const res = await invoke<string>("open_browser_extensions_page", { browserId: step3ActiveBrowserId });
    showToast(res || "Opened browser extensions page");
  } catch (e: any) {
    showToast(`Error: ${e}`);
  }
});

// Quick action buttons in Step 3 Guide Card
document.getElementById("btn-quick-open-ext-page")?.addEventListener("click", async () => {
  try {
    const res = await invoke<string>("open_browser_extensions_page", { browserId: step3ActiveBrowserId });
    showToast(res || "Opened browser extensions page");
  } catch (e: any) {
    showToast(`Error: ${e}`);
  }
});

document.getElementById("btn-quick-copy-path")?.addEventListener("click", async (e) => {
  const target = e.currentTarget as HTMLButtonElement | null;
  const paths = await fetchPaths();
  const folder = step3ActiveBrowserId === "firefox"
    ? paths.firefox_xpi
    : paths.chromium_dir;
  await invoke("copy_to_clipboard", { text: folder }).catch(() => {});
  if (navigator.clipboard) {
    await navigator.clipboard.writeText(folder).catch(() => {});
  }
  if (target) {
    const orig = target.innerHTML;
    target.innerHTML = "✓ Copied to clipboard!";
    setTimeout(() => { target.innerHTML = orig; }, 2000);
  }
  showToast("Copied extension path to clipboard");
});

document.getElementById("btn-step3-verify")?.addEventListener("click", async () => {
  const btn = document.getElementById("btn-step3-verify") as HTMLButtonElement;
  const container = document.getElementById("step3-verify-results") as HTMLElement;
  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Verifying...`;
  container.style.display = "flex";
  container.innerHTML = `<li style="padding: 12px; text-align: center; color: var(--text-secondary); width: 100%;"><span class="spinner" style="margin-right: 8px;"></span>Testing binaries, registry and IPC connection...</li>`;

  try {
    const result = await invoke<DiagnosticResult>("verify_installation");
    renderDiagnosticItems(container, result);
    btn.innerHTML = `<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline></svg> Run Diagnostics`;
    btn.disabled = false;
    showToast(`Diagnostics complete: ${result.passed}/${result.total} checks passed!`);
  } catch (e: any) {
    btn.innerHTML = `Run Diagnostics`;
    btn.disabled = false;
    container.innerHTML = `<li class="diag-item"><span class="diag-status-badge error">ERROR</span><div class="diag-content"><div class="diag-title">Verification Error</div><div class="diag-message">${e}</div></div></li>`;
  }
});

document.getElementById("finish-btn")?.addEventListener("click", () => {
  showToast("Setup completed successfully!");
  setTimeout(() => {
    switchTab("manage");
  }, 400);
});


const GITHUB_REPO_URL = "https://github.com/DarkSkyee/SnapLoad-Windows";

async function openExternalUrl(url: string) {
  try {
    await invoke("open_url", { url });
  } catch (err) {
    console.warn("open_url Tauri command not available or failed, falling back to window.open:", err);
    window.open(url, "_blank");
  }
}

type UpdateChannel = "stable" | "alpha";
let selectedUpdateChannel: UpdateChannel = "stable";

function initUpdateChannelSelector() {
  const channelBtnStable = document.getElementById("channel-btn-stable");
  const channelBtnAlpha = document.getElementById("channel-btn-alpha");
  const channelDesc = document.getElementById("channel-desc");

  const setChannel = (ch: UpdateChannel) => {
    selectedUpdateChannel = ch;
    if (channelBtnStable && channelBtnAlpha) {
      if (ch === "stable") {
        channelBtnStable.classList.add("active");
        channelBtnAlpha.classList.remove("active");
        if (channelDesc) {
          channelDesc.textContent = "Stable — tested production releases (recommended)";
        }
      } else {
        channelBtnAlpha.classList.add("active");
        channelBtnStable.classList.remove("active");
        if (channelDesc) {
          channelDesc.textContent = "Alpha — cutting-edge preview builds & pre-releases";
        }
      }
    }
    checkSetupUpdate(false, selectedUpdateChannel);
  };

  channelBtnStable?.addEventListener("click", () => setChannel("stable"));
  channelBtnAlpha?.addEventListener("click", () => setChannel("alpha"));
}

// Setup Self-Updater State & Actions
let cachedSetupUpdateInfo: SetupUpdateInfo | null = null;

async function checkSetupUpdate(silent: boolean = false, channel?: UpdateChannel): Promise<SetupUpdateInfo | null> {
  const activeChannel = channel || selectedUpdateChannel;
  const descEl = document.getElementById("setup-updater-desc") as HTMLElement;
  const btn = document.getElementById("btn-check-setup-update") as HTMLButtonElement;
  const curBadge = document.getElementById("setup-cur-ver-badge") as HTMLElement;
  const availBadge = document.getElementById("setup-update-available-badge") as HTMLElement;
  const actionBox = document.getElementById("setup-update-action-box") as HTMLElement;
  const latestVerText = document.getElementById("setup-latest-ver-text") as HTMLElement;
  const releaseDate = document.getElementById("setup-release-date") as HTMLElement;
  const releaseNotes = document.getElementById("setup-release-notes") as HTMLElement;
  const doUpdateBtn = document.getElementById("btn-do-setup-update") as HTMLButtonElement;
  const headerBadge = document.getElementById("header-version-badge") as HTMLElement;

  const statusMsg = document.getElementById("setup-update-status-msg") as HTMLElement;
  if (statusMsg) statusMsg.textContent = "";

  if (btn) {
    btn.disabled = true;
    btn.innerHTML = `<span class="spinner" style="margin-right: 6px;"></span> Checking (${activeChannel})...`;
  }
  if (descEl && !silent) descEl.textContent = `Checking GitHub for newer releases on ${activeChannel} stream...`;

  try {
    const res = await invoke<SetupUpdateInfo>("check_setup_update", { channel: activeChannel });
    cachedSetupUpdateInfo = res;

    if (curBadge) curBadge.textContent = `v${res.current_version}`;

    const splashVerChip = document.getElementById("splash-version-chip");
    if (splashVerChip && res?.current_version) {
      splashVerChip.textContent = `v${res.current_version} Setup`;
    }

    if (res.error) {
      if (availBadge) availBadge.classList.add("hidden");
      if (actionBox) actionBox.classList.add("hidden");
      const isNotFound = res.error.includes("Not Found") || res.error.includes("No published releases");
      if (descEl) {
        descEl.textContent = isNotFound
          ? `SnapLoad Setup is up to date (v${res.current_version}) on ${activeChannel} stream. You have the latest version.`
          : `Update check (${activeChannel}): ${res.error}. Local setup is v${res.current_version}.`;
      }
      if (!silent) showToast(isNotFound ? `Setup is up to date (v${res.current_version})` : `Update check: ${res.error}`);
      return res;
    }

    if (res.update_available) {
      if (availBadge) {
        availBadge.classList.remove("hidden");
        availBadge.textContent = activeChannel === "alpha" ? "Alpha Update Available!" : "Update Available!";
      }
      if (actionBox) actionBox.classList.remove("hidden");
      if (latestVerText) latestVerText.textContent = `v${res.latest_version}`;
      if (releaseDate && res.published_at) {
        const dateStr = new Date(res.published_at).toLocaleDateString();
        releaseDate.textContent = `(Released ${dateStr})`;
      }
      if (releaseNotes) {
        releaseNotes.textContent = res.release_notes || "New version release available with bug fixes and enhancements.";
      }
      const channelLabel = activeChannel === "alpha" ? " (Alpha stream)" : "";
      if (descEl) descEl.textContent = `A newer version of SnapLoad Setup (v${res.latest_version}${channelLabel}) is available on GitHub!`;
      if (doUpdateBtn) {
        doUpdateBtn.innerHTML = `<svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg> Download & Update to v${res.latest_version}`;
        doUpdateBtn.disabled = false;
      }
      if (headerBadge) {
        headerBadge.textContent = `Update v${res.latest_version} available!`;
        headerBadge.classList.add("update-pulse");
      }
      if (!silent) showToast(`New ${activeChannel} update v${res.latest_version} available!`);
    } else {
      if (availBadge) availBadge.classList.add("hidden");
      if (actionBox) actionBox.classList.add("hidden");
      const channelLabel = activeChannel === "alpha" ? " on Alpha stream" : "";
      if (descEl) descEl.textContent = `SnapLoad Setup is up to date (v${res.current_version})${channelLabel}. You have the latest version.`;
      if (headerBadge) {
        headerBadge.textContent = `v${res.current_version}`;
        headerBadge.classList.remove("update-pulse");
      }
      if (!silent) showToast(`Setup is up to date (v${res.current_version})`);
    }

    return res;
  } catch (err: any) {
    if (descEl) descEl.textContent = `Unable to connect to GitHub update service: ${err}`;
    if (!silent) showToast(`Error checking updates: ${err}`);
    return null;
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.innerHTML = `<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><polyline points="23 4 23 10 17 10"></polyline><polyline points="1 20 1 14 7 14"></polyline><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path></svg> Check Setup Update`;
    }
  }
}

async function doDownloadAndLaunchSetupUpdate() {
  const doUpdateBtn = document.getElementById("btn-do-setup-update") as HTMLButtonElement;
  const statusMsg = document.getElementById("setup-update-status-msg") as HTMLElement;

  if (doUpdateBtn) {
    doUpdateBtn.disabled = true;
    doUpdateBtn.innerHTML = `<span class="spinner" style="margin-right: 8px;"></span> Downloading Setup Update...`;
  }
  if (statusMsg) {
    statusMsg.textContent = "Downloading snapload-setup.exe from GitHub releases...";
  }

  try {
    const downloadUrl = cachedSetupUpdateInfo?.download_url || "https://github.com/DarkSkyee/snapload-windows/releases/latest/download/snapload-setup.exe";
    const assetUrl = cachedSetupUpdateInfo?.asset_url;
    const tag = cachedSetupUpdateInfo ? `v${cachedSetupUpdateInfo.latest_version}` : "v1.0.0";

    await invoke<string>("download_and_launch_setup_update", {
      downloadUrl,
      assetUrl,
      tag,
    });

    if (statusMsg) {
      statusMsg.textContent = "Update downloaded! Launching updated setup...";
    }
    showToast("Setup update downloaded! Launching new version...");

    // Wait 1.2 seconds and exit current setup
    setTimeout(async () => {
      await invoke("exit_app").catch(() => {});
    }, 1200);
  } catch (err: any) {
    if (doUpdateBtn) {
      doUpdateBtn.disabled = false;
      doUpdateBtn.innerHTML = `Retry Setup Download`;
    }
    if (statusMsg) {
      statusMsg.textContent = `Download error: ${err}`;
    }
    showToast(`Update error: ${err}`);
  }
}

document.getElementById("btn-check-setup-update")?.addEventListener("click", () => {
  checkSetupUpdate(false, selectedUpdateChannel);
});

document.getElementById("btn-do-setup-update")?.addEventListener("click", () => {
  doDownloadAndLaunchSetupUpdate();
});

document.getElementById("header-version-badge")?.addEventListener("click", () => {
  switchTab("updates");
  checkSetupUpdate(false, selectedUpdateChannel);
});

// Updates Tab
async function runUpdateCheck() {
  checkSetupUpdate(true, selectedUpdateChannel);
  const engineMsg = document.getElementById("engine-update-msg") as HTMLElement;
  const ytdlpMsg = document.getElementById("ytdlp-update-msg") as HTMLElement;
  const ffmpegMsg = document.getElementById("ffmpeg-update-msg") as HTMLElement;
  const engineCurVer = document.getElementById("engine-cur-ver") as HTMLElement;
  const ytdlpCurVer = document.getElementById("ytdlp-cur-ver") as HTMLElement;
  const ffmpegCurVer = document.getElementById("ffmpeg-cur-ver") as HTMLElement;
  const btn = document.getElementById("check-updates-btn") as HTMLButtonElement;

  btn.disabled = true;
  btn.innerHTML = `<span class="spinner"></span> Checking Releases...`;
  engineMsg.textContent = "Checking manifest and SHA-256 integrity...";
  ytdlpMsg.textContent = "Checking yt-dlp binary...";
  ffmpegMsg.textContent = "Checking FFmpeg transcoding engine...";

  if (engineCurVer) {
    engineCurVer.textContent = "Checking...";
    engineCurVer.className = "pill-badge";
  }
  if (ytdlpCurVer) {
    ytdlpCurVer.textContent = "Checking...";
    ytdlpCurVer.className = "pill-badge";
  }
  if (ffmpegCurVer) {
    ffmpegCurVer.textContent = "Checking...";
    ffmpegCurVer.className = "pill-badge";
  }

  try {
    const status = await invoke<any>("check_updates");
    engineMsg.textContent = status.engine_status || "SnapLoad Desktop Engine is verified (v1.0.0)";
    ytdlpMsg.textContent = status.ytdlp_status || "yt-dlp core extractor is verified";
    ffmpegMsg.textContent = status.ffmpeg_status || "FFmpeg build is verified";

    if (engineCurVer) {
      engineCurVer.textContent = status.engine_badge || "v1.0.0";
      engineCurVer.className = `pill-badge ${status.engine_badge === "Missing" ? "warning" : "core"}`;
    }
    if (ytdlpCurVer) {
      ytdlpCurVer.textContent = status.ytdlp_badge || "Ready";
      ytdlpCurVer.className = `pill-badge ${status.ytdlp_badge === "Not Installed" ? "warning" : "core"}`;
    }
    if (ffmpegCurVer) {
      ffmpegCurVer.textContent = status.ffmpeg_badge || "Ready";
      ffmpegCurVer.className = `pill-badge ${status.ffmpeg_badge === "Optional" ? "" : "core"}`;
    }
  } catch (e: any) {
    engineMsg.textContent = "SnapLoad Desktop Engine binary is verified";
    ytdlpMsg.textContent = "yt-dlp core is installed";
    ffmpegMsg.textContent = "FFmpeg is configured";

    if (engineCurVer) {
      engineCurVer.textContent = "v1.0.0";
      engineCurVer.className = "pill-badge core";
    }
    if (ytdlpCurVer) {
      ytdlpCurVer.textContent = "Ready";
      ytdlpCurVer.className = "pill-badge core";
    }
    if (ffmpegCurVer) {
      ffmpegCurVer.textContent = "Ready";
      ffmpegCurVer.className = "pill-badge core";
    }
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
  if (confirm("Are you sure you want to clean uninstall SnapLoad? Engine binaries, browser extensions, FFmpeg and registry keys will be removed. Your downloaded media in Downloads will remain untouched.")) {
    const btn = document.getElementById("uninstall-btn") as HTMLButtonElement;
    if (btn) {
      btn.disabled = true;
      btn.textContent = "Uninstalling...";
    }
    try {
      await invoke<string[]>("uninstall_snapload");
      alert("SnapLoad desktop components, browser extensions, and dependencies have been cleanly uninstalled.");
      cachedPaths = null;
      await runDiagnostics();
      showToast("Clean uninstallation completed!");
    } catch (e: any) {
      alert(`Uninstall result: ${e}`);
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.textContent = "Clean Uninstall SnapLoad";
      }
    }
  }
});

// Opening Launch Splash Animation
function initLaunchSplash() {
  const splashEl = document.getElementById("launch-splash");
  const splashProgressBar = document.getElementById("splash-progress-bar");
  const splashStatusText = document.getElementById("splash-status-text");
  if (!splashEl) return;

  let dismissed = false;
  const dismiss = () => {
    if (dismissed) return;
    dismissed = true;
    window.removeEventListener("keydown", onKeyDown);
    splashEl.classList.add("fade-out");
    setTimeout(() => {
      splashEl.style.display = "none";
    }, 450);
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape" || e.key === "Enter" || e.key === " ") {
      dismiss();
    }
  };

  // Click anywhere or press key to immediately dismiss splash
  splashEl.addEventListener("click", dismiss);
  window.addEventListener("keydown", onKeyDown);

  // Progressive animation sequence (total ~1.65s)
  setTimeout(() => {
    if (splashProgressBar) splashProgressBar.style.width = "35%";
    if (splashStatusText) splashStatusText.textContent = "Initializing SnapLoad environment...";
  }, 100);

  setTimeout(() => {
    if (splashProgressBar) splashProgressBar.style.width = "75%";
    if (splashStatusText) splashStatusText.textContent = "Detecting browser integrations...";
  }, 650);

  setTimeout(() => {
    if (splashProgressBar) splashProgressBar.style.width = "100%";
    if (splashStatusText) splashStatusText.textContent = "Ready!";
  }, 1250);

  setTimeout(() => {
    dismiss();
  }, 1650);
}

// Initialize on DOM ready
window.addEventListener("DOMContentLoaded", () => {
  initLaunchSplash();
  initBrowserDetection();
  fetchPaths();
  initUpdateChannelSelector();
  checkSetupUpdate(true, selectedUpdateChannel);

  // External repository link
  document.getElementById("link-github-repo")?.addEventListener("click", (e) => {
    e.preventDefault();
    openExternalUrl(GITHUB_REPO_URL);
  });

  // Check if launched after a self-update
  invoke<boolean>("is_updated_launch")
    .then((wasUpdated) => {
      if (wasUpdated) {
        setTimeout(() => {
          showToast("SnapLoad Setup was successfully updated to the latest version!");
        }, 1800);
      }
    })
    .catch(() => {});
});
