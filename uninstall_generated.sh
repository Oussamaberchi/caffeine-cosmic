#!/bin/bash
set -e

echo "Uninstalling Cosmic Caffeine..."

sudo rm -v \
    /usr/bin/cosmic-caffeine \
    /usr/share/applications/com.github.cosmic-caffeine.desktop \
    /usr/share/icons/hicolor/scalable/apps/com.github.cosmic-caffeine.svg \
    /usr/share/metainfo/com.github.cosmic-caffeine.metainfo.xml

echo "Updating system databases..."
sudo update-desktop-database
sudo gtk-update-icon-cache /usr/share/icons/hicolor

echo "Uninstallation complete."
