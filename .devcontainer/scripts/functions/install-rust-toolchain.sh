#!/bin/bash
install_rust_toolchain() {
    source ${WORKSPACE_FOLDER}/.devcontainer/scripts/functions/get-arch-and-os.sh
    read -r TARGET_ARCH TARGET_OS <<< "$(get_arch_and_os)"
    ARCH_AND_OS="${TARGET_ARCH}-${TARGET_OS}"
    echo "Detected architecture and OS: $ARCH_AND_OS"
    RUST_TOOLCHAIN="stable-$ARCH_AND_OS"
    echo "Adding rustup target '$ARCH_AND_OS'"
    rustup target add "$ARCH_AND_OS"
    echo "Installing Rust toolchain for '$ARCH_AND_OS'"
    rustup toolchain install "$RUST_TOOLCHAIN"
}
