# Justfile for cosmic-caffeine
# Run 'just --list' to see all available commands

# Installation paths (configurable)
prefix := "/usr"
bin_dir := prefix / "bin"
share_dir := prefix / "share"
applications_dir := share_dir / "applications"
icons_dir := share_dir / "icons/hicolor/scalable/apps"

# Local user installation paths
local_bin := env_var("HOME") / ".local/bin"
local_apps := env_var("HOME") / ".local/share/applications"

# Application metadata
app_name := "cosmic-caffeine"
desktop_file := "com.github.cosmic-caffeine.desktop"
icon_file := "oussama-berchi-caffeine-cosmic.svg"

# Default recipe - show help
default:
    @just --list

# Build the project in release mode
build:
    @echo "Building cosmic-caffeine..."
    cargo build --release
    @echo "Build complete! Binary located at: target/release/{{app_name}}"

# Build in debug mode for development
build-debug:
    @echo "Building in debug mode..."
    cargo build
    @echo "Debug build complete!"

# Install to system directories (requires sudo)
install: build
    @echo ""
    @echo "Installing cosmic-caffeine to system directories..."
    @echo "=================================================="
    @echo ""
    @echo "Installing binary to {{bin_dir}}/{{app_name}}..."
    sudo install -Dm755 target/release/{{app_name}} {{bin_dir}}/{{app_name}}
    @echo "Installing desktop entry to {{applications_dir}}/{{desktop_file}}..."
    sudo install -Dm644 assets/{{desktop_file}} {{applications_dir}}/{{desktop_file}}
    @echo "Installing icon to {{icons_dir}}/{{icon_file}}..."
    sudo install -Dm644 assets/{{icon_file}} {{icons_dir}}/{{icon_file}}
    @echo ""
    @echo "Updating icon cache..."
    -sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
    @echo ""
    @echo "=================================================="
    @echo "Installation complete!"
    @echo ""
    @echo "NEXT STEPS:"
    @echo "  1. Log out and log back in, OR restart the COSMIC panel"
    @echo "  2. Open Settings > Desktop > Panel"
    @echo "  3. Add 'Caffeine' applet to your panel"
    @echo ""

# Install to user's local directories (no sudo required)
install-local: build
    @echo ""
    @echo "Installing cosmic-caffeine to user directories..."
    @echo "=================================================="
    @echo ""
    @mkdir -p {{local_bin}}
    @echo "Installing binary to {{local_bin}}/{{app_name}}..."
    install -Dm755 target/release/{{app_name}} {{local_bin}}/{{app_name}}
    @mkdir -p {{local_apps}}
    @echo "Installing desktop entry to {{local_apps}}/{{desktop_file}}..."
    install -Dm644 assets/{{desktop_file}} {{local_apps}}/{{desktop_file}}
    @echo ""
    @echo "NOTE: Icon will be installed system-wide (requires sudo)..."
    sudo mkdir -p {{icons_dir}}
    sudo install -Dm644 assets/{{icon_file}} {{icons_dir}}/{{icon_file}}
    -sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
    @echo ""
    @echo "Updating desktop database..."
    -update-desktop-database {{local_apps}} 2>/dev/null || true
    @echo ""
    @echo "=================================================="
    @echo "Installation complete!"
    @echo ""
    @echo "NEXT STEPS:"
    @echo "  1. Make sure ~/.local/bin is in your PATH"
    @echo "  2. Log out and log back in, OR restart the COSMIC panel"
    @echo "  3. Open Settings > Desktop > Panel"
    @echo "  4. Add 'Caffeine' applet to your panel"
    @echo ""

# Uninstall from system directories (requires sudo)
uninstall:
    @echo ""
    @echo "Uninstalling cosmic-caffeine from system directories..."
    @echo "======================================================="
    @echo ""
    @echo "Removing binary..."
    -sudo rm -f {{bin_dir}}/{{app_name}}
    @echo "Removing desktop entry..."
    -sudo rm -f {{applications_dir}}/{{desktop_file}}
    @echo "Removing icon..."
    -sudo rm -f {{icons_dir}}/{{icon_file}}
    @echo ""
    @echo "Updating icon cache..."
    -sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
    @echo ""
    @echo "======================================================="
    @echo "Uninstallation complete!"
    @echo "You may need to restart the COSMIC panel or log out/in."
    @echo ""

# Uninstall from user's local directories
uninstall-local:
    @echo ""
    @echo "Uninstalling cosmic-caffeine from user directories..."
    @echo "======================================================"
    @echo ""
    @echo "Removing binary..."
    -rm -f {{local_bin}}/{{app_name}}
    @echo "Removing desktop entry..."
    -rm -f {{local_apps}}/{{desktop_file}}
    @echo "Removing icon..."
    -sudo rm -f {{icons_dir}}/{{icon_file}}
    @echo ""
    @echo "Updating icon cache..."
    -sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
    @echo ""
    @echo "======================================================"
    @echo "Uninstallation complete!"
    @echo ""

# Package the application as a .deb file
package: build
    @echo "Creating .deb package..."
    cargo deb
    @echo ""
    @echo "Package created at: target/debian/"
    @ls -la target/debian/*.deb 2>/dev/null || echo "No .deb files found"

# Run the application (for testing)
run:
    cargo run

# Run with debug logging
run-debug:
    RUST_LOG=debug cargo run

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    @echo "Clean complete!"

# Run tests
test:
    cargo test

# Check for compilation errors without building
check:
    cargo check

# Format code
fmt:
    cargo fmt

# Run clippy linter
lint:
    cargo clippy -- -D warnings
