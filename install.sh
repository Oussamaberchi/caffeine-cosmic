#!/bin/bash
set -e

echo "Building Caffeine..."
cargo build --release

echo "Installing binary..."
mkdir -p ~/.local/bin
cp target/release/cosmic-caffeine ~/.local/bin/cosmic-caffeine

echo "Installing icon..."
sudo mkdir -p /usr/share/icons/hicolor/scalable/apps/
sudo cp assets/oussama-berchi-caffeine-cosmic.svg /usr/share/icons/hicolor/scalable/apps/oussama-berchi-caffeine-cosmic.svg

echo "Installing desktop entry..."
mkdir -p ~/.local/share/applications
cp assets/com.github.cosmic-caffeine.desktop ~/.local/share/applications/com.github.cosmic-caffeine.desktop

echo "Updating databases..."
update-desktop-database ~/.local/share/applications
sudo gtk-update-icon-cache /usr/share/icons/hicolor

echo "Installation complete! You may need to restart the panel or log out/in."
