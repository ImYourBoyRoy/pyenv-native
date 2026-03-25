import os
import sys
from playwright.sync_api import sync_playwright
from PIL import Image

# Explicit Assumptions: 
# 1. 'icon.svg' exists in the current working directory.
# 2. Network access is not required; rendering is done entirely locally via Chromium.

TARGETS = [
    {"name": "icon.png", "size": 512},
    {"name": "128x128@2x.png", "size": 256},
    {"name": "128x128.png", "size": 128},
    {"name": "32x32.png", "size": 32},
]

def build_pipeline():
    svg_file = "icon.svg"
    if not os.path.exists(svg_file):
        print(f"FATAL: {svg_file} missing from current directory.")
        sys.exit(1)

    # Read the raw SVG to inject into the DOM
    with open(svg_file, "r", encoding="utf-8") as f:
        svg_content = f.read()

    # Wrap the SVG in a borderless HTML body that forces it to fill the viewport
    html_payload = f"""
    <!DOCTYPE html>
    <html>
    <head>
    <style>
        body {{ margin: 0; padding: 0; overflow: hidden; background: transparent; }}
        svg {{ width: 100vw; height: 100vh; display: block; }}
    </style>
    </head>
    <body>
        {svg_content}
    </body>
    </html>
    """

    print("Initiating Headless Chromium Render Pipeline...")
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        
        for target in TARGETS:
            # Dynamically size the viewport to match the target raster resolution
            page = browser.new_page(viewport={"width": target["size"], "height": target["size"]})
            page.set_content(html_payload)
            
            # omit_background=True is critical to prevent Chromium from enforcing a white background
            page.screenshot(path=target["name"], omit_background=True)
            print(f" [+] Rendered Vector to Raster: {target['name']} ({target['size']}x{target['size']})")
            
            page.close()
            
        browser.close()

    print("\nInitiating ICO Binary Compilation...")
    try:
        # Load the 256x256 render to build the ICO. 
        # Standard Windows practice dictates embedding multiple sizes to prevent scaling artifacts.
        img = Image.open("128x128@2.png")
        icon_sizes = [(256, 256), (128, 128), (64, 64), (32, 32), (16, 16)]
        
        img.save("icon.ico", format="ICO", sizes=icon_sizes)
        print(" [+] Compiled: icon.ico (Multi-layer structure bound)")
    except Exception as e:
        print(f" [-] FATAL: Pillow failed to compile ICO. Error: {e}")
        sys.exit(1)

    print("\nPipeline Execution Complete.")

if __name__ == "__main__":
    build_pipeline()