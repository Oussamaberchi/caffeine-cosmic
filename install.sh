#!/bin/bash
set -e

echo "Building Caffeine..."
cargo build --release

echo "Installing binary..."
mkdir -p ~/.local/bin
cp target/release/cosmic-caffeine ~/.local/bin/caffeine

echo "Installing icon..."
sudo mkdir -p /usr/share/icons/hicolor/scalable/apps/
sudo cp assets/caffeine.svg /usr/share/icons/hicolor/scalable/apps/

echo "Installing desktop entry..."
mkdir -p ~/.local/share/applications
cp assets/caffeine.desktop ~/.local/share/applications/

echo "Updating databases..."
update-desktop-database ~/.local/share/applications
sudo gtk-update-icon-cache /usr/share/icons/hicolor

echo "Installation complete! You may need to restart the panel or log out/in."
