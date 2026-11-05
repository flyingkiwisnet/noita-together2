"""Original NT2 geometric icon, no font/game/third-party art. MIT NT2 contributors."""
from pathlib import Path
from PIL import Image,ImageDraw
import io
root=Path(__file__).resolve().parents[1]/'src-tauri/icons'
paths=[[(25,82),(25,46),(43,82),(43,46)],[(53,46),(77,46)],[(65,46),(65,82)],[(87,47),(103,47),(103,63),(87,63),(87,81),(103,81)]]
svg='<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><rect x="5" y="5" width="118" height="118" rx="20" fill="#102130" stroke="#90ddbe" stroke-width="6"/>'
for line in paths:svg+='<polyline points="'+' '.join(str(x)+','+str(y) for x,y in line)+'" fill="none" stroke="#edf6f2" stroke-width="5" stroke-linecap="square" stroke-linejoin="miter"/>'
(root/'beta-icon.svg').write_text(svg+'</svg>\n',encoding='utf-8')
image=Image.new('RGBA',(512,512),(0,0,0,0));d=ImageDraw.Draw(image);d.rounded_rectangle((20,20,492,492),radius=80,fill='#102130',outline='#90ddbe',width=24)
for line in paths:d.line([(x*4,y*4) for x,y in line],fill='#edf6f2',width=20,joint='curve')
image=image.resize((128,128),Image.Resampling.LANCZOS);image.save(root/'icon.png');image.save(root/'icon.ico',sizes=[(16,16),(32,32),(48,48),(128,128)])
print('Generated original font-free NT2 beta icon and SVG source')
