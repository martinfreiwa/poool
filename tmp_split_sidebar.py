import re

filepath = '/Users/martin/Projects/poool/frontend/platform/components/sidebar.html'
with open(filepath, 'r') as f:
    content = f.read()

# The original file has:
# <aside ... class="sidebar"...>  <-- asides[0] (Stale fallback)
# <template id="investor-sidebar-template"> <aside...> </aside> </template> <-- asides[1] (Good investor)
# <template id="developer-sidebar-template"> <aside...> </aside> </template> <-- asides[2] (Good developer)

asides = re.findall(r'<aside[\s\S]*?</aside>', content)

if len(asides) < 3:
    print(f"Error: Found only {len(asides)} asides! Expected 3.")
    exit(1)

investor_aside = asides[1]
developer_aside = asides[2]

# Also ensure developer_aside is not corrupted:
developer_aside = developer_aside.replace('id="nav-item-dashboard"', 'id="nav-item-dashboard" class="sidebar__nav-item"')
# It actually doesn't matter, we are just ripping exactly what the client-side templating ripped, which is perfect.

# The Javascript at the top to fix active states without document.write and without blocking DOM FOUC.
head_script = """<div id="dynamic-sidebar-container">
  <script>
    (function () {
      var profile = localStorage.getItem("selectedProfile");
      var path = window.location.pathname;
      var isDeveloperPage = path.indexOf("/developer/") === 0 || path === "/developer";
      var styles = "";

      // Hide investor sidebar immediately if on a developer page
      if (isDeveloperPage) {
        styles += '#sidebar-navigation[data-profile="investor"] { display: none !important; }\\n';
      }

      var activePage = "";
      if (path === "/" || (path.includes("/marketplace") && !path.includes("/marketplace-secondary") && !path.includes("/marketplace-trading"))) activePage = "marketplace-parent";
      else if (path.includes("/my-trading")) activePage = "my-trading";
      else if (path.includes("/marketplace-secondary") || path.includes("/marketplace-trading")) activePage = "trading-parent";
      else if (path.includes("/wallet") || path.includes("/transactions")) activePage = "wallet";
      else if (path.includes("/portfolio")) activePage = "portfolio";
      else if (path.includes("/cart") || path.includes("/checkout")) activePage = "cart";
      else if (path.includes("/affiliate")) activePage = "affiliate";
      else if (path.includes("/rewards")) activePage = "rewards";
      else if (path.includes("/community")) activePage = "community";
      else if (path.includes("/leaderboard")) activePage = "leaderboard";
      else if (path.includes("/developer/dashboard")) activePage = "dashboard";
      else if (path.includes("/developer/submissions")) activePage = "submissions";
      else if (path.includes("/developer/assets")) activePage = "assets";
      else if (path.includes("/settings")) activePage = "settings";
      else if (path.includes("/support")) activePage = "support";

      if (activePage) {
        styles += '#nav-item-' + activePage + ' .sidebar__nav-item-content { background: rgba(3, 255, 136, 0.15) !important; border-radius: 6px; }\\n';
        styles += '#nav-item-' + activePage + ' .sidebar__nav-text { color: var(--brand-dark-blue) !important; }\\n';
        styles += '#nav-item-' + activePage + ' .nav-icon { filter: brightness(0) saturate(100%) invert(11%) sepia(97%) saturate(7484%) hue-rotate(247deg) brightness(94%) contrast(148%) !important; }\\n';
      }

      if (styles) {
        var styleEl = document.createElement('style');
        styleEl.id = 'dynamic-sidebar-style';
        styleEl.textContent = styles;
        document.head.appendChild(styleEl);
      }
      
      // Accessibility update: set aria-current when DOM is ready
      document.addEventListener("DOMContentLoaded", function () {
        if (activePage) {
          var activeLink = document.getElementById("nav-item-" + activePage + "-content");
          if (!activeLink) {
              activeLink = document.querySelector("#nav-item-" + activePage + " a.sidebar__nav-item-content");
          }
          if (activeLink) activeLink.setAttribute("aria-current", "page");
        }
      });
    })();
  </script>

<style>
  .sidebar__add-asset-btn, 
  #sidebar-add-asset-btn,
  .sidebar [href*="add-asset"],
  .sidebar .btn-primary {
    background-color: #0000FF !important;
    color: #98FB96 !important;
  }
  .sidebar [href*="add-asset"] svg,
  .sidebar .btn-primary svg {
    stroke: #98FB96 !important;
  }
</style>
"""

# Now grab the script AFTER the templates
# We use re.findall script up to </script> at the bottom.
bottom_scripts = re.findall(r'<script>[\s\S]*?</script>', content)

# We want the script with "window.toggleProfileDropdown = function" and "window.toggleNavExpand = function"
# Let's filter the bottom scripts that contain these key functions
final_bottom_scripts = []
for s in bottom_scripts:
    if "window.toggleProfileDropdown" in s or "window.toggleNavExpand" in s or "document.addEventListener" in s:
        # Exclude the script that has "replaceSidebar", we don't need it because SSR handles it!
        if "replaceSidebar" not in s and "document.write" not in s:
            final_bottom_scripts.append(s)

assembled_bottom = "\n".join(final_bottom_scripts)

# Note: The is_developer flag injected by backend
new_content = head_script + """
  {% if is_developer %}
""" + developer_aside + """
  {% else %}
""" + investor_aside + """
  {% endif %}
""" + assembled_bottom + "\n</div>"

with open(filepath, 'w') as f:
    f.write(new_content)

print("sidebar.html correctly refactored using aside[1]!")
