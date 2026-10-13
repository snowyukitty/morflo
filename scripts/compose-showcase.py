"""Compose disclosed crops of real Morflo captures; never alter the captured UI."""

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageOps

parser = argparse.ArgumentParser()
parser.add_argument("--capture", type=Path, required=True)
parser.add_argument("--out", type=Path, required=True)
args = parser.parse_args()
args.out.mkdir(parents=True, exist_ok=True)

detail_path = args.capture / "ledger-detail.png"
detail = Image.open(detail_path).convert("RGB")
canvas = Image.new("RGB", (1200, 750), "#e7e5db")
draw = ImageDraw.Draw(canvas)
font_root = Path("C:/Windows/Fonts")
bold = ImageFont.truetype(str(font_root / "segoeuib.ttf"), 38)
regular = ImageFont.truetype(str(font_root / "segoeui.ttf"), 23)
small = ImageFont.truetype(str(font_root / "segoeui.ttf"), 13)
mark = Image.open("src-tauri/icons/128x128.png").convert("RGBA")
mark.thumbnail((48, 48), Image.Resampling.LANCZOS)
canvas.paste(mark, (29, 25), mark)
draw.text((88, 22), "morflo", font=bold, fill="#214f42")
draw.text((275, 31), "Originals in. Results in view.", font=regular, fill="#283d33")
draw.text((943, 39), "LOCAL FILES. REAL RESULTS.", font=small, fill="#445b4d")

# One continuous crop. No repainted controls, substituted figures or rearranged rows.
scaled = ImageOps.contain(detail, (1136, 620), Image.Resampling.LANCZOS)
left = (1200 - scaled.width) // 2
top = 98 + (620 - scaled.height) // 2
draw.rounded_rectangle((left-8, top-8, left+scaled.width+8, top+scaled.height+8), radius=14, fill="#f8f7f2", outline="#cfcec4", width=1)
canvas.paste(scaled, (left, top))
canvas.save(args.out / "morflo-1200x750.png", optimize=True)
for size in [(480, 300), (320, 200)]:
    canvas.resize(size, Image.Resampling.LANCZOS).save(args.out / f"morflo-{size[0]}x{size[1]}.png", optimize=True)
manifest = {
    "composition": "Branded title above one continuous, unaltered crop of real Tauri conversion results. The title and frame are presentation, not app controls.",
    "sourceCapture": "ledger-detail.png",
    "sourceSha256": hashlib.sha256(detail_path.read_bytes()).hexdigest(),
    "sourceCrop": json.loads((args.capture / "crop.json").read_text()),
    "sourceEvidence": json.loads((args.capture / "evidence.json").read_text()),
    "placement": {"x": left, "y": top, "width": scaled.width, "height": scaled.height},
    "resize": "Pillow Lanczos; uniform scaling, no retouching",
    "sizes": [[1200, 750], [480, 300], [320, 200]],
}
(args.out / "showcase-provenance.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print(args.out)
