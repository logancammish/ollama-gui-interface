# ollama-gui-interface
[![build](https://github.com/logancammish/ollama-gui-interface/actions/workflows/rust.yml/badge.svg)](https://github.com/logancammish/ollama-gui-interface/actions/workflows/rust.yml)

A GUI interface for Ollama bots written in Rust


## Please note:
As of Ollama `0.10.0`, Ollama now has an official GUI interface - however, as far as I see as of `1.10.1`, it does not provide the following features:
1. Enabling or disabling thinking
2. Using a model hosted on an external IP
3. Customising system prompt and chat history
4. Enabling text filtering
5. Easily accessible and toggleable history and logs

   

## Streamlined installation

You may install the executable file in the release menu under the latest version.
To use:
1. Ensure you have Ollama installed and running (install here: https://ollama.com/download)
2. Run the executable file, and the application will start
3. You may install any models inside the application, the list of models can be found here: https://ollama.com/search 



## Building/Downloading

This application officially supports Windows, and should work fine on Linux.

1. Ensure you have cargo installed, if not install with Rustup
2. Clone this repository (git clone https://github.com/logancammish/ollama-gui-interface.git)
3. Build with cargo build --release
4. You will find the executable in target/release
