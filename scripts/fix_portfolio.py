import re

with open("frontend/platform/portfolio.html", "r") as f:
    html = f.read()

# Replace Chunk 1
chunk1 = """            </div>
          </div>
        </div>
        <!-- Quick Insights and Investment Limit -->
        <div id="insights-limit-section" class="insights-limit-section hidden">
          <div id="quick-insights-wrapper" class="quick-insights-wrapper">
            <h2 id="quick-insights-title" class="section-title">
              Quick insights
            </h2>
            <div id="quick-insights-grid" class="quick-insights-grid">"""

html = html.replace(chunk1, "            </div>")

# Replace Chunk 2
# Start at id="quick-insights-card-annual-rental-yield" closing divs
# and end right before <!-- Assets Table -->
chunk2_pattern = r"            </div>\n          </div>\n          <div id=\"investment-limit-wrapper\".*?</div>\n        </div>\n\n        <!-- Assets Table -->"
html = re.sub(chunk2_pattern, "          </div>\n        </div>\n\n        <!-- Assets Table -->", html, flags=re.DOTALL)

with open("frontend/platform/portfolio.html", "w") as f:
    f.write(html)

print("done")
