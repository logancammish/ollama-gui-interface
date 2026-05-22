# ollama-gui-interface

[![build](https://github.com/logancammish/ollama-gui-interface/actions/workflows/rust.yml/badge.svg)](https://github.com/logancammish/ollama-gui-interface/actions/workflows/rust.yml)

A simple desktop GUI for using Ollama models, written in Rust.

This project is made for people who want more control over their Ollama setup, including custom system prompts, chat history, thinking mode, filtering, logs, and remote Ollama servers.

---

## Why use this?

Ollama now has an official GUI interface as of Ollama `0.10.0`.
However, as far as Ollama `1.17.7`:

| Feature | Official Ollama GUI | ollama-gui-interface |
|---|---:|---:|
| Thinking | ❌ | ✅ |
| Use a model hosted on an external IP | ❌ | ✅ |
| Customize the system prompt | ❌ | ✅ |
| Customize chat history | ❌ | ✅ |
| Enable text filtering | ❌ | ✅ |
| Easily toggle history and logs | ❌ | ✅ |
| Highly customizable settings | ❌ | ✅ |

---

## Features

| Category | Description |
|---|---|
| Model control | Use local Ollama models or connect to an external Ollama server |
| Thinking toggle | Enable or disable thinking where supported |
| System prompts | Choose and customize system prompts |
| Chat history | View, edit, and manage conversation history |
| Logs | Easily access and toggle logs |
| Filtering | Optional text filtering support |
| Settings | Customizable interface and behavior |

---

## Quick installation

The easiest way to use the app is to download the executable from the latest release.

### Steps

1. Install and run Ollama  
   Download it here:  
   https://ollama.com/download

2. Download the latest executable  
   Go to the Releases page:  
   https://github.com/logancammish/ollama-gui-interface/releases

3. Run the executable  
   The application should start normally.

4. Install a model inside the application  
   You can browse available Ollama models here:  
   https://ollama.com/search

---

## Requirements

| Requirement | Notes |
|---|---|
| Ollama | Must be installed and running |
| Operating system | Windows & Linux is officially supported |
| Rust/Cargo | Only required if building from source |

---

## Building from source

You can also build the application manually.

### 1. Install Rust

If you do not already have Rust and Cargo installed, install them using Rustup:

https://rustup.rs/

### 2. Clone the repository

```bash
git clone https://github.com/logancammish/ollama-gui-interface.git
cd ollama-gui-interface
```

### 3. Build the application

```bash
cargo build --release
```

### 4. Find the executable

After building, the executable will be located in:

```text
target/release
```

---

## Platform support

| Platform | Support status |
|---|---|
| Windows | Officially supported |
| Linux | Officially supported (Wayland) |
| macOS | Not officially supported or tested |

---

## Useful links

| Link | URL |
|---|---|
| Ollama download | https://ollama.com/download |
| Ollama model library | https://ollama.com/search |
| Latest releases | https://github.com/logancammish/ollama-gui-interface/releases |
| Repository | https://github.com/logancammish/ollama-gui-interface |

---

## Notes

This project is intended for users who want a more configurable Ollama GUI experience than the default official interface currently provides.

If you only need a basic Ollama chat interface, the official Ollama GUI may be enough.

If you want more control over prompts, thinking, filtering, history, logs, and remote model connections, this app may be more useful.