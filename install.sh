#!/bin/sh
# Install the script and the systemd user service, then start it.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
install -Dm755 "$here/kbd-layout-switch" "$HOME/.local/bin/kbd-layout-switch"
install -Dm644 "$here/kbd-layout-switch.service" \
    "$HOME/.config/systemd/user/kbd-layout-switch.service"
systemctl --user daemon-reload
systemctl --user enable kbd-layout-switch.service
systemctl --user restart kbd-layout-switch.service
