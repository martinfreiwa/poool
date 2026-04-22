filepath = 'frontend/platform/components/sidebar.html'
with open(filepath, 'r') as f:
    content = f.read()

# The user wants "electric blue for the background and green for the inside"
# The Add Asset button is likely the one with class sidebar__add-asset or similar
# Based on the screenshot, it's the blue button on the left sidebar.

css_update = """
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

# Find where to inject
if "</script>" in content:
    content = content.replace("</script>", "</script>\n" + css_update, 1)
else:
    content = css_update + content

with open(filepath, 'w') as f:
    f.write(content)

print("Updated sidebar with explicit electric blue/green styling.")
