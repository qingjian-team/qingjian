---
title: Clipboard sync over the local network
order: 6
description: Passing copied text and images between computers and phones on the same network without any server; turning it on, pairing, connection states and what the neon glow means.
---

Pass copied text and images between macOS, Windows and Android: copy on one device, paste on the other. The content travels encrypted between your own devices only and never goes through a server. This is an experimental feature, off by default, and not yet available on Linux.

## Turning it on and pairing

1. Turn on sync on both devices:
   - macOS, Windows: under 设置 → 实验功能 (Settings → Experimental features), turn on Qingjian Clipboard and 局域网同步 (Local network sync).
   - Android: 设置 → 局域网同步 (Settings → Local network sync).
2. Open the devices page: on a computer, bring up the clipboard panel with its shortcut (⌘⇧V on macOS and Ctrl + Alt + V on Windows by default) and click 设备 (Devices); on Android it's the page you just opened.
3. On one device click 显示配对码 (Show pairing code); on the other, tap that device under 附近的设备 (Nearby devices) and enter the 6 digits.

A pairing code is valid for 2 minutes and is voided after one wrong entry; just generate a new one. If the other device doesn't show up under Nearby devices, use 手动输入地址 (Enter address manually) and type its IP address and port, for example `192.168.1.5:53319`.

You only pair once. You can rename a device; until you do, it has a random name, which is what other devices show after pairing. While sync is on, the device name and the computer name (or phone model) are visible to devices on the local network, so don't put anything private in them.

## What syncs

- Text and images sync; files don't.
- Received items go into the history. On a computer the newest one is also put on the system clipboard, ready to paste; on Android only text is, and images have to be taken from the clipboard panel.
- Deleting an item by hand deletes it on the other devices too. Items dropped for going over the history limit, or clearing this device's history, only affect this device.
- A device being offline isn't a failure; items are sent once it comes back.
- Each device syncs its whole history, including what it received from other devices. So if A is paired with B and B with C, what A copies reaches C through B, and C's reaches A through B (B has to be online). A and C aren't paired and don't see each other on the devices page. If you don't want content to reach C, don't pair B with both.

## Connection states

| Shown | Meaning |
|---|---|
| 已连接 (Connected) | Actually connected recently |
| 正在连接… (Connecting…) | No result yet, still trying |
| 连不上 (Can't connect) | The last attempt failed; the other device may have turned sync off, shut down or changed networks. The time shown is the last sync |

## Neon glow

When content arrives, a ring of light glows around the edge of the screen (around the keyboard on Android). While it transfers the color moves from blue through cyan to purple, turns green and fades on success, and turns red on failure; on computers a failure also flashes twice, so you can tell even without telling red from green. The sender only lights up, in red, when sending fails. You can turn the glow off in the sync settings. On Windows it isn't shown during full-screen games and presentations; on macOS it always shows.

## Unpairing

On the devices page, click 解除配对 (Unpair) on the other device's row. Doing it on one side is enough: the other device is told the next time it connects and removes the pairing on its side too, even if it was offline when you unpaired.

## Common questions

- **Does it still sync with the clipboard panel closed?** Yes. On a computer the clipboard program just has to be running: check the tray on Windows, or the clipboard icon in the menu bar on macOS.
- **The phone didn't get it right away?** Android syncs only while the keyboard or the Qingjian app is running; if the system has closed them, it catches up the next time the keyboard comes up.
- **Can't pair or connect?** Make sure both are on the same network. The first time you turn on sync on Windows, allow private networks if a firewall prompt appears; on macOS with the firewall on, allow 青简剪贴板 (Qingjian Clipboard) to accept incoming connections when asked. Guest networks and routers with "AP isolation" turned on may block devices from reaching each other.
