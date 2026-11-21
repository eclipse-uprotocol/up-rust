# up-spec

## 📘 Overview

**`up-spec`** is the core specification repository of [uProtocol](https://github.com/eclipse-uprotocol/up-spec).  
It defines the **Protobuf-based communication protocols** used by uProtocol across its layers.

This repository is included as a submodule in the [up-rust](https://github.com/eclipse-uprotocol/up-rust/) project.

---

## 📦 What's Inside?

The `up-spec` repository contains:

- Protobuf message definitions.
- Specification for all **three layers** of the uProtocol communication stack.
- Core schema files that are shared across uProtocol implementations.

---

## ⬇️ How to Download

There are two ways to get `up-spec`:

---

### ✅ Method 1: Clone with Submodules (Recommended)

Download the **up-spec** with the **up-rust** in single command

```bash
# clone the up-rust repository
git clone --recurse-submodules git@github.com:eclipse-uprotocol/up-rust
```

Here the submodule **up-spec** also downloaded in you can see it in parent location named **up-spec**.

### 🔧 Method 2: Clone Without Submodules (Manual Setup)

First Download the **up-rust** and then download the **up-spec** as submodule using the below command

##### For ssh clone

```bash
# Clone the up-rust
git@github.com:eclipse-uprotocol/up-rust.git

# then run to download the submodules
git submodule update --init --recursive
```

##### For Https clone

```bash
# Clone the up-rust
https://github.com/eclipse-uprotocol/up-rust.git

# Make sure to change the .gitmodules file in the up-rust folder
# ./.gitmodules
# url = git@github.com:eclipse-uprotocol/up-spec
# To
# url = https://github.com/eclipse-uprotocol/up-spec.git

# Then update the submodule
git submodule update --init --recursive
```
