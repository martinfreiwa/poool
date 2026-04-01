import re
import os

js_path = '/Users/martin/Projects/poool/frontend/platform/static/js/admin-sidebar-loader.js'
with open(js_path, 'r') as f:
    js_content = f.read()

# Extract exactly what is inside sidebarHtml = ` ... \n            `;
match = re.search(r'const sidebarHtml = `([\s\S]*?)`;', js_content)
if not match:
    print("Could not find sidebarHtml")
    exit(1)

html = match.group(1)

# Remove the template literal interpolation ${isPathActive(["..."]) ? "active" : ""}
html_clean = re.sub(r'\$\{isPathActive\(\[.*?\]\)\s*\?\s*"active"\s*:\s*""\}', '', html)

# Also remove any trailing spaces from the regex match
script = """
<script>
  document.addEventListener("DOMContentLoaded", function () {
    const currentPath = window.location.pathname;
    const cleanCurrent = currentPath.replace(/\/$/, "").replace(".html", "");
    
    // Find all links in the admin sidebar
    const links = document.querySelectorAll('#main-admin-sidebar .admin-nav-item');
    links.forEach(link => {
        let cleanTarget = link.getAttribute('href').replace(/\/$/, "").replace(".html", "");
        // Only mark active if it matches exactly, or is an explicit parent matches
        if (cleanCurrent === cleanTarget) {
            link.classList.add('active');
            link.setAttribute('aria-current', 'page');
        }
    });
  });
</script>
"""

final_html = html_clean + "\n" + script

# Save to destination
dest_path = '/Users/martin/Projects/poool/frontend/platform/admin/components/sidebar.html'
os.makedirs(os.path.dirname(dest_path), exist_ok=True)
with open(dest_path, 'w') as f:
    f.write(final_html)

print("Saved SSR admin sidebar template to", dest_path)
