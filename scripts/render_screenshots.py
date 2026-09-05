import json
import os
from PIL import Image, ImageDraw, ImageFont

FONT_PATH = "/usr/share/fonts/Adwaita/AdwaitaMono-Regular.ttf"
FONT_BOLD_PATH = "/usr/share/fonts/Adwaita/AdwaitaMono-Bold.ttf"
FONT_SIZE = 15

os.makedirs("assets/screenshots", exist_ok=True)

try:
    font = ImageFont.truetype(FONT_PATH, FONT_SIZE)
    font_bold = ImageFont.truetype(FONT_BOLD_PATH, FONT_SIZE)
except Exception:
    font = ImageFont.load_default()
    font_bold = font

# Measure character dimensions
dummy_img = Image.new("RGBA", (100, 100))
dummy_draw = ImageDraw.Draw(dummy_img)
bbox = dummy_draw.textbbox((0, 0), "M", font=font)
char_width = max(bbox[2] - bbox[0], 9)
char_height = max(bbox[3] - bbox[1] + 6, 19)

def render_frame(json_path, output_png, window_title="mcpforge — terminal"):
    with open(json_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    cols = data["width"]
    rows = data["height"]

    padding_x = 24
    padding_bottom = 24
    header_height = 42

    term_width = cols * char_width
    term_height = rows * char_height

    total_width = term_width + (padding_x * 2)
    total_height = term_height + header_height + padding_bottom

    # Canvas with dark background
    canvas = Image.new("RGBA", (total_width, total_height), (18, 19, 26, 255))
    draw = ImageDraw.Draw(canvas)

    # Window title bar background
    draw.rectangle([(0, 0), (total_width, header_height)], fill=(28, 30, 42, 255))
    # Border under header
    draw.line([(0, header_height), (total_width, header_height)], fill=(45, 48, 66, 255), width=1)

    # Window control buttons (macOS / GNOME style)
    btn_y = header_height // 2
    draw.ellipse([(18, btn_y - 6), (30, btn_y + 6)], fill=(255, 95, 86, 255)) # Close
    draw.ellipse([(38, btn_y - 6), (50, btn_y + 6)], fill=(255, 189, 46, 255)) # Minimize
    draw.ellipse([(58, btn_y - 6), (70, btn_y + 6)], fill=(39, 201, 63, 255)) # Maximize

    # Header title text
    title_bbox = dummy_draw.textbbox((0, 0), window_title, font=font)
    title_w = title_bbox[2] - title_bbox[0]
    draw.text(((total_width - title_w) // 2, (header_height - (title_bbox[3] - title_bbox[1])) // 2), window_title, fill=(150, 155, 175, 255), font=font)

    # Render terminal cells
    content_y_start = header_height + 8

    for y, row in enumerate(data["cells"]):
        for x, cell in enumerate(row):
            ch = cell["ch"]
            fg = tuple(cell["fg"]) + (255,)
            bg = tuple(cell["bg"]) + (255,)
            bold = cell.get("bold", False)

            px = padding_x + (x * char_width)
            py = content_y_start + (y * char_height)

            # Draw background if not transparent / default
            if cell["bg"] not in [(0, 0, 0), (24, 25, 38), (18, 19, 26)]:
                draw.rectangle([(px, py), (px + char_width, py + char_height)], fill=bg)

            # Draw text
            if ch.strip():
                fnt = font_bold if bold else font
                draw.text((px, py), ch, fill=fg, font=fnt)

    # Outer border
    draw.rectangle([(0, 0), (total_width - 1, total_height - 1)], outline=(60, 64, 86, 255), width=1)

    canvas.save(output_png, "PNG")
    print(f"Rendered {output_png} ({total_width}x{total_height})")

render_frame("screenshot_dashboard.json", "assets/screenshots/dashboard.png", "mcpforge — [1] Configured Servers & Health Monitor")
render_frame("screenshot_inspector_clients.json", "assets/screenshots/inspector_clients.png", "mcpforge — Segmented Server Inspector (Sub-Tab 2: Configured Clients)")
render_frame("screenshot_clients.json", "assets/screenshots/clients.png", "mcpforge — [2] Clients & Agent Harness Matrix (27 Supported)")
render_frame("screenshot_wizard_catalog.json", "assets/screenshots/catalog.png", "mcpforge — Add MCP Server (Step 2: Curated 110-Server Catalog)")
render_frame("screenshot_wizard_diff.json", "assets/screenshots/diff_preview.png", "mcpforge — Add MCP Server (Step 4: Unified Atomic Diff Preview)")
render_frame("screenshot_delete_modal.json", "assets/screenshots/removal_modal.png", "mcpforge — Remove MCP Server Modal")
render_frame("screenshot_form_builder.json", "assets/screenshots/form_builder.png", "mcpforge — Interactive Schema-Guided Form Builder ([f] key)")
render_frame("screenshot_backup_modal.json", "assets/screenshots/backup_manager.png", "mcpforge — Configuration Snapshots & Diff Inspector ([b] key)")
render_frame("screenshot_client_config_modal.json", "assets/screenshots/client_config_modal.png", "mcpforge — Configuration File Inspector ([v] key)")
