# Caffeine for COSMIC

A native Rust applet for COSMIC to prevent system sleep.


## Features

- **Rust-Powered**: Built with native performance and reliability using libcosmic/iced.
- **Symbolic Icons**: Clean, scalable SVG icons that adapt to your theme.
- **Modern UI**: Active state (Brick Red), Inactive state (White), and smooth hover interactions (Scale effect) with no intrusive tooltips.
- **Multi-Instance Sync**: Seamlessly synchronizes state across Panel, Dock, and other instances via D-Bus.


## Prerequisites

Installation requires building the applet locally on your machine, so in addition to running COSMIC as your desktop environment, you also need to install Rust and some development libraries.

- [COSMIC](https://system76.com/cosmic) Desktop environment
- [Rust](https://rust-lang.org/tools/install/) (latest stable)
- These library dependencies: `libssl-dev`, `libwayland-dev`, `libxkbcommon-dev`


## Installation

To install, clone this repository and run the install script.

```bash
git clone https://github.com/Oussamaberchi/caffeine-cosmic.git
cd caffeine-cosmic
./install.sh
```

The install script uses Rust's `cargo` tool to build the Caffeine applet binary, then install it and other artifacts into appropriate locations and update desktop databases for the applet to be found by the system and work.

Once installed, you will find the applet in COSMIC Settings ready to be installed into your panel or dock.

1. In COSMIC Settings, open `Desktop > Panel > Configure panel applets` or `Desktop > Dock > Configure dock applets`.
1. Use `Add applet` to find Caffeine.
1. Click `Add` to add the applet.
1. (optional) Use the handle at the left of the Caffeine entry to move it to your desired location within the panel or dock.


## Usage

1. Click the coffee cup icon in the panel to toggle caffeine mode.
2. Select your preferred duration or use "Infinity" mode.
3. The icon turns red when active and white when inactive.
