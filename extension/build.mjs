import * as esbuild from "esbuild";
import * as fs from "fs";
import * as path from "path";

const rootDir = process.cwd();
const distChrome = path.join(rootDir, "dist", "chrome");
const distFirefox = path.join(rootDir, "dist", "firefox");

function ensureDir(dir) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

function copyFile(src, dest) {
  ensureDir(path.dirname(dest));
  fs.copyFileSync(src, dest);
}

function copyDir(src, dest) {
  if (!fs.existsSync(src)) return;
  ensureDir(dest);
  const entries = fs.readdirSync(src, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDir(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

async function build() {
  console.log("🚀 Building SnapLoad Browser Extensions for Chrome & Firefox...");

  ensureDir(distChrome);
  ensureDir(distFirefox);

  // Common esbuild config
  const entryPoints = [
    { in: "src/background/index.ts", out: "background" },
    { in: "src/content/youtube.ts", out: "content/youtube" },
    { in: "src/popup/popup.ts", out: "popup/popup" },
  ];

  for (const [targetName, targetDir] of [["Chrome", distChrome], ["Firefox", distFirefox]]) {
    console.log(`📦 Bundling for ${targetName}...`);
    await esbuild.build({
      entryPoints,
      outdir: targetDir,
      bundle: true,
      minify: false,
      sourcemap: true,
      target: ["chrome100", "firefox109"],
      format: "iife",
    });

    // Copy popup static files
    copyFile(path.join(rootDir, "src/popup/popup.html"), path.join(targetDir, "popup/popup.html"));
    copyFile(path.join(rootDir, "src/popup/popup.css"), path.join(targetDir, "popup/popup.css"));

    // Copy assets
    copyDir(path.join(rootDir, "assets"), path.join(targetDir, "assets"));
  }

  // Copy target-specific manifests
  copyFile(path.join(rootDir, "manifests/manifest.chrome.json"), path.join(distChrome, "manifest.json"));
  copyFile(path.join(rootDir, "manifests/manifest.firefox.json"), path.join(distFirefox, "manifest.json"));

  console.log("✅ Build completed successfully for Chrome and Firefox!");
}

build().catch((err) => {
  console.error("❌ Build failed:", err);
  process.exit(1);
});
