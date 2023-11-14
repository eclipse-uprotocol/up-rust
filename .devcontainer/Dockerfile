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

FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

ARG USERNAME=vscode
ARG TARGETARCH

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install \
    # Others
    curl \
    zip \
    gcc \
    git \
    && \
    rm -rf /var/lib/apt/lists/*

# Before installing cargo tools, change to the user that will be used in the container later.
# The reason is that cargo creates some cache, etc. folders with the correct group rustlang, but 
# without making them group writable. Easiest fix is to change to the correct user before the install,
# so that the owner is correct from the start.
USER ${USERNAME}

# Install cargo cli tools
RUN cargo install cargo-tarpaulin --locked

# Install cargo tools for cross compilation
RUN rustup target add aarch64-unknown-linux-gnu \
    && rustup toolchain install stable-aarch64-unknown-linux-gnu 
