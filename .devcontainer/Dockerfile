################################################################################
# Copyright (c) 2023 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
################################################################################

FROM mcr.microsoft.com/devcontainers/rust:latest
ARG USERNAME=vscode
ARG TARGETARCH

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install \
    curl \
    gcc \
    git \
    openjdk-17-jre \
    zip \
    && \
    rm -rf /var/lib/apt/lists/*

# Download openfasttrace JARs and create shortcut oft executable
ARG OFT_CORE_VERSION=4.1.0
ARG OFT_ASCIIDOC_PLUGIN_VERSION=0.3.0
ENV LIB_DIR=/opt/oft/lib
RUN <<EOF
mkdir -p $LIB_DIR
base_url=https://github.com/itsallcode
wget -P $LIB_DIR ${base_url}/openfasttrace/releases/download/$OFT_CORE_VERSION/openfasttrace-$OFT_CORE_VERSION.jar
wget -P $LIB_DIR ${base_url}/openfasttrace-asciidoc-plugin/releases/download/$OFT_ASCIIDOC_PLUGIN_VERSION/openfasttrace-asciidoc-plugin-$OFT_ASCIIDOC_PLUGIN_VERSION-with-dependencies.jar
EOF
RUN echo '#!/bin/bash\n\njava -cp "${LIB_DIR}/*" org.itsallcode.openfasttrace.core.cli.CliStarter "$@"' > /usr/local/bin/oft \
    && chmod +x /usr/local/bin/oft

# Before installing cargo tools, change to the user that will be used in the container later.
# The reason is that cargo creates some cache, etc. folders with the correct group rustlang, but 
# without making them group writable. Easiest fix is to change to the correct user before the install,
# so that the owner is correct from the start.
USER ${USERNAME}

# Install cargo cli tools
RUN cargo install cargo-nextest cargo-deny cargo-tarpaulin --locked

# Install cargo tools for cross compilation
RUN rustup target add aarch64-unknown-linux-gnu \
    && rustup toolchain install stable-aarch64-unknown-linux-gnu 
