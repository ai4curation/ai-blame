#!/usr/bin/env python3
"""
Generate PNG icons from SVG for Tauri bundling.
Requires: pip install cairosvg
"""
import os
import sys

try:
    import cairosvg
except ImportError as e:
    print(f"Error: Missing required library: {e}")
    print("Install with: pip install cairosvg")
    sys.exit(1)

svg_path = "docs/assets/favicon.svg"
output_dir = "src-tauri/icons"
sizes = [32, 128, 256, 512, 1024]

os.makedirs(output_dir, exist_ok=True)

with open(svg_path, 'rb') as f:
    svg_data = f.read()

png_512 = None

for size in sizes:
    png_data = cairosvg.svg2png(bytestring=svg_data, output_width=size, output_height=size)
    
    # Save 512x512 data for reuse as icon.png
    if size == 512:
        png_512 = png_data
    
    # Determine output filename
    if size == 256:
        output_path = os.path.join(output_dir, "128x128@2x.png")
    else:
        output_path = os.path.join(output_dir, f"{size}x{size}.png")
    
    with open(output_path, 'wb') as f:
        f.write(png_data)
    print(f"Generated {output_path}")

# Reuse 512x512 PNG as the main icon
if png_512:
    with open(os.path.join(output_dir, "icon.png"), 'wb') as f:
        f.write(png_512)
    print(f"Generated {os.path.join(output_dir, 'icon.png')}")

print("\nNext steps:")
print("1. Install tauri-cli: cargo install tauri-cli --version '^1.5'")
print("2. Generate .icns and .ico: cargo tauri icon src-tauri/icons/icon.png")
