#!/bin/bash

#region sudo stuff

# This is a workaround which is againt necessary on MacOS 14.0, it looks like this bug is back:
# https://github.com/microsoft/vscode-dev-containers/issues/1487#issuecomment-1143907307
# grant permissions to mounted rust volume
sudo chown vscode:vscode /rust-volume

# create /.cargo/config.toml in root folder
sudo mkdir /.cargo/
sudo touch /.cargo/config.toml
sudo bash -c "cat << EOF > /.cargo/config.toml
[build]
target-dir = \"/rust-volume/target\"
EOF"

#endregion

for arg in "$@"; do
    if [[ "$arg" == --workspace-folder=* ]]; then
        WORKSPACE_FOLDER="${arg#--workspace-folder=}"
    fi
done
INSTALL_RUST_TOOLCHAIN_FUNC="${WORKSPACE_FOLDER}.devcontainer/scripts/functions/install-rust-toolchain.sh"
if [[ -f "$INSTALL_RUST_TOOLCHAIN_FUNC" ]]; then
    source $INSTALL_RUST_TOOLCHAIN_FUNC
    install_rust_toolchain "$@"
fi