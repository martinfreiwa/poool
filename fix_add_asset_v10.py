import re

with open('frontend/platform/developer/add-asset.html', 'r', encoding='utf-8') as f:
    content = f.read()

# I will update the inline style to be even more aggressive and ensure the Google Font is NOT being used if it's overriding.
# The screenshot shows a very "standard" serif or generic sans-serif.

# I will also check if head.html is actually being included correctly or if there's a typo.
# extra_css=['fonts', 'bundle', 'developer-add-asset']

# Let's rewrite the force style
force_font_style = """
<style>
  /* Extremely aggressive font override */
  @font-face {
    font-family: 'TT Norms Pro Fake';
    src: url('/fonts/TTNormsPro/TT_Norms_Pro_Regular.woff2') format('woff2');
  }
  
  :root {
    --font-family: 'TT Norms Pro', sans-serif !important;
  }

  html, body, div, span, h1, h2, h3, h4, h5, h6, p, a, button, input {
    font-family: 'TT Norms Pro', 'TT Norms Pro Fake', sans-serif !important;
  }
  
  .add-asset-title {
     font-family: 'TT Norms Pro', sans-serif !important;
     font-weight: 700 !important;
  }
  
  .add-asset-next-btn { 
    background-color: #0000FF !important; 
    border: none !important;
  }
  
  .next-btn-text { 
    color: #98FB96 !important; 
    font-family: 'TT Norms Pro', sans-serif !important;
    font-weight: 600 !important;
  }
</style>
"""

# Replace the previous style block or prepend if not found
if "<style>" in content and "Extremely aggressive font override" not in content:
    content = re.sub(r'<style>.*?</style>', force_font_style, content, flags=re.DOTALL)
else:
    # Just prepend to main
    content = content.replace('<main', force_font_style + '<main')

with open('frontend/platform/developer/add-asset.html', 'w', encoding='utf-8') as f:
    f.write(content)

print("Injected ultra-aggressive font fix into add-asset.html")
