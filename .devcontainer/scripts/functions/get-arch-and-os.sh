#!/bin/bash
get_arch_and_os() {
    ARCH=$(uname -m)
    OS=$(uname -s | tr "[:upper:]" "[:lower:]")

    case "$ARCH" in
    x86_64)
        TARGET_ARCH="x86_64"
        ;;
    aarch64 | arm64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
    esac

    case "$OS" in
    linux)
        TARGET_OS="unknown-linux-gnu"
        ;;
    *)
        echo "OS is unsupported or not implemented in this script: $OS"
        exit 1
        ;;
    esac
    echo "$TARGET_ARCH $TARGET_OS"
}
