import re

# 1. Update fonts.css with a fallback that is known to work and check paths
filepath = 'frontend/platform/static/css/fonts.css'
with open(filepath, 'r') as f:
    content = f.read()

# Make sure the font-family is exactly 'TT Norms Pro'
# And that we have a global reset in fonts.css
global_reset = """
:root {
    --primary-font: 'TT Norms Pro', sans-serif;
}

* {
    font-family: 'TT Norms Pro', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif !important;
}
"""
if "global_reset" not in content:
    content = global_reset + content

with open(filepath, 'w') as f:
    f.write(content)

# 2. Update head.html to ensure bundle.css (which includes fonts.css) is loaded properly
# Actually head.html already loads bundle.css and fonts.css if passed in extra_css.

# 3. Update add-asset.html one last time - remove the fake font and use a more standard approach
# but keep it aggressive.
asset_path = 'frontend/platform/developer/add-asset.html'
with open(asset_path, 'r') as f:
    fc = f.read()

style_block = """
<style>
  @font-face {
    font-family: 'TT Norms Pro';
    src: url('/fonts/TTNormsPro/TT_Norms_Pro_Regular.woff2') format('woff2');
    font-weight: 400;
  }
  @font-face {
    font-family: 'TT Norms Pro';
    src: url('/fonts/TTNormsPro/TT_Norms_Pro_Bold.woff2') format('woff2');
    font-weight: 700;
  }
  
  body, main, h1, h2, h3, p, span, button {
    font-family: 'TT Norms Pro', sans-serif !important;
  }
  
  .add-asset-title {
    font-weight: 700 !important;
    font-size: 32px !important;
    color: #101828 !important;
  }
</style>
"""

# Re-inject the style block
fc = re.sub(r'<style>.*?</style>', style_block, fc, flags=re.DOTALL)

with open(asset_path, 'w') as f:
    f.write(fc)

print("Final font synchronization completed.")
