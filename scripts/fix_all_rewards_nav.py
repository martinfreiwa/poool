import re
import glob

files = glob.glob("frontend/platform/*.html")

for file in files:
    with open(file, "r") as f:
        content = f.read()

    desktop_replacement = """<a id="nav-item-rewards-content" href="/rewards" class="sidebar__nav-item-content">
<div id="nav-text-icon-rewards" class="sidebar__nav-text-icon">
<div id="nav-icon-wrapper-rewards" class="sidebar__nav-icon-wrapper">
<img src="/images/star-01.svg" alt="Rewards" class="nav-icon" width="20" height="20">
</div>
<span id="nav-text-rewards" class="sidebar__nav-text">Rewards</span>
</div>
</a>"""

    desktop_pattern = r'<div id="nav-item-rewards-content" class="sidebar__nav-item-content sidebar__nav-item-content--disabled">[\s\S]*?<div class="soon-badge">[\s\S]*?</div>\s*</div>'
    
    content = re.sub(desktop_pattern, desktop_replacement, content)

    mobile_replacement = """<a href="/rewards" class="mobile-burger-menu__nav-item">
<div class="mobile-burger-menu__nav-icon-wrapper">
<img src="/images/star-01.svg" alt="Rewards" class="mobile-burger-menu__nav-icon">
</div>
<span class="mobile-burger-menu__nav-text">Rewards</span>
</a>"""

    mobile_pattern = r'<div class="mobile-burger-menu__nav-item mobile-burger-menu__nav-item--disabled">\s*<div class="mobile-burger-menu__nav-icon-wrapper">\s*<img src="/images/star-01\.svg" alt="Rewards" class="mobile-burger-menu__nav-icon">\s*</div>\s*<span class="mobile-burger-menu__nav-text">Rewards</span>\s*<span class="mobile-burger-menu__nav-badge soon-badge">Soon</span>\s*</div>'

    content = re.sub(mobile_pattern, mobile_replacement, content)
    
    with open(file, "w") as f:
        f.write(content)

print("Nav fixed globally!")
