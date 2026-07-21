# wonderfulr-d7e0-texdesk

`wonderfulr-d7e0-texdesk` is a local desktop LaTeX workspace for editing `.tex` files, compiling them with a local TeX toolchain, and previewing generated PDFs.

## Prerequisites

Install these on every development machine:

- Node.js 20+ and npm
- Rust stable
- A LaTeX toolchain:
  - `latexmk` is preferred.
  - `pdflatex` or `xelatex` are supported fallbacks.
  - `bibtex` or `biber` are optional for bibliography support.

## Linux setup: Debian/Ubuntu

1. Install system dependencies, Node.js/npm, Tauri build libraries, and LaTeX packages:

   ```bash
   sudo apt-get update
   sudo apt-get install -y \
     nodejs \
     npm \
     build-essential \
     curl \
     libwebkit2gtk-4.1-dev \
     libappindicator3-dev \
     librsvg2-dev \
     patchelf \
     texlive-latex-base \
     texlive-latex-extra \
     latexmk \
     biber
   ```

   If your distribution package repository does not provide Node.js 20+, install Node.js from <https://nodejs.org/> or a Node version manager, then confirm:

   ```bash
   node --version
   npm --version
   ```

2. Install Rust stable with rustup:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   rustup default stable
   ```

3. Install project dependencies:

   ```bash
   npm install
   ```

4. Run the desktop app in development mode:

   ```bash
   npm run tauri:dev
   ```

   For the Vite frontend only, without the Tauri desktop shell, run:

   ```bash
   npm run dev
   ```

5. Build a production desktop bundle:

   ```bash
   npm run tauri:build
   ```

   Tauri writes Linux release bundles under:

   ```text
   src-tauri/target/release/bundle/
   ```

## Windows setup

1. Install platform prerequisites:

   - Node.js LTS from <https://nodejs.org/>. Use Node.js 20+.
   - Rust from <https://rustup.rs/>. Use the stable toolchain.
   - Microsoft C++ Build Tools from <https://visualstudio.microsoft.com/visual-cpp-build-tools/>. Select the **Desktop development with C++** workload.
   - Microsoft Edge WebView2 Runtime from <https://developer.microsoft.com/en-us/microsoft-edge/webview2/>.
   - A LaTeX distribution:
     - MiKTeX: <https://miktex.org/>
     - TeX Live: <https://tug.org/texlive/>

   Make sure `latexmk`, `pdflatex`, and/or `xelatex` are available on `PATH`. `bibtex` or `biber` should also be on `PATH` if you need bibliography support.

2. Open PowerShell or Command Prompt in the repository directory and install project dependencies:

   ```powershell
   npm install
   ```

3. Run the desktop app in development mode:

   ```powershell
   npm run tauri:dev
   ```

   For the Vite frontend only, without the Tauri desktop shell, run:

   ```powershell
   npm run dev
   ```

4. Build a production desktop bundle:

   ```powershell
   npm run tauri:build
   ```

   Tauri writes Windows release bundles under:

   ```text
   src-tauri\target\release\bundle\
   ```

## Configuration

The app can read optional environment variables from the desktop process environment. Copy `.env.example` if you want a local starting point:

```bash
cp .env.example .env
```

Supported variables are:

| Variable | Purpose |
| --- | --- |
| `DEFAULT_WORKSPACE_ROOT` | Optional default workspace directory opened by the desktop app. |
| `VITE_DEFAULT_WORKSPACE_ROOT` | Vite-prefixed alternative for the default workspace directory. |
| `LATEX_TOOLCHAIN_PATH` | Optional explicit path to a LaTeX executable or toolchain directory. |
| `VITE_LATEX_TOOLCHAIN_PATH` | Vite-prefixed alternative for the LaTeX executable or toolchain directory. |

The names above are mirrored in `src-tauri/src/config.rs` and `.env.example`.

## Useful commands

| Command | Description |
| --- | --- |
| `npm run dev` | Run the Vite frontend only on `0.0.0.0:8080`. |
| `npm run build` | Type-check and build the frontend with `tsc && vite build`. |
| `npm run preview` | Preview the built frontend with Vite on `0.0.0.0:8080`. |
| `npm run tauri:dev` | Run the full Tauri desktop app in development mode. |
| `npm run tauri:build` | Build the production Tauri desktop bundle. |
| `npm test` | Run frontend tests and Rust tests. |
| `npm run test:frontend` | Run TypeScript frontend unit tests. |
| `npm run test:rust` | Run Rust tests with `cargo test --manifest-path src-tauri/Cargo.toml`. |
| `npm run test:e2e` | Run the ignored end-to-end template/edit/compile/preview PDF test. Requires a real local LaTeX install. |
