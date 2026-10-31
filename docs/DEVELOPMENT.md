# SnapLoad Windows — Development Guide

## 1. Prerequisites

To build and develop the SnapLoad Windows ecosystem from source, ensure the following tools are installed:

- **Rust Toolchain**: Rust 1.80+ (MSVC or MinGW 64-bit).
- **Node.js**: Node 20+ (LTS) or Node 24.
- **Package Manager**: `npm` 10+.
- **Git**: 2.40+.

---

## 2. Repository Layout

```
SnapLoad-Windows/
├── engine/              # Rust Native Messaging Host & Media Process Manager
├── extension/           # TypeScript browser extensions for Chrome & Firefox
├── installer/           # Tauri 2 Desktop Setup & Updater GUI
├── scripts/             # Build and packaging automation PowerShell scripts
├── docs/                # Architectural & technical specifications
├── LICENSE              # MIT Non-Commercial Attribution License
└── README.md            # Primary documentation & ecosystem overview
```

---

## 3. Building From Source

### A. SnapLoad Desktop Engine (Rust)
```powershell
cd engine

# Run unit & integration tests
cargo test

# Build optimized release binary
cargo build --release

# The resulting binary is located at:
# engine/target/release/snapload-engine.exe
```

### B. Browser Extensions (TypeScript)
```powershell
cd extension

# Install dev dependencies
npm install

# Run strict TypeScript typechecking
npm run typecheck

# Build both Chromium and Firefox distribution packages
npm run build

# Build artifacts are placed in:
# extension/dist/chrome/
# extension/dist/firefox/
```

### C. Setup & Management Desktop Application (Tauri 2)
```powershell
cd installer

# Install frontend dependencies
npm install

# Build frontend assets with Vite
npm run build

# Check Rust Tauri backend
cd src-tauri
cargo check
```

### D. Automated Full Build Script
To build all ecosystem targets in a single sequence:
```powershell
powershell -ExecutionPolicy Bypass -File scripts\build\build_all.ps1
```

---

## 4. Testing

### Engine Protocol Tests
The Rust engine test suite verifies:
- Native Messaging 4-byte length prefix framing and round-trip parsing.
- Rejection of oversized messages (> 1 MB limit).
- URL validation and rejection of dangerous command flag injections (`--exec`).
- Clean handling of EOF when the browser closes standard input.

Run tests:
```powershell
cd engine
cargo test
```

### Extension Typechecking
```powershell
cd extension
npm run typecheck
```
