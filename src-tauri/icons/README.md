# Tauri App Icons

This directory contains the application icons used for bundling the Tauri desktop app.

## Required Icons

For proper cross-platform bundling, Tauri needs:
- **PNG files**: 32x32.png, 128x128.png, 128x128@2x.png (256x256), icon.png (512x512 or higher)
- **macOS**: icon.icns (generated from PNGs)
- **Windows**: icon.ico (generated from PNGs)

## Generating Icons

To generate proper icons from the SVG source:

1. Install dependencies:
   ```bash
   pip install cairosvg
   ```

2. Run the icon generation script:
   ```bash
   python3 tools/generate_icons.py
   ```

3. Or manually convert from SVG:
   ```bash
   # From the repository root
   # Using ImageMagick
   convert -background none -resize 32x32 docs/assets/favicon.svg src-tauri/icons/32x32.png
   convert -background none -resize 128x128 docs/assets/favicon.svg src-tauri/icons/128x128.png
   convert -background none -resize 256x256 docs/assets/favicon.svg src-tauri/icons/128x128@2x.png
   convert -background none -resize 512x512 docs/assets/favicon.svg src-tauri/icons/icon.png
   ```

4. For .icns and .ico, use Tauri's icon generation tool:
   ```bash
   cargo install tauri-cli --version "^1.5"
   cargo tauri icon src-tauri/icons/icon.png
   ```

## Current Status

The placeholder `icon.png` (1x1) should be replaced with proper high-resolution icons before release.
See: https://tauri.app/v1/guides/features/icons
