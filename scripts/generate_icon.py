from pathlib import Path
from PIL import Image,ImageDraw
import xml.etree.ElementTree as ET
root=Path(__file__).resolve().parents[1]
# The canonical SVG is generated directly from the official pinned Tabler component.
# Render through resvg-js in generate-icon.cjs; this script wraps the PNG into ICO.
path=root/'desktop/src-tauri/icons/icon.png'
image=Image.open(path).convert('RGBA')
image.save(path.with_suffix('.ico'),sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
