# kbd-layout-switch

Switch the GNOME keyboard layout automatically when a Bluetooth keyboard
connects or disconnects.

For example: a laptop with a built-in JIS keyboard and an external US
keyboard. While the external keyboard is connected the layout is `us`; when it
disconnects the layout goes back to `jp`.

GNOME has no per-device layout setting, so this program listens for BlueZ
events on D-Bus and rewrites the xkb entry of
`org.gnome.desktop.input-sources sources`.

## Requirements

- GNOME (Wayland or X11), with the `gsettings` command
- BlueZ
- systemd, for the user service
- A Rust toolchain to build it, unless you use a prebuilt binary

## Tested with

- Fedora 45, GNOME 51 (Wayland)
- Built-in keyboard: Intel MacBook, JIS layout (butterfly keyboard)
- Bluetooth keyboard: Split65, US layout

## Install

```sh
cargo install --git https://github.com/u-na-gi/kbd-layout-switch
```

Or, to download a prebuilt binary instead of compiling, with
[cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```sh
cargo binstall --git https://github.com/u-na-gi/kbd-layout-switch kbd-layout-switch
```

Prebuilt binaries are static and available for x86_64 and aarch64 Linux.

Then install and start the systemd user service. `--device` is a prefix of the
keyboard's name as shown by `bluetoothctl devices`:

```sh
kbd-layout-switch install --device Split65
```

The service starts with your graphical session from then on. To change the
options, run `install` again with the new ones. After upgrading with
`cargo install`, run `systemctl --user restart kbd-layout-switch`.

## Options

`install` and `run` take the same options.

| Option | Default | Meaning |
| --- | --- | --- |
| `--device <PREFIX>` | (required) | Name prefix of the Bluetooth keyboard |
| `--connected <LAYOUT>` | `us` | Layout while the keyboard is connected |
| `--disconnected <LAYOUT>` | `jp` | Layout while it is not connected |

A layout is an xkb layout, optionally with a variant: `us`, `jp`,
`us+dvorak`.

`kbd-layout-switch run` runs the watcher in the foreground without systemd.

## Behavior

- The layout applies to every keyboard. While the external keyboard is
  connected, the built-in keyboard uses its layout too.
- Input sources other than xkb, such as Mozc through IBus, are kept. Several
  xkb sources are replaced by the single selected layout.
- Right after a switch the xkb source (direct input) is selected. An input
  method keeps the keymap that was loaded last, so the program briefly leaves
  only the xkb source in the list to make GNOME Shell load the new keymap, then
  restores the others.

## Logs

```sh
journalctl --user -u kbd-layout-switch
```

## Uninstall

```sh
kbd-layout-switch uninstall
cargo uninstall kbd-layout-switch
```

## License

[MIT](LICENSE)
