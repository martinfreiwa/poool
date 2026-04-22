import re

with open('frontend/platform/developer/add-asset.html', 'r', encoding='utf-8') as f:
    content = f.read()

# standard headers with fonts.css included
# I will use 'fonts' which maps to fonts.css as verified in head.html
new_header = """{% with title="Developer Add Asset", meta_description="Add a new property or asset to POOOL.", extra_css=['fonts', 'bundle', 'developer-add-asset'], extra_js=['htmx-init', 'developer-add-asset', 'profile-dropdown', 'mobile-navigation', 'poool-dropdown', 'poool-dropdown-init'] %}{% include "components/head.html" %}{% endwith %}

<body id="developer-add-asset-body">
  {% include 'components/mobile-menu.html' %}
  <div id="developer-add-asset-page" class="marketplace-page">
    <div id="developer-add-asset-sidebar" class="marketplace-sidebar">
      {% include 'components/sidebar.html' %}
    </div>
"""

main_match = re.search(r'(<main id="developer-add-asset-main".*?</main>)', content, re.DOTALL)

if main_match:
    main_section = main_match.group(1)
    
    # Ensure button arrow stroke is neon green
    main_section = main_section.replace('stroke="#FFFFFF"', 'stroke="#98FB96"')
    # Add a fallback style inside the template just in case the external CSS is delayed
    # This ensures the font is forced immediately.
    force_font_style = """
<style>
  * { font-family: 'TT Norms Pro', sans-serif !important; }
  .add-asset-next-btn { background-color: #0000FF !important; }
  .next-btn-text, .next-btn-arrow path { color: #98FB96 !important; stroke: #98FB96 !important; }
</style>
"""
    
    footer = "\\n  </div>\\n</body>\\n</html>"
    
    final_output = new_header + force_font_style + main_section + footer
    final_output = final_output.replace("\\n", "\n")
    
    with open('frontend/platform/developer/add-asset.html', 'w', encoding='utf-8') as f:
        f.write(final_output)
    print("Fixed add-asset.html structure (v9) - forced font via inline style.")
else:
    print("Could not find Main section")
