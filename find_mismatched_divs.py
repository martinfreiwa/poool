import re

with open('frontend/platform/portfolio.html', 'r', encoding='utf-8') as f:
    text = f.read()

# Let's count open/close tags
divs = []
for m in re.finditer(r'<(div)[^>]*>|</(div)>', text):
    tag = m.group(0)
    start = m.start()
    if not tag.startswith('</'):
        # extract id or class
        id_m = re.search(r'id=["\']([^"\']+)["\']', tag)
        cls_m = re.search(r'class=["\']([^"\']+)["\']', tag)
        label = (id_m.group(1) if id_m else "") + " | " + (cls_m.group(1) if cls_m else "")
        divs.append({"type": "open", "label": label, "pos": start})
    else:
        if divs and divs[-1]['type'] == 'open':
            popped = divs.pop()
        else:
            divs.append({"type": "close", "label": "</div>", "pos": start})

if divs:
    print("Unmatched divs remaining:")
    for d in divs:
        print(d)
else:
    print("All divs matched correctly!")
