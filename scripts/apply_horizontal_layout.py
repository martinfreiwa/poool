import re

with open("frontend/platform/portfolio.html", "r") as f:
    html = f.read()

# 1. Fix Portfolio Value collapsed
html = html.replace('x-data="{ expanded: true, activeTab: \'twelveMonths\' }"', 'x-data="{ expanded: false, activeTab: \'twelveMonths\' }"')
html = html.replace('<span>Show more</span>\n                  <svg width="20" height="20" viewBox="0 0 20 20" fill="none" class="transition-transform duration-200">', 
                    '<span x-text="expanded ? \'Show less\' : \'Show more\'">Show more</span>\n                  <svg width="20" height="20" viewBox="0 0 20 20" fill="none" class="transition-transform duration-200" :class="expanded ? \'rotate-180\' : \'\'">')

# Definitions for 6 cards
SVGS = {
    'wallet': '<path d="M20 4H4a2 2 0 00-2 2v12a2 2 0 002 2h16a2 2 0 002-2V6a2 2 0 00-2-2z"/><path d="M2 10h20"/><circle cx="17" cy="15" r="1.5" fill="currentColor" stroke="none"/>',
    'bldg': '<rect x="4" y="2" width="16" height="20" rx="2"/><path d="M12 22v-4h-3v4M9 6h1.01M14 6h1.01M9 10h1.01M14 10h1.01M9 14h1.01M14 14h1.01" stroke-width="2.5" stroke-linecap="round"/>',
    'lineUp': '<polyline points="22 7 13 16 8 11 2 17"/><polyline points="16 7 22 7 22 13"/>',
    'home': '<path d="M3 10l9-7 9 7v10a2 2 0 01-2 2H5a2 2 0 01-2-2Z"/><path d="M9 22V12h6v10"/>',
    'cal': '<rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/><path d="M12 13v4M10 15h4"/>',
    'award': '<path d="M8 21h8"/><path d="M12 17v4"/><path d="M7 4h10v4c0 2.8-2.2 5-5 5s-5-2.2-5-5V4z"/><path d="M7 8H4V5h3"/><path d="M17 8h3V5h-3"/>',
}

# Ensure ALL are blue per user request
BLUE = {
    'glow': 'rgba(0,0,255,.1)',
    'bg1': 'background:linear-gradient(135deg,#03FF88 0%,#00CC6F 100%);opacity:0.28;box-shadow:0 2px 8px rgba(3,255,136,.12);',
    'bg2': 'background:linear-gradient(135deg,#0000FF 0%,#3344FF 100%);opacity:0.32;box-shadow:0 2px 8px rgba(0,0,255,.1);',
    'front': 'background:linear-gradient(160deg,rgba(255,255,255,.95) 0%,rgba(244,246,255,.78) 40%,rgba(236,240,255,.62) 100%);border:0;box-shadow:0 .5px .5px rgba(0,0,0,.04),0 1px 2px rgba(0,0,0,.04),0 4px 12px rgba(0,0,255,.04),0 8px 24px rgba(0,0,255,.025),inset 0 .5px 0 rgba(255,255,255,.95),inset 0 0 0 .5px rgba(255,255,255,.7),inset 0 -1px 3px rgba(180,195,255,.08);backdrop-filter:blur(20px)saturate(1.85);-webkit-backdrop-filter:blur(20px)saturate(1.85);',
    'stroke': '#0000FF',
    'strokeShadow': '0 .5px 0 rgba(0,0,200,.08)'
}

def generate_card(id_attr, base_id, title, svgs_key, show_currency=True):
    svg = SVGS[svgs_key]
    style = BLUE
    currency_str = '<span class="financials-currency" style="font-size: 24px;">$</span>' if show_currency else ''

    # Notice that ID mapping for bottom cards in JS is different, 
    # e.g., "insights-value-number-of-properties" instead of "portfolio-number-of-properties".
    value_id = f"portfolio-{base_id}" if "insights-value" not in base_id else base_id

    return f"""<div id="{id_attr}" class="quick-insights-card">
              <div class="insights-content" style="display: flex; flex-direction: row; align-items: center; gap: 16px;">
                <div class="insights-icon-wrapper">
                  <div class="p-icon" style="--glow-color:{style['glow']};">
                    <div class="p-bg p-bg-1" style="{style['bg1']}"></div>
                    <div class="p-bg p-bg-2" style="{style['bg2']}"></div>
                    <div class="p-front" style="{style['front']}">
                      <svg viewBox="0 0 24 24" stroke="{style['stroke']}" fill="none" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" style="filter:drop-shadow({style['strokeShadow']});">
                        {svg}
                      </svg>
                    </div>
                  </div>
                </div>
                <div style="display: flex; flex-direction: column; gap: 4px; flex-grow: 1;">
                  <div class="insights-header" style="margin-bottom:0;">
                    <span class="insights-title" style="white-space:nowrap;">{title}</span>
                  </div>
                  <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
                    <div class="insights-value" style="display:flex; align-items:center;">
                      {currency_str}
                      <span id="{value_id}" class="financials-amount" style="font-size: 24px;">...</span>
                    </div>
                    <!-- All cards have % badge per user request -->
                    <div class="financials-change change-increase" style="position:static; height:24px; display:inline-flex; align-items:center; padding: 2px 8px; border-radius: 6px; border: 1px solid #e5e7eb;">
                      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M3.5 8.5L8.5 3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                        <path d="M8.5 8.5V3.5H3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                      </svg>
                      <span id="{value_id}-change" class="financials-percent" style="margin-left: 4px;">+0.0%</span>
                    </div>
                  </div>
                  <div id="{value_id}-period" class="financials-period" style="display:none;"></div>
                </div>
              </div>
            </div>"""

def replace_card(html, start_id, end_id_or_comment, new_html):
    if end_id_or_comment.startswith("<!--"):
        pattern = re.compile(rf"(<div id=\"{start_id}\" class=\"quick-insights-card\">.*?(?={end_id_or_comment}))", re.DOTALL)
    else:
        pattern = re.compile(rf"(<div id=\"{start_id}\" class=\"quick-insights-card\">.*?(?=<div id=\"{end_id_or_comment}\" class=\"quick-insights-card\">))", re.DOTALL)
    
    # If the end is the end of the block, we can just match till specific ending tag if needed.
    # Actually, simpler logic:
    return re.sub(pattern, new_html + "\n            ", html)

# Card 1: Monthly income
html = replace_card(html, "key-financials-card-monthly-income-gradient", "key-financials-card-total-rental-gradient", generate_card("key-financials-card-monthly-income-gradient", "monthly-income", "Monthly income", "wallet"))

# Card 2: Total rental income
html = replace_card(html, "key-financials-card-total-rental-gradient", "key-financials-card-total-appreciation-gradient", generate_card("key-financials-card-total-rental-gradient", "total-rental", "Total rental income", "bldg"))

# Card 3: Total appreciation
html = replace_card(html, "key-financials-card-total-appreciation-gradient", "quick-insights-card-number-of-properties", generate_card("key-financials-card-total-appreciation-gradient", "total-appreciation", "Total appreciation", "lineUp"))

# Card 4: Number of properties
html = replace_card(html, "quick-insights-card-number-of-properties", "quick-insights-card-occupancy-rate", generate_card("quick-insights-card-number-of-properties", "insights-value-number-of-properties", "Number of properties", "home", show_currency=False))

# Card 5: Occupancy rate
html = replace_card(html, "quick-insights-card-occupancy-rate", "quick-insights-card-annual-rental-yield", generate_card("quick-insights-card-occupancy-rate", "insights-value-occupancy-rate", "Occupancy rate", "cal", show_currency=False))

# Card 6: Annual rental yield
# For the last one, we end at portfolio assets table
pattern6 = re.compile(r"(<div id=\"quick-insights-card-annual-rental-yield\" class=\"quick-insights-card\">.*?(?=</div>\s*</div>\s*</div>\s*<!-- Assets Table -->))", re.DOTALL)
new_html6 = generate_card("quick-insights-card-annual-rental-yield", "insights-value-annual-rental-yield", "Annual rental yield", "award", show_currency=False)
html = re.sub(pattern6, new_html6 + "\n              ", html)


with open("frontend/platform/portfolio.html", "w") as f:
    f.write(html)

print("done")
