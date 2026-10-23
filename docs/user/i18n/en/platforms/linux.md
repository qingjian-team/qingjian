---
title: Linux
order: 2
description: "Installing Qingjian on Linux (and FreeBSD): the Fcitx5 prebuilt package, installing Fcitx5 and Fcitx4 from source, your first input, updating and uninstalling, and how it differs from the desktop versions."
---

The Linux version of Qingjian runs as an input method in Fcitx5 or Fcitx4, and candidates use Fcitx's own candidate panel. For now it's installed from a terminal, and the background service has to be started by hand after each login.

- For Fcitx5 there's a prebuilt package for Ubuntu 26.04; other distributions install from source. Verified with GTK4 and Qt6 apps and Firefox on the GNOME desktop (Wayland) of Ubuntu 26.04.
- For Fcitx4 there's a source install for Ubuntu 22.04, verified with GTK3 text fields.
- On FreeBSD 15.1, a Fcitx5 source install has been verified in XFCE and KDE Plasma X11 sessions and with Wayland apps under sway.

Local candidates, learning, the local sentence model and the optional AI services are supported, and the Qingjian settings app is included. For differences from macOS / Windows, see [Platform differences](differences.md).

## Fcitx5 prebuilt package (Ubuntu 26.04)

The prebuilt package is built on Ubuntu 26.04 (x86_64) and tied to the system's Fcitx5 version; on other distributions, use the source install in the next section. First install Fcitx5 and input support for applications:

```sh
sudo apt install python3 fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt6 fcitx5-config-qt
```

Download the package whose name ends in `linux-x86_64.tar.gz` from the [download page](https://qingjian.app/en/download), unpack it, and run in the unpacked folder:

```sh
./install.sh

# Start Qingjian and keep this terminal open
~/.local/bin/qingjian-linux-server
```

Then add the input method as described in [Your first input](#Your-first-input) below.

## Fcitx5 source install

First install Rust 1.96 or later and the packages needed to build. On Debian / Ubuntu:

```sh
sudo apt install cmake g++ pkg-config python3 libssl-dev \
  libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev nlohmann-json3-dev \
  fonts-noto-core fonts-noto-cjk fonts-noto-color-emoji \
  fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt6 fcitx5-config-qt
```

The three font packages are required: without `fonts-noto-core` the settings app won't open, and without the other two the Chinese text and emoji in the interface can't be displayed. The install script lists anything missing at the start.

On FreeBSD:

```sh
pkg install bash cmake pkgconf python3 rustup-init nlohmann-json \
  fcitx5 fcitx5-gtk3 fcitx5-qt6 fcitx5-configtool
rustup-init   # install Rust
```

Enable Fcitx5 as your distribution describes; X11 apps usually need `GTK_IM_MODULE=fcitx`, `QT_IM_MODULE=fcitx` and `XMODIFIERS=@im=fcitx` set, then log in again.

In the source folder, run:

```sh
# Download and verify the full dictionaries and the local sentence model
tools/release/data-fetch.sh
apps/linux/scripts/install.sh

# Start Qingjian and keep this terminal open
~/.local/bin/qingjian-linux-server
```

- To try out the install process first, use `apps/linux/scripts/install.sh --sample --debug`: it doesn't download the full dictionaries and has only a few sample words (such as 你好).
- It installs to `~/.local` by default; `--prefix /absolute/path` changes the location.
- Install and run it as your own user, without `sudo`.

## Fcitx4 source install (Ubuntu 22.04)

If you already use Fcitx4, `fcitx-frontend-qt5` is enough for Qt5 apps. First install Rust 1.96 or later, plus:

```sh
sudo apt install cmake g++ pkg-config python3 libssl-dev fcitx-libs-dev libx11-dev libxres-dev nlohmann-json3-dev \
  fonts-noto-core fonts-noto-cjk fonts-noto-color-emoji \
  fcitx fcitx-frontend-gtk3 fcitx-frontend-qt5 fcitx-config-gtk
```

In the source folder, run:

```sh
tools/release/data-fetch.sh
apps/linux/scripts/install.sh --fcitx4
~/.local/bin/qingjian-linux-server
```

Keep the terminal open for the last command. Run `fcitx -r` to restart Fcitx4, and add 青简 (Qingjian) to the input method list in the configuration tool.
To try out the install process first, use `apps/linux/scripts/install.sh --fcitx4 --sample --debug`.
When you switch to Qingjian, the input method icon in the tray shows the Qingjian logo.

## Your first input

1. Restart Fcitx, or log out and back in.
2. Open the Fcitx configuration tool, untick "Only Show Current Language", and add 青简 (Qingjian).
3. Switch to Qingjian, type `nihao` and press Space to get 你好.

Tap `Shift` to switch between Chinese and English; for keys see the [Key reference](../getting-started/keys.md). Settings are under 青简设置 (Qingjian Settings) in the application menu.
Closing the terminal you started Qingjian from stops Qingjian; after your next login, run `~/.local/bin/qingjian-linux-server` again.

## Differences from the desktop versions

- **Candidate panel**: Fcitx's candidate panel is used and its look comes from the Fcitx theme, so Qingjian's themes, fonts, expanded horizontal layout and key hints don't apply. The panel shows only the first gloss, with part of speech and Japanese readings.
- **Chinese/English mode**: each window keeps its own; new windows start in Chinese, or can be set to start in English, see [Chinese and English](../input/english-mode.md#Default-mode-and-per-app-memory). Fcitx5's left `Shift` hotkey needs to be turned off; see the same page.
- **Tab**: goes to the app when you're not typing; while typing pinyin it accepts the sentence completion, or goes to the next page if there's none; among English candidates it picks the highlighted one.
- **Digits**: when the current page has no matching candidate, the digit is appended to the pinyin, and `hao6` followed by Enter is output as typed.
- **Direct English input**: once in it, digits and punctuation keep being appended, and ending with a space keeps the space (`gpt-6` then Space gives `gpt-6 `).
- **Sentence completion** is shown in the hint line below the candidate panel; press `Tab` to accept it.
- **Private input**: you can type in input fields Fcitx5 recognizes as private, but nothing is recorded, learned or sent; password fields are handed straight back to the app. This relies on apps marking fields correctly, and GTK4 apps' private hints currently don't come through. Fcitx4 only recognizes the password-field mark, not other private fields; if you don't want records kept, turn off learning and the input log, see [Editing the configuration file](../advanced/config-file.md#Linux).
- **Not yet supported**: Wubi and custom schemes, auxiliary code, the symbol panel, extra paging keys and highlight keys, the punctuation shortcut, automatic spacing between Chinese and Latin text, toned pinyin, filling in glosses automatically and sentence translation for the vocabulary book, AI usage statistics, on-demand suggestions.

## Updating and uninstalling

To update: close the terminal running Qingjian, run the matching install command again (for the prebuilt package, download the new package and run `./install.sh`), then restart Fcitx and start Qingjian again.
The installer checks files and keeps installed files you've modified, telling you so; back them up first.

To uninstall:

```sh
apps/linux/scripts/uninstall.sh
# If you chose a custom install location:
apps/linux/scripts/uninstall.sh --prefix /absolute/path/used/when/installing
```

For the prebuilt package, use the package's `./uninstall.sh`. After uninstalling, close the terminal running Qingjian yourself and restart Fcitx. Uninstalling keeps your settings, personal dictionaries, learning data and other input methods.
