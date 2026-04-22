import os

filepath = 'frontend/platform/static/css/fonts.css'
with open(filepath, 'r') as f:
    content = f.read()

# The current head.html uses /fonts/... but the service nests frontend/www/fonts
# Verified folder: ../frontend/www/fonts/TTNormsPro contains TT_Norms_Pro_Regular.woff2 etc.
# Verified entry in main.rs: .nest_service("/fonts", ServeDir::new("../frontend/www/fonts"))
# So url('/fonts/TTNormsPro/TT_Norms_Pro_Regular.woff2') should be correct.

# However, looking at the CSP in main.rs:
# font-src 'self' https://fonts.gstatic.com;
# It DOES allow 'self'.

# Let's check if the font-family naming in the CSS matches what is used in the app.
# The app uses 'TT Norms Pro'.

# Let's verify if there is any other fonts.css or if this one is correctly being loaded.
# I will also add a generic sans-serif fallback just in case.

force_css = """
/* Force application-wide font */
html, body, button, input, select, textarea {
    font-family: 'TT Norms Pro', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif !important;
}
"""

if force_css not in content:
    with open(filepath, 'a') as f:
        f.write(force_css)

print("Updated fonts.css with global override.")
