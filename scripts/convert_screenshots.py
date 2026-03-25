# ./scripts/convert_screenshots.py
"""
Converts PNG screenshots in docs/screenshots to WebP and creates an animated sequence.
Also removes the original PNGs.
"""
import os
from PIL import Image

def main():
    base_dir = r"docs/screenshots"
    sequence = [
        "Dashboard",
        "Installed_Versions",
        "VENVs",
        "Available",
        "Settings",
        "About"
    ]
    
    # 1. Convert all PNGs to WebP
    webp_images = []
    for name in sequence:
        png_path = os.path.join(base_dir, f"{name}.png")
        webp_path = os.path.join(base_dir, f"{name}.webp")
        
        if os.path.exists(png_path):
            with Image.open(png_path) as img:
                img.save(webp_path, "WEBP", quality=90)
                print(f"Converted {png_path} -> {webp_path}")
                webp_images.append(Image.open(webp_path))
        else:
            print(f"Warning: {png_path} not found.")

    # 2. Create animated WebP if we have images
    if webp_images:
        animated_path = os.path.join(base_dir, "animated_gui.webp")
        # Save as animated webp with 2-second duration per frame
        webp_images[0].save(
            animated_path,
            save_all=True,
            append_images=webp_images[1:],
            duration=2000,
            loop=0,
            quality=85,
            method=6 # highest compression
        )
        print(f"Created animated sequence: {animated_path}")

    # 3. Cleanup PNGs
    for name in sequence:
        png_path = os.path.join(base_dir, f"{name}.png")
        if os.path.exists(png_path):
            os.remove(png_path)
            print(f"Removed {png_path}")

if __name__ == "__main__":
    main()
