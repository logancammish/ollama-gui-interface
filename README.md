<div align="center">

# Ollama GUI Interface

**A fast, configurable desktop workspace for getting more from Ollama.**

Chat with local or remote models, control reasoning, work with images, add
source-backed web search, and keep your conversations organised—all from a
native Rust application.

[![Build](https://github.com/logancammish/ollama-gui-interface/actions/workflows/rust.yml/badge.svg)](https://github.com/logancammish/ollama-gui-interface/actions/workflows/rust.yml)
[![Latest release](https://img.shields.io/github/v/release/logancammish/ollama-gui-interface?display_name=tag)](https://github.com/logancammish/ollama-gui-interface/releases/latest)
[![License](https://img.shields.io/github/license/logancammish/ollama-gui-interface)](LICENSE)
[![Rust](https://img.shields.io/badge/built_with-Rust-dca282?logo=rust)](https://www.rust-lang.org/)

[Download](https://github.com/logancammish/ollama-gui-interface/releases/latest)
·
[Browse Ollama models](https://ollama.com/search)
·
[Build from source](#build-from-source)

</div>

> [!NOTE]
> This README describes the current `main` branch (`0.5.1`). Packaged releases
> may trail the source branch; check the release notes for the exact feature set
> in a download.

## Why this app?

Ollama GUI Interface is for people who like running open models locally but want
more control than a basic chat window provides. It keeps the everyday workflow
simple while putting advanced controls—remote hosts, reasoning effort, system
prompts, token limits, streaming behaviour, storage, and filtering—within easy
reach.

It is a particularly good fit if you want to:

- connect to Ollama on another computer instead of only `localhost`;
- choose exactly how much supported models reason;
- separate disposable chats from conversations worth keeping;
- inspect images or experiment with Ollama image-generation models;
- give tool-capable models optional, source-linked access to the web; or
- tune generation and rendering without building your own Ollama client.

## Highlights in 0.5

### A smarter chat workspace

- **Capability-aware reasoning** — supported models unlock Off, Low, Medium,
  and High thinking levels. Reasoning is kept in a collapsible section so the
  final answer stays readable.
- **Polished streaming output** — responses render as Markdown, code blocks have
  one-click copy controls, and an in-progress answer can be stopped at any time.
- **Precise generation controls** — tune temperature, maximum response length
  from 512 to 65,536 tokens, and context windows from 4,096 to 262,144 tokens.
- **A theme that fits your desk** — switch between the redesigned dark and light
  interfaces and adjust text size for comfortable reading.

### Chats that work your way

- **Saved chat sidebar** — reopen, pin, unpin, and delete conversations without
  digging through files.
- **Temporary chats** — explore an idea without writing the conversation to
  saved-chat storage.
- **Flexible local storage** — see the exact chat folder and move saved chats to
  a location you choose.
- **Independent context control** — decide whether earlier messages are sent to
  the model while still keeping the visible conversation organised.

### Vision and image generation

- Attach an image with the file picker, drag and drop, or your clipboard.
- Ask a vision-capable model to describe, classify, read, or reason about it.
- Use the dedicated Images workspace with models that report Ollama's
  experimental `image` capability.
- Keep generated images in a local gallery and copy them straight to the
  clipboard.

The app checks the selected model's Ollama capabilities before exposing vision,
reasoning, or image-generation controls. Image attachments must be under 20 MB.
Experimental image generation requests a 1024 × 1024 image and also depends on
support in your Ollama runtime and operating system.

### Optional, source-backed web search

Give a compatible tool-calling model access to fresh public information through
Brave Search. The model can search, fetch relevant pages, and return clickable
sources with its answer. Live status shows what it is searching and reading.

Web access is off by default and can be set globally or toggled for the current
chat. Requests are guarded with limits on searches, pages, redirects, response
size, and tool iterations. Private/local network targets and unsupported content
types are blocked.

### More control, without more friction

- Switch between reusable system-prompt profiles.
- Install an Ollama model by name from Advanced Settings.
- Connect to a trusted Ollama server using a custom host/IP and port.
- Choose instant streaming or batch visual updates for lower rendering overhead.
- Optionally mask inappropriate output with the built-in content filter.
- Use the English interface or the experimental, machine-generated Spanish
  translation.

## Quick start

### 1. Install Ollama

Download and start [Ollama](https://ollama.com/download). On Linux, make sure its
service is running:

```bash
ollama serve
```

### 2. Install Ollama GUI Interface

Open the [latest release](https://github.com/logancammish/ollama-gui-interface/releases/latest)
and choose the asset for your operating system.

- **Windows:** use `ollama-gui-win64-installer.exe` for the standard per-user
  installation. It does not require administrator privileges.
- **Linux:** download the Linux archive when one is provided, extract it, and
  run the `ollama-gui` executable.

### 3. Choose a model and chat

The model picker automatically lists models available from the connected Ollama
server. If the list is empty, open **Settings → Advanced settings**, enter a
model name under **Install model**, and press Enter. Find model names in the
[Ollama library](https://ollama.com/search).

### Prefer the simpler classic version?

If you want a smaller, less feature-heavy interface that feels more like a
simple desktop project, try
[version 0.3.7](https://github.com/logancammish/ollama-gui-interface/releases/tag/0.3.7).
It keeps the experience more basic and may suit users who do not need the newer
web, image, storage, and workspace features.

> [!WARNING]
> **Version 0.3.7 is no longer supported or maintained.** It does not receive
> bug fixes, security updates, compatibility work, or help with new Ollama
> releases. Use the latest release for the supported experience.

## Configure the experience

Most controls live in **Settings**:

| Setting | What it controls |
|---|---|
| Model and reasoning | Active Ollama model and its supported thinking effort |
| Response and context limits | Output cap and how much conversation the model can hold |
| Temperature | Predictability versus variety |
| System prompt | Active instruction/personality profile |
| Web search | Provider, API key, and results per search |
| Chat storage | The folder containing saved conversations |
| Model conversation context | Whether earlier messages are included in the next request |
| Interface | Language, theme, and text size |

**Advanced settings** contains model installation, custom Ollama connection
details, streaming/batching controls, and content filtering.

### Custom system prompts

Prompt profiles come from [`config/defaultprompts.json`](config/defaultprompts.json).
Add a JSON key/value pair where the key is the profile name and the value is the
instruction, then restart the application:

```json
{
  "concise": "Answer directly. Prefer short explanations and concrete examples.",
  "reviewer": "Review the supplied code for correctness, security, and maintainability."
}
```

Keep the file as valid JSON. When using an installed build, edit the copy in the
`config` folder beside the executable.

### Remote Ollama servers

Open **Settings → Advanced settings → Ollama address** and enter the server host
or IP plus its port. The default is `127.0.0.1:11434`.

The application currently connects over HTTP, so only use a trusted network or
put appropriate transport security in front of a remote Ollama instance.

### Web search setup

Web search requires a [Brave Search API](https://brave.com/search/api/) key and
a model that supports Ollama tool calling.

1. Open **Settings** and enable **Web Search**.
2. Leave **Brave Search** selected and choose the result limit.
3. Provide the API key using one of the methods below.
4. Use the **Web** button beside the chat input whenever you want web tools
   enabled for that conversation.

The preferred approach is to set the key before launching the app:

```bash
export BRAVE_SEARCH_API_KEY="your-key"
```

You can also enter it in Settings. Keys entered there are saved in the local
`settings.json`; API keys are redacted from diagnostics and are not printed in
logs.

> [!IMPORTANT]
> Ordinary chats are sent only to the Ollama address you configure. When web
> search is enabled, search queries are also sent to Brave Search and the app
> fetches public webpages selected by the model. A remote Ollama server receives
> the conversation data needed to answer your request.

## Local data and privacy

Chats, settings, diagnostics, and generated images are stored on your machine.
Temporary chats are not added to `chats.json`.

| Platform | Default application-data folder |
|---|---|
| Windows | `%LOCALAPPDATA%\Ollama GUI` |
| Linux | `$XDG_DATA_HOME/ollama-gui` or `~/.local/share/ollama-gui` |

Saved conversations live in the `chats` subfolder by default. The exact active
path is always visible under **Settings → Chat storage**, and it can be changed
from there. The application can also detect conversations from the legacy
`output/chats.json` location.

Generated images are stored in `generated`, while user settings and diagnostics
use `settings.json` and `history.json` in the application-data folder.

## Platform support

| Platform | Status |
|---|---|
| Windows x64 | Officially supported; per-user installer available |
| Linux x64 | Officially supported on Wayland |
| macOS | Not officially supported or tested |

Ollama itself must be installed and running locally or reachable at the custom
address you configure. Rust and Cargo are required only when building from
source.

## Build from source

Install the [Rust toolchain](https://rustup.rs/), then:

```bash
git clone https://github.com/logancammish/ollama-gui-interface.git
cd ollama-gui-interface
cargo build --release
```

The finished binary is `target/release/ollama-gui` on Linux and
`target\release\ollama-gui.exe` on Windows.

Run a development build with:

```bash
cargo run
```

Before submitting a change, run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build
```

## Project links

- [Download the latest release](https://github.com/logancammish/ollama-gui-interface/releases/latest)
- [Browse Ollama models](https://ollama.com/search)
- [Install Ollama](https://ollama.com/download)
- [View the source repository](https://github.com/logancammish/ollama-gui-interface)
- [Read the GNU GPL v3 license](LICENSE)

---

Ollama GUI Interface is an independent open-source project built for people who
want a configurable, local-first desktop experience around Ollama.
