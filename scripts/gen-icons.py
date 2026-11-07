#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Hariharan Narayanan

# Renders crates/app/assets/icon.svg to the PNG sizes the XDG icon theme and the Dioxus
# bundler need. Uses librsvg (what GNOME itself renders icons with) via GdkPixbuf.
# Not ImageMagick: with no rsvg delegate installed it silently falls back to its own SVG
# renderer, which drops the gradient and writes a black blob that looks nothing like the
# icon the app shows. The flat-render check below fails loudly if that ever happens again.

import sys
from pathlib import Path

import gi

gi.require_version("GdkPixbuf", "2.0")
from gi.repository import GdkPixbuf  # noqa: E402  (must follow require_version)

SIZES = (32, 48, 64, 128, 256, 512)

assets = Path(__file__).resolve().parent.parent / "crates" / "app" / "assets"
src = assets / "icon.svg"
dest_dir = assets / "icons"
dest_dir.mkdir(exist_ok=True)

for size in SIZES:
    pb = GdkPixbuf.Pixbuf.new_from_file_at_size(str(src), size, size)
    px, stride, chan = pb.get_pixels(), pb.get_rowstride(), pb.get_n_channels()
    colours = {
        bytes(px[y * stride + x * chan : y * stride + x * chan + 3])
        for y in range(pb.get_height())
        for x in range(pb.get_width())
    }
    if len(colours) < 8:
        sys.exit(f"{size}px render has only {len(colours)} colours - renderer is broken")
    dest = dest_dir / f"{size}.png"
    pb.savev(str(dest), "png", [], [])
    print(f"{dest.name}: {pb.get_width()}x{pb.get_height()}, {len(colours)} colours")
