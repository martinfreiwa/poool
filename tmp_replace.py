import os
import re

dir_path = '/Users/martin/Projects/poool/frontend/platform/admin'

pattern1 = r'<!--\s*Sidebar Placeholder\s*-->\s*<div\s+id="admin-sidebar-placeholder">\s*</div>\s*<script\s+src="/static/js/admin-sidebar-loader\.js">\s*</script>'
pattern2 = r'<div\s+id="admin-sidebar-placeholder">\s*</div>\s*<script\s+src="/static/js/admin-sidebar-loader\.js">\s*</script>'

replacement = "{% include 'admin/components/sidebar.html' %}"

count = 0
for root, dirs, files in os.walk(dir_path):
    if 'components' in root:
        continue
    for file in files:
        if file.endswith('.html'):
            filepath = os.path.join(root, file)
            with open(filepath, 'r') as f:
                content = f.read()
            
            orig_content = content
            content = re.sub(pattern1, replacement, content)
            if content == orig_content:
                content = re.sub(pattern2, replacement, content)
                
            if content != orig_content:
                with open(filepath, 'w') as f:
                    f.write(content)
                count += 1
                print(f"Updated {filepath}")

print(f"Total files updated: {count}")
