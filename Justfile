# Justfile for cosmic-caffeine

# Installation paths
prefix := "/usr"
bin_dir := prefix + "/bin"
share_dir := prefix + "/share"
applications_dir := share_dir + "/applications"
icons_dir := share_dir + "/icons/hicolor/scalable/apps"

# Default recipe
default: build

# Build the project
build:
    cargo build --release

# Install the application
install: build
    install -Dm755 target/release/cosmic-caffeine {{bin_dir}}/cosmic-caffeine
    install -Dm644 resources/app.desktop {{applications_dir}}/cosmic-caffeine.desktop
    install -Dm644 resources/oussama-berchi-caffeine-cosmic.svg {{icons_dir}}/oussama-berchi-caffeine-cosmic.svg

# Package the application as a .deb file
package:
    cargo deb

# Clean build artifacts
clean:
    cargo clean
