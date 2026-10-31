# Voxflow

**A free, open source, and extensible speech-to-text application that works completely offline.**

Voxflow is a cross-platform desktop application that provides simple, privacy-focused speech transcription. Press a shortcut, speak, and have your words appear in any text field. This happens on your own computer without sending any information to the cloud.

## Why Voxflow?

- **Free**: Accessibility tooling belongs in everyone's hands, not behind a paywall
- **Open Source**: Together we can build further. Extend Voxflow for yourself and contribute to something bigger
- **Private**: Your voice stays on your computer. Get transcriptions without sending audio to the cloud
- **Simple**: One tool, one job. Transcribe what you say and put it into a text box

## How It Works

1. **Press** a configurable keyboard shortcut: hold it to record and release to stop, or tap it to toggle recording on and off (Hold-only and Toggle-only modes are also available)
2. **Speak** your words while the shortcut is active
3. **Release** and Voxflow processes your speech with a speech model that runs on your computer
4. **Get** your transcribed text pasted directly into whatever app you're using

The process is entirely local:

- Silence is filtered using VAD (Voice Activity Detection) with Silero
- Transcription uses your choice of models:
  - **Whisper models** (Small/Medium/Turbo/Large) with GPU acceleration when available
  - **Parakeet** - CPU-optimized models with excellent performance and automatic language detection
  - Many more, each shown with an accuracy grade and a speed rating so you can choose
- Works on Windows, macOS, and Linux

## Features

- **Snippets**: say a trigger phrase like "my address" and Voxflow types the saved text
- **Dictionary**: custom words, plus replacements that swap one phrase for another ("gonna" becomes "going to")
- **Writing styles**: Formal, Casual, Very casual or Technical, set as a default or per app
- **Usage stats**: words this week, speaking speed and time saved on the History page
- **Undo and corrections**: a shortcut, or say "scratch that" to undo, or fix yourself mid-sentence ("Send it Monday, scratch that, Tuesday" types "Send it Tuesday")
- **Paste by voice**: say "paste link" or "paste clipboard" while dictating to type what you copied, such as a link
- **Ignore background voices**: an option that skips speech much quieter than yours, such as a nearby conversation
- **Accuracy and speed labels** in the model picker

## Download and install

Download the latest installer for your system from the **[Installers](https://github.com/reachneoagent-commits/voxflow/releases/latest)**, then follow the steps for your system below.

Voxflow isn't signed with a paid Apple or Microsoft certificate (that costs money every year, and Voxflow is free). Your computer will warn you the first time you open it. The warning is expected, and the steps below get past it once.

### Mac

1. Download the `.dmg` that matches your Mac:
   - **`Mac_Apple-Silicon`**: Macs with an M1, M2, M3 or newer chip.
   - **`Mac_Intel`**: older Macs with an Intel chip.
2. Open the `.dmg` and drag **Voxflow** into **Applications**.
3. Open Voxflow. macOS will say it can't verify the app. Click **Done** (not Move to Trash).
4. Open the Privacy & Security settings. Paste this into Terminal, or open **System Settings → Privacy & Security** yourself:
   ```bash
   open "x-apple.systempreferences:com.apple.preference.security"
   ```
5. Scroll down to the message about Voxflow, click **Open Anyway**, and enter your password.
6. When asked, allow **Microphone** and **Accessibility** access. Voxflow needs both: one to hear you, one to type for you.

If step 5 doesn't show Open Anyway, run this once and open Voxflow again:

```bash
xattr -cr /Applications/Voxflow.app
```

After an update you may be asked to allow Microphone and Accessibility again. That's a side effect of the app being unsigned.

### Windows

1. Download the `.exe` installer (`x64-setup` for most PCs, `arm64-setup` for Windows on ARM) and run it.
2. If Windows says **"Windows protected your PC"**, click **More info**, then **Run anyway**.
3. Finish the installer and open Voxflow.

### Linux

- **Ubuntu, Debian, Mint:** download the `.deb`, then run `sudo apt install ./Voxflow_*.deb`.
- **Fedora, RHEL:** download the `.rpm`, then run `sudo dnf install ./Voxflow-*.rpm`.
- **Anything else:** download the `.AppImage`, run `chmod +x Voxflow_*.AppImage`, then double-click it.

### First launch

1. Pick a speech model when asked. Smaller models are faster; larger ones are more accurate. The picker shows an accuracy grade (A is best) and a speed rating for each.
2. Set your dictation shortcut in **General**.
3. Click into any text field, hold the shortcut, speak, and let go.

## Build it yourself

You need [Rust](https://rustup.rs/), [Bun](https://bun.sh/), and CMake. Building from source avoids the unsigned-app warning, because your own computer made the app.

```bash
bun install
mkdir -p src-tauri/resources/models
curl -L -o src-tauri/resources/models/silero-vad-v6.2-F32.gguf https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/main/silero-vad-v6.2-F32.gguf
bun run tauri build
```

On macOS, if CMake complains, prefix the build with `CMAKE_POLICY_VERSION_MINIMUM=3.5`. For development, use `bun run tauri dev`.

## Troubleshooting and more detail

Model setup, Linux notes, command-line options and known issues are in **[docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)**.

## License

MIT License. See [LICENSE](LICENSE) for details. Voxflow includes code originally written by CJ Pais (copyright 2025), used under the same license.

## Acknowledgments

- **Whisper** by OpenAI for the speech recognition model
- **ggml** for an amazing cross-platform tensor library
- **Silero** for great lightweight VAD
- **Tauri** team for the excellent Rust-based app framework
