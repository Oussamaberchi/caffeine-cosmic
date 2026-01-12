#!/bin/bash
set -e

echo "Starting build process..."
# Build the project in release mode
cargo build --release

echo "Installing files to system directories (requires sudo)..."
# Install binary
sudo install -Dm0755 target/release/cosmic-caffeine /usr/bin/cosmic-caffeine

# Install desktop entry
sudo install -Dm0644 resources/app.desktop /usr/share/applications/com.github.cosmic-caffeine.desktop

# Install metadata
sudo install -Dm0644 resources/app.metainfo.xml /usr/share/metainfo/com.github.cosmic-caffeine.metainfo.xml

# Install icon
sudo install -Dm0644 resources/caffeine-cup-symbolic.svg /usr/share/icons/hicolor/scalable/apps/com.github.cosmic-caffeine.svg

# Update icon cache
if command -v gtk-update-icon-cache &> /dev/null; then
    echo "Updating icon cache..."
    sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi

echo "Installation complete! You may need to restart the COSMIC panel or session to see the applet."
