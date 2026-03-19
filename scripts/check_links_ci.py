#!/usr/bin/env python3
"""
check_links_ci.py - Checks for obviously broken internal HTML page links.
Skips static asset checks (images etc.) as those are server-side/GCS assets.
Used in CI to catch broken hrefs that reference non-existent HTML pages.
"""

import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
FRONTEND_DIR = ROOT / "frontend" / "platform"


def get_html_files():
    return list(FRONTEND_DIR.glob("*.html"))


def check_links():
    html_files = get_html_files()
    html_names = {f.name for f in html_files}

    errors = []

    for html_file in html_files:
        content = html_file.read_text(encoding="utf-8", errors="replace")

        # Check href links that point to .html files (internal page navigation only)
        hrefs = re.findall(r'href=["\']([^"\'#?]+\.html)["\']', content)
        for href in hrefs:
            # Skip template variables (MiniJinja / Jinja2 syntax)
            if "{{" in href or "{%" in href:
                continue
            # Strip leading / and get the basename
            target = href.lstrip("/")
            basename = Path(target).name
            if basename not in html_names:
                errors.append(f"{html_file.name}: broken href → '{href}'")

    if errors:
        print(f"\n❌ Found {len(errors)} broken HTML link(s):\n")
        for e in errors:
            print(f"  - {e}")
        sys.exit(1)
    else:
        print(f"✅ All internal HTML links OK ({len(html_files)} HTML files checked)")


if __name__ == "__main__":
    check_links()
