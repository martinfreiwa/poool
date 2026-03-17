import re

with open("frontend/platform/portfolio.html", "r") as f:
    html = f.read()

# Splitting before main
head_content = html.split('<main id="portfolio-main"')[0]
footer_content = '\n\t</div>\n\t<script src="/static/js/mobile-navigation.js"></script>\n</body>\n</html>'

# Replace Portfolio active states -> inactive
def make_portfolio_inactive(s):
    s = s.replace('class="mobile-burger-menu__nav-item active"', 'class="mobile-burger-menu__nav-item"')
    s = s.replace('id="nav-item-portfolio" class="sidebar__nav-item sidebar__nav-item--active"', 'id="nav-item-portfolio" class="sidebar__nav-item"')
    return s

def make_rewards_active(s):
    # Desktop
    # We replace the disabled rewards block
    desktop_rewards_old = """<div id="nav-item-rewards-content" class="sidebar__nav-item-content sidebar__nav-item-content--disabled">
<div id="nav-text-icon-rewards" class="sidebar__nav-text-icon">
<div id="nav-icon-wrapper-rewards" class="sidebar__nav-icon-wrapper">
<img src="/images/star-01.svg" alt="Rewards" class="nav-icon" width="20" height="20">
</div>
<span id="nav-text-rewards" class="sidebar__nav-text">Rewards</span>
</div>
<!-- Soon Badge for Rewards and Leaderboard tabs -->
<div class="soon-badge">
<span>Soon</span>
</div>
</div>"""
    desktop_rewards_new = """<a id="nav-item-rewards-content" href="/rewards" class="sidebar__nav-item-content">
<div id="nav-text-icon-rewards" class="sidebar__nav-text-icon">
<div id="nav-icon-wrapper-rewards" class="sidebar__nav-icon-wrapper">
<img src="/images/star-01.svg" alt="Rewards" class="nav-icon" width="20" height="20">
</div>
<span id="nav-text-rewards" class="sidebar__nav-text">Rewards</span>
</div>
</a>"""
    # Fix whitespace variations
    s = re.sub(r'<div id="nav-item-rewards-content" class="sidebar__nav-item-content sidebar__nav-item-content--disabled">[\s\S]*?<div class="soon-badge">[\s\S]*?</div>\n\t*</div>', 
               desktop_rewards_new, s)
    s = s.replace('id="nav-item-rewards" class="sidebar__nav-item"', 'id="nav-item-rewards" class="sidebar__nav-item sidebar__nav-item--active"')
    
    # Mobile
    mobile_rewards_old = """<div class="mobile-burger-menu__nav-item mobile-burger-menu__nav-item--disabled">
					<div class="mobile-burger-menu__nav-icon-wrapper"><img src="/images/star-01.svg" alt="Rewards"
							class="mobile-burger-menu__nav-icon"></div><span
						class="mobile-burger-menu__nav-text">Rewards</span> <span
						class="mobile-burger-menu__nav-badge soon-badge">Soon</span>
				</div>"""
    mobile_rewards_new = """<a href="/rewards" class="mobile-burger-menu__nav-item active">
					<div class="mobile-burger-menu__nav-icon-wrapper"><img src="/images/star-01.svg" alt="Rewards"
							class="mobile-burger-menu__nav-icon"></div><span
						class="mobile-burger-menu__nav-text">Rewards</span>
				</a>"""
    s = re.sub(r'<div class="mobile-burger-menu__nav-item mobile-burger-menu__nav-item--disabled">\s*<div class="mobile-burger-menu__nav-icon-wrapper">\s*<img src="/images/star-01.svg" alt="Rewards"\s*class="mobile-burger-menu__nav-icon">\s*</div>\s*<span\s*class="mobile-burger-menu__nav-text">Rewards</span>\s*<span\s*class="mobile-burger-menu__nav-badge soon-badge">Soon</span>\s*</div>', 
               mobile_rewards_new, s)
    return s

rewards_head = make_rewards_active(make_portfolio_inactive(head_content))
# title
rewards_head = rewards_head.replace("<title>Portfolio - POOOL</title>", "<title>Rewards - POOOL</title>")
rewards_head = rewards_head.replace('id="portfolio-body"', 'id="rewards-body"')

# we also need to include CSS for rewards
# add <link rel="stylesheet" href="/static/css/rewards.css"> before custom javascript
rewards_head = rewards_head.replace("<!-- Custom JavaScript -->", '<link rel="stylesheet" href="/static/css/rewards.css">\n\t<!-- Custom JavaScript -->')

# Generate rewards.html
rewards_main = f"""
\t<!-- Main Content Area -->
\t<main id="rewards-main" class="rewards-main">
\t\t<div class="rewards-container">
\t\t\tREWARDS_CONTENT_PLACEHOLDER
\t\t</div>
\t</main>
"""

with open("frontend/platform/rewards.html", "w") as f:
    f.write(rewards_head + rewards_main + footer_content)

# Generate tier.html
tier_head = rewards_head.replace("<title>Rewards - POOOL</title>", "<title>Tier - POOOL</title>")
tier_main = f"""
\t<!-- Main Content Area -->
\t<main id="tier-main" class="tier-main">
\t\t<div class="tier-container">
\t\t\tTIER_CONTENT_PLACEHOLDER
\t\t</div>
\t</main>
"""
with open("frontend/platform/tier.html", "w") as f:
    f.write(tier_head + tier_main + footer_content)

print("Scaffolded both pages.")
