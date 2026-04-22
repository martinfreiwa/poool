import re

# 1. Update HTML Table Header
with open("frontend/platform/portfolio.html", "r") as f:
    html = f.read()

new_header = """<div class="portfolio-assets-header-cell property-col">
                <div class="portfolio-assets-header-content">
                  <img  src="/static/images/home-05.svg" alt="Property" width="16" height="16" />
                  <span class="portfolio-assets-header-text">Property</span>
                </div>
              </div>
              <div class="portfolio-assets-header-cell investment-col">
                <div class="portfolio-assets-header-content">
                  <img  src="/static/images/line-chart-up-02.svg" alt="Investment" width="16" height="16" />
                  <span class="portfolio-assets-header-text">Investment value</span>
                </div>
              </div>
              <div class="portfolio-assets-header-cell appreciation-col">
                <div class="portfolio-assets-header-content">
                  <img  src="/static/images/chart-line-up.svg" alt="Growth" width="16" height="16" />
                  <span class="portfolio-assets-header-text">Growth</span>
                </div>
              </div>
              <div class="portfolio-assets-header-cell rental-col">
                <div class="portfolio-assets-header-content">
                  <img  src="/static/images/coins-stacked-03.svg" alt="Rental" width="16" height="16" />
                  <span class="portfolio-assets-header-text">Total rental income</span>
                </div>
              </div>"""

# we need to replace the header area carefully
# from <div class="portfolio-assets-header-cell property-col"> to the end of rental-col.
old_header_pattern = re.compile(r'<div class="portfolio-assets-header-cell property-col">.*?<div class="portfolio-assets-header-cell status-col">', re.DOTALL)
replacement = new_header + '\n              <div class="portfolio-assets-header-cell status-col">'
html = re.sub(old_header_pattern, replacement, html)

with open("frontend/platform/portfolio.html", "w") as f:
    f.write(html)
    
# 2. Update CSS
with open("frontend/platform/static/css/portfolio-assets-table.css", "r") as f:
    css = f.read()

if ".appreciation-col" not in css:
    css = css.replace(".investment-col {\n  width: 200px;\n  flex: none;\n}", """.investment-col {
  width: 160px;
  flex: none;
}

.appreciation-col {
  width: 140px;
  flex: none;
}""")
    css = css.replace(".rental-col {\n  width: 200px;", ".rental-col {\n  width: 160px;")
    with open("frontend/platform/static/css/portfolio-assets-table.css", "w") as f:
        f.write(css)

# 3. Update JS
with open("frontend/platform/static/js/portfolio-data.js", "r") as f:
    js = f.read()

# remove buildChainBadge logic completely
js = re.sub(r'const chainBadge = buildChainBadge\(inv\);', '', js)
js = re.sub(r'<br\/>\$\{chainBadge\}', '', js)

# Extract the ds-badge span
badge_span = """<div class="portfolio-assets-cell appreciation-col">
          <span class="ds-badge" style="background: #FFFFFF; color: #475467; border: 1px solid #E9EAEB; padding: 2px 6px; font-weight: 600; font-size: 11px; display:inline-flex; align-items:center; gap:2px; border-radius: 4px;">
            <svg width="8" height="8" viewBox="0 0 12 12" fill="none"><path d="M3.5 8.5L8.5 3.5M8.5 3.5H3.5M8.5 3.5V8.5" stroke="#475467" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
            ${escHtml(inv.appreciationDisplay)}
          </span>
        </div>"""

# Remove ds-badge from investment-col
old_inv_col = r'<div class="portfolio-assets-cell investment-col" style="display:flex; flex-direction:column; align-items:flex-start; gap:4px; padding-top:4px;">\s*<div style="font-weight: 700; color: #101828; font-size: 14px;">\$\{escHtml\(inv.currentValueDisplay\)\}</div>\s*<span class="ds-badge" style="background.*?</svg>\s*\$\{escHtml\(inv.appreciationDisplay\)\}\s*</span>\s*</div>'
new_inv_col = f"""<div class="portfolio-assets-cell investment-col">
          <div style="font-weight: 700; color: #101828; font-size: 14px;">${{escHtml(inv.currentValueDisplay)}}</div>
        </div>
        {badge_span}"""

js = re.sub(old_inv_col, new_inv_col, js, flags=re.DOTALL)

with open("frontend/platform/static/js/portfolio-data.js", "w") as f:
    f.write(js)

