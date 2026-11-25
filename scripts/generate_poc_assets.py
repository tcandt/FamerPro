import os
import shutil
import json
from PIL import Image, ImageDraw, ImageFont

REPO_ROOT = "e:/DATA/FarmerPro"

def draw_rounded_shield(size, bg_top, bg_bot, border_col, accent_col, glyph_type):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    pad = max(2, int(size * 0.06))
    rad = max(4, int(size * 0.22))

    # Draw gradient rounded rectangle background
    for y in range(pad, size - pad):
        t = (y - pad) / max(1, (size - 2 * pad))
        r = int(bg_top[0] * (1 - t) + bg_bot[0] * t)
        g = int(bg_top[1] * (1 - t) + bg_bot[1] * t)
        b = int(bg_top[2] * (1 - t) + bg_bot[2] * t)
        draw.line([(pad, y), (size - pad - 1, y)], fill=(r, g, b, 255))

    # Mask to rounded rectangle
    mask = Image.new("L", (size, size), 0)
    mdraw = ImageDraw.Draw(mask)
    mdraw.rounded_rectangle([pad, pad, size - pad - 1, size - pad - 1], radius=rad, fill=255)
    base = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    base.paste(img, (0, 0), mask)
    draw = ImageDraw.Draw(base)

    bw = max(1, int(size * 0.045))
    draw.rounded_rectangle(
        [pad, pad, size - pad - 1, size - pad - 1],
        radius=rad,
        outline=border_col + (255,),
        width=bw,
    )

    cx, cy = size // 2, size // 2
    if glyph_type == "studio":
        # Cyber workflow nodes + vision crosshair
        r_outer = int(size * 0.27)
        r_inner = int(size * 0.11)
        lw = max(2, int(size * 0.055))
        draw.ellipse([cx - r_outer, cy - r_outer, cx + r_outer, cy + r_outer], outline=accent_col + (255,), width=lw)
        draw.ellipse([cx - r_inner, cy - r_inner, cx + r_inner, cy + r_inner], fill=(59, 195, 141, 255))
        # Crosshair ticks
        tick = int(size * 0.35)
        draw.line([(cx, cy - tick), (cx, cy - r_inner - 2)], fill=(255, 255, 255, 240), width=lw)
        draw.line([(cx, cy + r_inner + 2), (cx, cy + tick)], fill=(255, 255, 255, 240), width=lw)
        draw.line([(cx - tick, cy), (cx - r_inner - 2, cy)], fill=(255, 255, 255, 240), width=lw)
        draw.line([(cx + r_inner + 2, cy), (cx + tick, cy)], fill=(255, 255, 255, 240), width=lw)
    elif glyph_type == "jx2":
        # Martial arts golden diamond/blade emblem
        s = int(size * 0.30)
        lw = max(2, int(size * 0.06))
        pts = [(cx, cy - s), (cx + s, cy), (cx, cy + s), (cx - s, cy)]
        draw.polygon(pts, fill=(180, 35, 24, 230), outline=accent_col + (255,))
        # Inner sword/vertical crest
        draw.line([(cx, cy - int(size * 0.24)), (cx, cy + int(size * 0.24))], fill=(255, 235, 160, 255), width=lw)
        draw.line([(cx - int(size * 0.14), cy - int(size * 0.05)), (cx + int(size * 0.14), cy - int(size * 0.05))], fill=(255, 235, 160, 255), width=lw)
    elif glyph_type == "slg":
        # Tactical SLG Crown / Fortress shield emblem
        s = int(size * 0.28)
        pts = [
            (cx - s, cy + int(s * 0.7)),
            (cx - s, cy - int(s * 0.5)),
            (cx - int(s * 0.45), cy),
            (cx, cy - int(s * 0.85)),
            (cx + int(s * 0.45), cy),
            (cx + s, cy - int(s * 0.5)),
            (cx + s, cy + int(s * 0.7)),
        ]
        draw.polygon(pts, fill=accent_col + (255,), outline=(255, 255, 255, 240))
    return base

def save_icon_set(out_dir, bg_top, bg_bot, border_col, accent_col, glyph_type):
    os.makedirs(out_dir, exist_ok=True)
    sizes = [16, 24, 32, 48, 64, 128, 256]
    imgs = {s: draw_rounded_shield(s, bg_top, bg_bot, border_col, accent_col, glyph_type) for s in sizes}
    img512 = draw_rounded_shield(512, bg_top, bg_bot, border_col, accent_col, glyph_type)

    imgs[32].save(os.path.join(out_dir, "32x32.png"), format="PNG")
    imgs[128].save(os.path.join(out_dir, "128x128.png"), format="PNG")
    imgs[256].save(os.path.join(out_dir, "128x128@2x.png"), format="PNG")
    img512.save(os.path.join(out_dir, "icon.png"), format="PNG")

    ico_path = os.path.join(out_dir, "icon.ico")
    imgs[256].save(
        ico_path,
        format="ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    print(f"Generated icon set in {out_dir}: icon.ico ({os.path.getsize(ico_path)} bytes)")

def create_template_png(path, w, h, rgb):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    im = Image.new("RGB", (w, h), rgb)
    im.save(path, format="PNG")

def main():
    # 1. Studio Branding Icons (in assets/branding/studio, NOT overwriting existing src-tauri yet or backing up first)
    save_icon_set(
        os.path.join(REPO_ROOT, "assets", "branding", "studio"),
        bg_top=(15, 23, 42),
        bg_bot=(24, 120, 245),
        border_col=(56, 189, 248),
        accent_col=(56, 189, 248),
        glyph_type="studio",
    )

    # 2. Game 1 PoC: Võ Hiệp Truyền Kỳ 2 Mobile (projects/poc/jx2-mobile)
    jx2_dir = os.path.join(REPO_ROOT, "projects", "poc", "jx2-mobile")
    save_icon_set(
        os.path.join(jx2_dir, "branding"),
        bg_top=(68, 18, 20),
        bg_bot=(153, 27, 27),
        border_col=(245, 158, 11),
        accent_col=(251, 191, 36),
        glyph_type="jx2",
    )
    # Copy existing JX2 templates & sample workflows into PoC project folder
    os.makedirs(os.path.join(jx2_dir, "templates"), exist_ok=True)
    os.makedirs(os.path.join(jx2_dir, "workflows"), exist_ok=True)
    src_jx2 = os.path.join(REPO_ROOT, "games", "jx2mobile")
    for fn in os.listdir(os.path.join(src_jx2, "templates")):
        if fn.endswith(".png"):
            shutil.copy2(os.path.join(src_jx2, "templates", fn), os.path.join(jx2_dir, "templates", fn))
    # Also create deterministic test template compatible with FakeDeviceDriver (24, 120, 245) for PoC headless test
    create_template_png(os.path.join(jx2_dir, "templates", "poc_quest_marker.png"), 120, 60, (24, 120, 245))

    # 3. Game 2 PoC: Kingdom Rise: Tactics SLG (projects/poc/kingdom-slg)
    slg_dir = os.path.join(REPO_ROOT, "projects", "poc", "kingdom-slg")
    save_icon_set(
        os.path.join(slg_dir, "branding"),
        bg_top=(17, 24, 39),
        bg_bot=(79, 70, 229),
        border_col=(167, 139, 250),
        accent_col=(56, 189, 248),
        glyph_type="slg",
    )
    os.makedirs(os.path.join(slg_dir, "templates"), exist_ok=True)
    os.makedirs(os.path.join(slg_dir, "workflows"), exist_ok=True)
    create_template_png(os.path.join(slg_dir, "templates", "btn_harvest_food.png"), 120, 60, (24, 120, 245))
    create_template_png(os.path.join(slg_dir, "templates", "btn_train_cavalry.png"), 120, 60, (24, 120, 245))
    create_template_png(os.path.join(slg_dir, "templates", "btn_alliance_help.png"), 120, 60, (24, 120, 245))

if __name__ == "__main__":
    main()
