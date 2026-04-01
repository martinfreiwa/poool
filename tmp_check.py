import os

docs_dir = "/Users/martin/Projects/poool/docs"
doc_files = [f for f in os.listdir(docs_dir) if f.endswith(".md") and f not in ["MASTERPLAN.md", "IMPLEMENTATION_ROADMAP.md"]]

with open(os.path.join(docs_dir, "MASTERPLAN.md"), "r", encoding="utf-8") as f:
    masterplan = f.read().lower()

with open(os.path.join(docs_dir, "IMPLEMENTATION_ROADMAP.md"), "r", encoding="utf-8") as f:
    roadmap = f.read().lower()

print("Missing from Masterplan:")
for f_name in doc_files:
    title_variant = f_name.replace(".md", "").lower().replace("_", " ")
    if title_variant not in masterplan:
        print(f" - {f_name}")

print("\nMissing from Roadmap:")
for f_name in doc_files:
    title_variant = f_name.replace(".md", "").lower().replace("_", " ")
    if title_variant not in roadmap:
        print(f" - {f_name}")
