"""Shared helpers for the /developer/* static template-render tests.

These helpers parse rendered HTML using only stdlib (html.parser) and expose
a small fixture surface that each `test_developer_<page>_static.py` module
imports. Kept underscore-prefixed so pytest's `test_*.py` collection glob
ignores it.

The helpers are intentionally tolerant: when the backend is unreachable or
when no DEV_SESSION_COOKIE is set, callers skip via pytest.skip(). They do
NOT assert anything by themselves — assertion logic stays in the per-page
test modules so failures point at a concrete page.
"""
from __future__ import annotations

import os
from dataclasses import dataclass, field
from html.parser import HTMLParser
from typing import Iterable

import pytest
import requests


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DEV_COOKIE = os.environ.get("DEV_SESSION_COOKIE", "")

# Files that must not be referenced after their deletion in earlier cleanups.
# Asserted on every developer page so a re-add anywhere is caught.
FORBIDDEN_DELETED_FILES: tuple[str, ...] = (
    "developer-submission-success.js",
    "affiliate-team-analytics.html",
    "dashboard.html.bak",
)

# Strings that should never appear in any rendered developer template. Note
# that "coming soon" is page-conditional — the add-asset page uses it for
# unsupported asset categories — so it is NOT in the global blacklist here.
GLOBAL_FORBIDDEN_TEXT: tuple[str, ...] = (
    "lorem ipsum",
    "todo",
    "fixme",
)


@dataclass
class ParsedPage:
    ids: set[str] = field(default_factory=set)
    classes: set[str] = field(default_factory=set)
    scripts: list[str] = field(default_factory=list)
    stylesheets: list[str] = field(default_factory=list)
    titles: list[str] = field(default_factory=list)
    meta_viewport: bool = False
    placeholder_hrefs: list[str] = field(default_factory=list)
    data_attrs: list[tuple[str, str, str | None]] = field(default_factory=list)
    inputs: list[dict[str, str | None]] = field(default_factory=list)

    @property
    def script_blob(self) -> str:
        return " ".join(self.scripts)

    @property
    def css_blob(self) -> str:
        return " ".join(self.stylesheets)


class _PageParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.page = ParsedPage()
        self._in_title = False

    def handle_starttag(self, tag, attrs):
        attr = {k: v for k, v in attrs}
        page = self.page

        if "id" in attr and attr["id"]:
            page.ids.add(attr["id"])
        if "class" in attr and attr["class"]:
            for cls in attr["class"].split():
                page.classes.add(cls)
        if tag == "script" and attr.get("src"):
            page.scripts.append(attr["src"])
        if tag == "link":
            rel = (attr.get("rel") or "").lower()
            if "stylesheet" in rel and attr.get("href"):
                page.stylesheets.append(attr["href"])
        if tag == "meta" and (attr.get("name") or "").lower() == "viewport":
            page.meta_viewport = True
        if tag == "title":
            self._in_title = True
        if tag == "a" and attr.get("href") == "#":
            role = (attr.get("role") or "").lower()
            has_data_attr = any(k.startswith("data-") for k in attr.keys())
            has_onclick = bool(attr.get("onclick"))
            # Bare "#" with no role/data/onclick is the placeholder anti-pattern.
            if role not in {"button", "tab", "menuitem", "menuitemcheckbox"} and not has_data_attr and not has_onclick:
                page.placeholder_hrefs.append(repr(attr))
        if tag == "input":
            page.inputs.append(attr)
        for k, v in attr.items():
            if k.startswith("data-"):
                page.data_attrs.append((tag, k, v))

    def handle_endtag(self, tag):
        if tag == "title":
            self._in_title = False

    def handle_data(self, data):
        if self._in_title and data.strip():
            self.page.titles.append(data.strip())


def fetch_page(route: str) -> requests.Response:
    """GET `route` from the running backend, or pytest.skip on missing prereqs."""
    if not DEV_COOKIE:
        pytest.skip("DEV_SESSION_COOKIE not set — skipping static template test")
    try:
        return requests.get(
            f"{BASE_URL}{route}",
            cookies={"session": DEV_COOKIE},
            timeout=5,
            allow_redirects=False,
        )
    except requests.RequestException as exc:
        pytest.skip(f"Backend not reachable at {BASE_URL}: {exc}")


def parse_page(response: requests.Response) -> ParsedPage:
    parser = _PageParser()
    parser.feed(response.text)
    return parser.page


def assert_required_ids(page: ParsedPage, required: Iterable[str]) -> None:
    missing = set(required) - page.ids
    assert not missing, f"Missing expected element IDs: {sorted(missing)}"


def assert_scripts_present(page: ParsedPage, hints: Iterable[str]) -> None:
    blob = page.script_blob
    missing = [h for h in hints if h not in blob]
    assert not missing, f"Missing expected scripts: {missing}\nLoaded scripts: {page.scripts}"


def assert_stylesheets_present(page: ParsedPage, hints: Iterable[str]) -> None:
    blob = page.css_blob
    missing = [h for h in hints if h not in blob]
    assert not missing, f"Missing expected stylesheets: {missing}\nLoaded css: {page.stylesheets}"


def assert_meta_viewport(page: ParsedPage) -> None:
    assert page.meta_viewport, "Missing <meta name=viewport ...>"


def assert_title_non_empty(page: ParsedPage) -> None:
    assert page.titles, "Page <title> is missing or empty"
    joined = " ".join(page.titles)
    assert joined.strip(), "Page <title> text is whitespace-only"


def assert_no_forbidden_global_text(response: requests.Response, extra: Iterable[str] = ()) -> None:
    body = response.text.lower()
    for needle in (*GLOBAL_FORBIDDEN_TEXT, *extra):
        assert needle.lower() not in body, (
            f"Forbidden text {needle!r} found in rendered page"
        )


def assert_no_deleted_file_refs(response: requests.Response) -> None:
    body = response.text
    for deleted in FORBIDDEN_DELETED_FILES:
        assert deleted not in body, (
            f"Reference to deleted file {deleted!r} found in rendered HTML"
        )


def assert_no_placeholder_anchors(page: ParsedPage) -> None:
    assert not page.placeholder_hrefs, (
        f"Found {len(page.placeholder_hrefs)} bare href=\"#\" placeholder anchors: "
        f"{page.placeholder_hrefs[:3]}…"
    )
