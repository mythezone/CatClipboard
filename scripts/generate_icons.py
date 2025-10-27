from pathlib import Path
from PIL import Image

BASE_DIR = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
SOURCE_ICON = BASE_DIR / "icon.png"

if not SOURCE_ICON.exists():
    raise SystemExit(f"Source icon not found: {SOURCE_ICON}")

img = Image.open(SOURCE_ICON).convert("RGBA")

# Ensure we start from a sufficiently large canvas
if max(img.size) < 512:
    img = img.resize((512, 512), Image.LANCZOS)

# Generate PNG sizes expected by tauri bundle config
png_targets = {
    "32x32.png": (32, 32),
    "128x128.png": (128, 128),
    "128x128@2x.png": (256, 256),
}

for name, size in png_targets.items():
    resized = img.resize(size, Image.LANCZOS)
    resized.save(BASE_DIR / name)

# Generate ICO with multiple resolutions for Windows shell
ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
img.save(BASE_DIR / "icon.ico", sizes=ico_sizes)

# Generate ICNS for macOS bundle
icns_sizes = [(16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512)]
img.save(BASE_DIR / "icon.icns", format="ICNS", sizes=icns_sizes)

print("Generated icon assets in", BASE_DIR)
