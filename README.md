# kbd-layout-switch

Bluetooth キーボードの接続状態に合わせて、GNOME のキーボード配列を自動で切り替える常駐スクリプト。

- 外付けキーボード (既定: 名前が `Split65` で始まるデバイス) が接続中 → US 配列
- 未接続 → JIS 配列 (内蔵キーボード用)

GNOME にはデバイスごとに配列を割り当てる設定がないため、BlueZ の D-Bus イベントを監視して
`org.gnome.desktop.input-sources` の xkb ソースを書き換えている。

## 動作環境

- GNOME (Wayland / X11)
- BlueZ
- Python 3 + PyGObject (`python3-gobject`)

Fedora 45 / GNOME 51 で動作確認。

## インストール

```sh
./install.sh
```

`~/.local/bin/kbd-layout-switch` と systemd ユーザーサービスを配置して起動する。
以降はログイン時に自動で起動する。

## 設定

`kbd-layout-switch` 冒頭の定数を書き換える。

| 定数 | 既定値 | 意味 |
| --- | --- | --- |
| `DEVICE_NAME_PREFIX` | `Split65` | 対象デバイス名の先頭一致 |
| `EXTERNAL_LAYOUT` | `us` | 接続中の配列 |
| `INTERNAL_LAYOUT` | `jp` | 未接続時の配列 |

## 挙動メモ

- 配列は全キーボード共通。外付け接続中は内蔵キーボードも `EXTERNAL_LAYOUT` になる。
- xkb 以外の入力ソース (ibus の Mozc など) はそのまま残す。
- 切り替え直後は xkb ソース (直接入力) が選択された状態になる。
  新しいキーマップを確実に読み込ませるため、いったん xkb ソースだけにしてから元に戻している。

## ログ

```sh
journalctl --user -u kbd-layout-switch.service
```

## アンインストール

```sh
systemctl --user disable --now kbd-layout-switch.service
rm ~/.local/bin/kbd-layout-switch ~/.config/systemd/user/kbd-layout-switch.service
systemctl --user daemon-reload
```
