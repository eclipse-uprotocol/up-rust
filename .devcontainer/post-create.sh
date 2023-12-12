#!/bin/bash

# This is a workaround which is againt necessary on MacOS 14.0, it looks like this bug is back:
# https://github.com/microsoft/vscode-dev-containers/issues/1487#issuecomment-1143907307

# grant permissions to mounted rust volume
chown vscode:vscode /rust-volume

# create /.cargo/config.toml in root folder
mkdir /.cargo/
touch /.cargo/config.toml
cat << EOF > /.cargo/config.toml
[build]
target-dir = "/rust-volume/target"
EOF
