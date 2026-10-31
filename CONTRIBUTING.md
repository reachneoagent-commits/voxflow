# Contributing to Voxflow

Thank you for helping improve Voxflow. This guide covers the basics.

## Set up

1. Install [Rust](https://rustup.rs/) and [Bun](https://bun.sh/). Platform notes, including CMake and system libraries, are in [BUILD.md](BUILD.md).
2. Clone the repository and install dependencies:

   ```bash
   git clone git@github.com:reachneoagent-commits/voxflow.git
   cd voxflow
   bun install
   ```

3. Download the voice-activity model the app needs:

   ```bash
   mkdir -p src-tauri/resources/models
   curl -L -o src-tauri/resources/models/silero-vad-v6.2-F32.gguf https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/main/silero-vad-v6.2-F32.gguf
   ```

4. Run the app in development mode:

   ```bash
   bun run tauri dev
   ```

   On macOS, if CMake complains, prefix the command with `CMAKE_POLICY_VERSION_MINIMUM=3.5`.

## Project layout

- `src/`: the React and TypeScript interface (settings pages, overlay, onboarding)
- `src-tauri/`: the Rust backend (audio, speech models, shortcuts, text output)
- `brand/`: logo, icon sources, fonts and UI mockups
- `docs/`: troubleshooting and platform notes

See [AGENTS.md](AGENTS.md) for a fuller architecture overview.

## Before you open a pull request

```bash
bun run lint
bun run format:check
bun run build
cd src-tauri && cargo test --lib
```

- Use conventional commit prefixes: `feat:`, `fix:`, `docs:`, `refactor:`, `chore:`. Say why the change is needed, not just what changed.
- Add user-facing text to `src/i18n/locales/en/translation.json` and run `bun run check:translations`. Other languages can fall back to the English text until they are translated.
- Rust: run `cargo fmt` and `cargo clippy`, and avoid `unwrap()` in production code.
- TypeScript: keep types strict and avoid `any`.

## Reporting bugs

Open an issue with your operating system, the Voxflow version (Settings → About), and what you did and expected. Attach the log file if you can; its folder is shown in Settings → About.
