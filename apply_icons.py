import re

with open("frontend/platform/static/css/portfolio-enhancements.css", "a") as f:
    f.write("""

/* ── Premium Admin Icon Shell adopted for Portfolio ── */
.p-icon{position:relative;width:56px;height:56px;flex-shrink:0;transition:transform .35s cubic-bezier(.4,0,.2,1);will-change:transform;transform:translateZ(0);}
.p-icon::before{content:'';position:absolute;inset:4px;border-radius:22.37%;background:var(--glow-color,rgba(0,0,255,.06));filter:blur(14px);opacity:0;transition:opacity .45s cubic-bezier(.4,0,.2,1);z-index:0;}
.quick-insights-card:hover .p-icon::before, .key-financials-card:hover .p-icon::before{opacity:1;}
.p-bg{position:absolute;border-radius:22.37%;transition:all .4s cubic-bezier(.4,0,.2,1);will-change:transform;}
.p-bg-1{width:48px;height:48px;opacity:0.3;transform:rotate(18deg) translateZ(0);left:9px;top:-5px;z-index:1;}
.p-bg-2{width:52px;height:52px;opacity:0.38;transform:rotate(10deg) translateZ(0);left:4px;top:-3px;z-index:2;}
.p-front{position:absolute;inset:0;width:56px;height:56px;border-radius:22.37%;z-index:3;display:flex;align-items:center;justify-content:center;transition:all .4s cubic-bezier(.4,0,.2,1);overflow:hidden;}
.p-front::after{content:'';position:absolute;inset:0;border-radius:inherit;opacity:.015;mix-blend-mode:soft-light;background:url("data:image/svg+xml,%3Csvg viewBox='0 0 512 512' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.75' numOctaves='5' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");pointer-events:none;z-index:6;}
.p-front::before{content:'';position:absolute;top:-20%;left:-20%;width:90%;height:90%;border-radius:50%;background:radial-gradient(ellipse at 30% 20%,rgba(255,255,255,.22) 0%,rgba(255,255,255,.08) 40%,transparent 70%);z-index:5;pointer-events:none;}
.p-front svg{width:22px;height:22px;stroke-width:1.5;fill:none;stroke-linecap:round;stroke-linejoin:round;position:relative;z-index:7;}
""")

# Define SVGs
SVGS = {
    'wallet': '<path d="M20 4H4a2 2 0 00-2 2v12a2 2 0 002 2h16a2 2 0 002-2V6a2 2 0 00-2-2z"/><path d="M2 10h20"/><circle cx="17" cy="15" r="1.5" fill="currentColor" stroke="none"/>',
    'bldg': '<rect x="4" y="2" width="16" height="20" rx="2"/><path d="M12 22v-4h-3v4M9 6h1.01M14 6h1.01M9 10h1.01M14 10h1.01M9 14h1.01M14 14h1.01" stroke-width="2.5" stroke-linecap="round"/>',
    'lineUp': '<polyline points="22 7 13 16 8 11 2 17"/><polyline points="16 7 22 7 22 13"/>',
    'home': '<path d="M3 10l9-7 9 7v10a2 2 0 01-2 2H5a2 2 0 01-2-2Z"/><path d="M9 22V12h6v10"/>',
    'cal': '<rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/><path d="M12 13v4M10 15h4"/>',
    'award': '<path d="M8 21h8"/><path d="M12 17v4"/><path d="M7 4h10v4c0 2.8-2.2 5-5 5s-5-2.2-5-5V4z"/><path d="M7 8H4V5h3"/><path d="M17 8h3V5h-3"/>',
}

# Styles
BLUE = {
    'glow': 'rgba(0,0,255,.1)',
    'bg1': 'background:linear-gradient(135deg,#03FF88 0%,#00CC6F 100%);opacity:0.28;box-shadow:0 2px 8px rgba(3,255,136,.12);',
    'bg2': 'background:linear-gradient(135deg,#0000FF 0%,#3344FF 100%);opacity:0.32;box-shadow:0 2px 8px rgba(0,0,255,.1);',
    'front': 'background:linear-gradient(160deg,rgba(255,255,255,.95) 0%,rgba(244,246,255,.78) 40%,rgba(236,240,255,.62) 100%);border:0;box-shadow:0 .5px .5px rgba(0,0,0,.04),0 1px 2px rgba(0,0,0,.04),0 4px 12px rgba(0,0,255,.04),0 8px 24px rgba(0,0,255,.025),inset 0 .5px 0 rgba(255,255,255,.95),inset 0 0 0 .5px rgba(255,255,255,.7),inset 0 -1px 3px rgba(180,195,255,.08);backdrop-filter:blur(20px)saturate(1.85);-webkit-backdrop-filter:blur(20px)saturate(1.85);',
    'stroke': '#0000FF',
    'strokeShadow': '0 .5px 0 rgba(0,0,200,.08)'
}
GREEN = {
    'glow': 'rgba(0,187,102,.1)',
    'bg1': 'background:linear-gradient(135deg,#03FF88 0%,#00CC6F 100%);opacity:0.32;box-shadow:0 2px 8px rgba(3,255,136,.14);',
    'bg2': 'background:linear-gradient(135deg,#0000FF 0%,#3344FF 100%);opacity:0.28;box-shadow:0 2px 8px rgba(0,0,255,.08);',
    'front': 'background:linear-gradient(160deg,rgba(255,255,255,.95) 0%,rgba(245,255,250,.78) 40%,rgba(238,255,245,.62) 100%);border:0;box-shadow:0 .5px .5px rgba(0,0,0,.04),0 1px 2px rgba(0,0,0,.04),0 4px 12px rgba(3,200,100,.035),0 8px 24px rgba(3,200,100,.02),inset 0 .5px 0 rgba(255,255,255,.95),inset 0 0 0 .5px rgba(255,255,255,.7),inset 0 -1px 3px rgba(200,255,230,.08);backdrop-filter:blur(20px)saturate(1.85);-webkit-backdrop-filter:blur(20px)saturate(1.85);',
    'stroke': '#00BB66',
    'strokeShadow': '0 .5px 0 rgba(0,140,80,.08)'
}

def generate_html(path_name, style):
    svg = SVGS[path_name]
    return f"""<div class="p-icon" style="--glow-color:{style['glow']};">
  <div class="p-bg p-bg-1" style="{style['bg1']}"></div>
  <div class="p-bg p-bg-2" style="{style['bg2']}"></div>
  <div class="p-front" style="{style['front']}">
    <svg viewBox="0 0 24 24" stroke="{style['stroke']}" fill="none" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" style="filter:drop-shadow({style['strokeShadow']});">
      {svg}
    </svg>
  </div>
</div>"""

with open("frontend/platform/portfolio.html", "r") as f:
    html = f.read()

# Replace Monthly income (wallet, blue)
html = re.sub(
    r'<div class="glass-icon-container">.*?</div>\s*</div>',
    generate_html('wallet', BLUE),
    html, count=1, flags=re.DOTALL
)

# Replace Total rental (bldg, green)
html = re.sub(
    r'<div class="glass-icon-container">.*?</div>\s*</div>',
    generate_html('bldg', GREEN),
    html, count=1, flags=re.DOTALL
)

# Replace Total appreciation (lineUp, blue)
html = re.sub(
    r'<div class="glass-icon-container">.*?</div>\s*</div>',
    generate_html('lineUp', BLUE),
    html, count=1, flags=re.DOTALL
)

# Replace Number of properties (home, blue)
html = re.sub(
    r'<div class="glass-icon-container">.*?</div>\s*</div>',
    generate_html('home', BLUE),
    html, count=1, flags=re.DOTALL
)

# Replace Occupancy rate (cal, green)
html = re.sub(
    r'<div class="glass-icon-container">.*?</div>\s*</div>',
    generate_html('cal', GREEN),
    html, count=1, flags=re.DOTALL
)

# Replace Annual rental yield (award, green)
html = re.sub(
    r'<div class="glass-icon-container">.*?</div>\s*</div>',
    generate_html('award', GREEN),
    html, count=1, flags=re.DOTALL
)

with open("frontend/platform/portfolio.html", "w") as f:
    f.write(html)

print("done")
