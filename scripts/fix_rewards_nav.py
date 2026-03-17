import re

files = ["frontend/platform/rewards.html", "frontend/platform/tier.html"]

for file in files:
    with open(file, "r") as f:
        content = f.read()

    # Mobile menu: making rewards active by regex
    # Before we do it, let's make sure it's correct.
    # Actually, mobile menu already matched as it had:
    # <a href="/rewards" class="mobile-burger-menu__nav-item active">

    # Desktop menu original:
    # <div id="nav-item-rewards-content" class="sidebar__nav-item-content sidebar__nav-item-content--disabled">
    # ...
    # </div>
    
    # We want to replace the whole <div id="nav-item-rewards-content" ... > ... </div> with an <a href="/rewards">
    
    replacement = """<a id="nav-item-rewards-content" href="/rewards" class="sidebar__nav-item-content">
<div id="nav-text-icon-rewards" class="sidebar__nav-text-icon">
<div id="nav-icon-wrapper-rewards" class="sidebar__nav-icon-wrapper">
<img src="/images/star-01.svg" alt="Rewards" class="nav-icon" width="20" height="20">
</div>
<span id="nav-text-rewards" class="sidebar__nav-text">Rewards</span>
</div>
</a>"""

    pattern = r'<div id="nav-item-rewards-content" class="sidebar__nav-item-content sidebar__nav-item-content--disabled">[\s\S]*?<div class="soon-badge">[\s\S]*?</div>\s*</div>'
    
    content = re.sub(pattern, replacement, content)

    with open(file, "w") as f:
        f.write(content)

print("Nav fixed")
