---
title: Install DesignCraft
description: Download DesignCraft 0.6.0 for Windows 11, macOS or Linux, or install it from ArtCraft Launcher.
order: 20
verified: 0.6.0
---

Download the file for your system from the [latest release](https://github.com/storytold/designcraft/releases/latest) and install it. In the file names, `<ver>` stands for the version number, for example `0.6.0`. The release also has `SHA256SUMS.txt`, which lists a checksum for every file. The [DesignCraft page on getartcraft.com](https://getartcraft.com/apps/designcraft) links to the download as well.

## Windows 11

1. Open the [latest release](https://github.com/storytold/designcraft/releases/latest).
2. Download `designcraft-<ver>-windows-x64.msi`. It fits most PCs (64-bit Intel and AMD).
3. Run the file and follow the installer.
4. Start **DesignCraft** from the Start menu.

The release also has an `arm64` installer for ARM PCs such as Snapdragon, and an `x86` installer for 32-bit Windows. To run without installing, download `designcraft-<ver>-windows-x64-portable.zip` (or the `arm64` or `x86` version), unzip it and start the program inside. The installers and executables are code-signed.

## macOS

1. Download `designcraft-<ver>-macos-universal.dmg`. It works on Apple silicon and Intel Macs.
2. Open it and drag DesignCraft to Applications.

The app is signed and notarized.

## Linux

Pick one file from the release. Replace `x86_64` with `aarch64` on ARM.

| File                                      | Notes                                                    |
|-------------------------------------------|----------------------------------------------------------|
| `designcraft-<ver>-linux-x86_64.AppImage` | Runs anywhere.                                           |
| `designcraft-<ver>-linux-x86_64.flatpak`  | Sandboxed. Install with `flatpak install --user <file>`. |
| `designcraft-<ver>-linux-x86_64.deb`      | Debian and Ubuntu.                                       |
| `designcraft-<ver>-linux-x86_64.rpm`      | Fedora, RHEL and openSUSE.                               |
| `designcraft-<ver>-linux-x86_64.tar.gz`   | Unpack anywhere.                                         |

For RISC-V there is only `designcraft-<ver>-linux-riscv64.tar.gz`. It needs glibc 2.39 or newer (Ubuntu 24.04 or newer). A FreeBSD build (`designcraft-<ver>-freebsd-x86_64.tar.gz`) is on the release page too.

## Install with ArtCraft Launcher

ArtCraft Launcher lists DesignCraft and can install and update it for you. See [ArtCraft Launcher](https://getartcraft.com/docs/craft-launcher) for how it works.

## First run

Open DesignCraft. You land on the start screen. Continue with [Get started](index.md). If the window closes right away, see [DesignCraft closes right after it starts](../troubleshooting/app-closes-at-start.md).
