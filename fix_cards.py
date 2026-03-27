import re

with open("frontend/platform/portfolio.html", "r") as f:
    html = f.read()

def replace_card(html, start_comment, next_comment, base_id, title, icon_file, icon_bg_class):
    pattern = re.compile(rf"({start_comment}).*?(?={next_comment})", re.DOTALL)
    
    replacement = f"""{start_comment}
            <div id="key-financials-card-{base_id}-gradient" class="quick-insights-card">
              <div class="insights-content">
                <div class="insights-icon-wrapper">
                  <div class="glass-icon-container">
                    <div class="glass-icon-bg {icon_bg_class}"></div>
                    <div class="glass-icon-front">
                      <img src="/static/images/Portfolio/{icon_file}" alt="{title}" width="24" height="24" />
                    </div>
                  </div>
                </div>
                <div class="insights-header">
                  <span class="insights-title">{title}</span>
                </div>
                <div style="display: flex; flex-direction: column; gap: 8px;">
                  <div class="insights-value">
                    <span class="financials-currency" style="font-size: 24px;">$</span>
                    <span id="portfolio-{base_id}" class="financials-amount" style="font-size: 24px;">...</span>
                  </div>
                  <div style="display: flex; align-items: center;">
                    <div class="financials-change change-increase" style="position:static; height:24px; display:inline-flex; padding: 2px 8px; border-radius: 6px; border: 1px solid #e5e7eb;">
                      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M3.5 8.5L8.5 3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                        <path d="M8.5 8.5V3.5H3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                      </svg>
                      <span id="portfolio-{base_id}-change" class="financials-percent" style="margin-left: 4px;">...</span>
                    </div>
                  </div>
                  <div id="portfolio-{base_id}-period" class="financials-period" style="display:none;"></div>
                </div>
              </div>
            </div>
            """
    return re.sub(pattern, replacement, html)

# 1
html = replace_card(html, "<!-- Monthly Income Card -->", "<!-- Total Rental Income Card -->", "monthly-income", "Monthly income", "coins-stacked-02.svg", "glass-purple-bg")

# 2
html = replace_card(html, "<!-- Total Rental Income Card -->", "<!-- Total Appreciation Card -->", "total-rental", "Total rental income", "sale-03.svg", "glass-green-bg")

# 3
# Note: Since the ID in JS is total-appreciation, we map base_id='total-appreciation'
html = replace_card(html, "<!-- Total Appreciation Card -->", "<div id=\"quick-insights-card-number-of-properties\"", "total-appreciation", "Total appreciation", "line-chart-up-02.svg", "glass-blue-bg")

with open("frontend/platform/portfolio.html", "w") as f:
    f.write(html)

print("done")
