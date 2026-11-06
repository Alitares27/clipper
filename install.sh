#!/bin/sh
set -eu
cd "$(dirname "$0")"
cargo build --locked --release
install -Dm755 "${CARGO_TARGET_DIR:-target}/release/clipper" "$HOME/.local/bin/clipper"
desktop_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
install -d "$desktop_dir"
install -Dm644 packaging/io.github.altairdeb.Clipper.svg "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps/io.github.altairdeb.Clipper.svg"
# Desktop launchers do not inherit interactive shell PATH reliably.
sed 's|^Exec=clipper |Exec="'"$HOME"'/.local/bin/clipper" |' \
    packaging/io.github.altairdeb.Clipper.desktop > "$desktop_dir/io.github.altairdeb.Clipper.desktop"
