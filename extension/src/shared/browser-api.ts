// Universal Browser API wrapper for Chromium (chrome.*) and Firefox (browser.*)
export const api: typeof chrome = 
  typeof (globalThis as any).browser !== "undefined" && (globalThis as any).browser.runtime
    ? (globalThis as any).browser
    : chrome;
