# Caffeine for COSMIC

A native Rust applet for Pop!_OS COSMIC to prevent system sleep.

## Features

- **Rust-Powered**: Built with native performance and reliability using libcosmic/iced.
- **Symbolic Icons**: Clean, scalable SVG icons that adapt to your theme.
- **Modern UI**: Active state (Brick Red), Inactive state (White), and smooth hover interactions (Scale effect) with no intrusive tooltips.

## Installation

```bash
git clone https://github.com/Oussamaberchi/caffeine-cosmic.git
cd caffeine-cosmic
./install.sh
```

## Requirements

- Rust (latest stable)
- Pop!_OS COSMIC Desktop environment
- `libssl-dev`, `libwayland-dev`, `libxkbcommon-dev` (standard build dependencies)

## Usage

1. Click the coffee cup icon in the panel to toggle caffeine mode.
2. Select your preferred duration or use "Infinity" mode.
3. The icon turns red when active and white when inactive.
