#!/usr/bin/env python3
"""Audit and render the POOOL page review tracker.

Source of truth:
    docs/page-review-tracker.yml

The audit intentionally stays repo-native and dependency-light. PyYAML is used
because the tracker is YAML; no app code, database, or web server is required.
"""

from __future__ import annotations

import argparse
import datetime as dt
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:  # pragma: no cover - local developer environment guard
    print("PyYAML is required: python3 -m pip install PyYAML", file=sys.stderr)
    sys.exit(2)


REPO_ROOT = Path(__file__).resolve().parents[1]
TRACKER_PATH = REPO_ROOT / "docs" / "page-review-tracker.yml"
REPORT_PATH = REPO_ROOT / "docs" / "PAGE_REVIEW_TRACKER.md"
FRONTEND_ROOT = REPO_ROOT / "frontend" / "platform"
BACKEND_ROOT = REPO_ROOT / "backend" / "src"

REVIEW_CATEGORIES = (
    "security_review",
    "accessibility_review",
    "e2e_review",
    "functional_review",
)

FRESHNESS_DAYS = {
    "critical": 30,
    "high": 60,
    "medium": 90,
    "low": 180,
}

ROUTE_PREFIX_BY_FILE = {
    "backend/src/auth/routes.rs": "/auth",
}

NON_PAGE_PREFIXES = (
    "/api/",
    "/ws/",
    "/static/",
    "/images/",
    "/uploads/",
    "/fonts/",
    "/en/",
    "/id/",
    "/png/",
    "/svg/",
    "/webp/",
    "/webm/",
)

NON_PAGE_PATHS = {
    "/health",
    "/blog/feed.xml",
    "/robots.txt",
    "/sitemap.xml",
}


def repo_rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def parse_date(value: Any) -> dt.date | None:
    if value in (None, ""):
        return None
    if isinstance(value, dt.date):
        return value
    try:
        return dt.date.fromisoformat(str(value))
    except ValueError:
        return None


def load_tracker(path: Path = TRACKER_PATH) -> dict[str, Any]:
    if not path.exists():
        raise FileNotFoundError(path)
    with path.open("r", encoding="utf-8") as handle:
        data = yaml.safe_load(handle) or {}
    if not isinstance(data, dict):
        raise ValueError(f"{path} must contain a YAML mapping")
    return data


def discover_html_templates() -> tuple[set[str], set[str]]:
    """Return (page_templates, supporting_templates)."""
    page_templates: set[str] = set()
    supporting_templates: set[str] = set()

    for path in sorted(FRONTEND_ROOT.rglob("*.html")):
        rel_platform = path.relative_to(FRONTEND_ROOT).as_posix()
        if rel_platform.startswith("_archive/"):
            continue
        rel_repo = repo_rel(path)
        if rel_platform.startswith(
            (
                "components/",
                "partials/",
                "templates/",
                "admin/components/",
                "admin/templates/",
            )
        ):
            supporting_templates.add(rel_repo)
        else:
            page_templates.add(rel_repo)

    return page_templates, supporting_templates


def discover_page_routes() -> dict[str, dict[str, str]]:
    """Discover non-API GET routes that look like pages or page fragments."""
    routes: dict[str, dict[str, str]] = {}
    route_re = re.compile(r"\.route\(\s*\"([^\"]+)\"\s*,\s*(.*?)\)", re.S)

    for path in sorted(BACKEND_ROOT.rglob("*.rs")):
        rel = repo_rel(path)
        text = path.read_text(encoding="utf-8")
        prefix = ROUTE_PREFIX_BY_FILE.get(rel, "")

        for match in route_re.finditer(text):
            raw_route = match.group(1)
            route = f"{prefix}{raw_route}" if prefix and raw_route != "/" else raw_route
            route = re.sub(r"//+", "/", route)
            call = " ".join(match.group(2).split())

            handler_match = re.search(r"\bget\(([^)\s]+)", call)
            if not handler_match:
                continue
            handler = handler_match.group(1).rstrip(",")

            if route in NON_PAGE_PATHS or route.startswith(NON_PAGE_PREFIXES):
                continue
            if handler.startswith("api_") or "::api_" in handler:
                continue
            if handler.startswith("get_") or "::get_" in handler:
                continue
            if raw_route in {"/tickets"} and "support/mod.rs" in rel:
                continue

            line = text.count("\n", 0, match.start()) + 1
            routes[route] = {
                "file": rel,
                "handler": handler,
                "line": str(line),
            }

    # route_service("/", landing-v2) and host-dispatch root behavior are
    # represented by explicit tracker entries; add the root if regex misses it.
    routes.setdefault(
        "/",
        {
            "file": "backend/src/main.rs",
            "handler": "handle_root / www ServeFile landing-v2.html",
            "line": "",
        },
    )
    return routes


def tracker_route_index(pages: list[dict[str, Any]]) -> dict[str, str]:
    index: dict[str, str] = {}
    for page in pages:
        page_id = str(page.get("id", ""))
        for route in [page.get("url_path"), *(page.get("route_aliases") or [])]:
            if route:
                index[str(route)] = page_id
    return index


def page_template_index(pages: list[dict[str, Any]]) -> dict[str, str]:
    index: dict[str, str] = {}
    for page in pages:
        page_id = str(page.get("id", ""))
        template = page.get("source_template")
        if template:
            index[str(template)] = page_id
    return index


def supporting_template_index(templates: list[dict[str, Any]]) -> dict[str, str]:
    index: dict[str, str] = {}
    for template in templates:
        template_id = str(template.get("id", ""))
        path = template.get("path")
        if path:
            index[str(path)] = template_id
    return index


def validate_file_refs(tracker: dict[str, Any]) -> list[str]:
    missing: list[str] = []
    for page in tracker.get("pages", []) or []:
        page_id = page.get("id", "<missing id>")
        refs = []
        if page.get("source_template"):
            refs.append(page["source_template"])
        refs.extend(page.get("related_js") or [])
        refs.extend(page.get("related_css") or [])
        backend_route = page.get("backend_route") or {}
        if backend_route.get("file"):
            refs.append(backend_route["file"])

        for ref in refs:
            if not (REPO_ROOT / ref).exists():
                missing.append(f"{page_id}: {ref}")

    for template in tracker.get("supporting_templates", []) or []:
        template_id = template.get("id", "<missing id>")
        path = template.get("path")
        if path and not (REPO_ROOT / path).exists():
            missing.append(f"{template_id}: {path}")

    return missing


def review_state(page: dict[str, Any], today: dt.date) -> tuple[list[str], list[str]]:
    stale: list[str] = []
    missing: list[str] = []
    risk = str(page.get("business_risk", "medium"))
    max_age = FRESHNESS_DAYS.get(risk, 90)

    reviews = page.get("reviews") or {}
    for category in REVIEW_CATEGORIES:
        review = reviews.get(category) or {}
        status = str(review.get("status", "not reviewed"))
        if status == "not reviewed":
            missing.append(category)
            continue
        if status == "not applicable":
            continue

        due = parse_date(review.get("next_review_due_date"))
        last = parse_date(review.get("last_review_date"))
        if due and due < today:
            stale.append(category)
        elif last and (today - last).days > max_age:
            stale.append(category)
        elif status == "stale":
            stale.append(category)

    return stale, missing


def audit(tracker: dict[str, Any]) -> dict[str, Any]:
    pages = tracker.get("pages", []) or []
    supporting_templates = tracker.get("supporting_templates", []) or []
    discovered_routes = discover_page_routes()
    discovered_pages, discovered_supporting = discover_html_templates()

    route_index = tracker_route_index(pages)
    template_index = page_template_index(pages)
    support_index = supporting_template_index(supporting_templates)

    missing_routes = sorted(route for route in discovered_routes if route not in route_index)
    missing_page_templates = sorted(path for path in discovered_pages if path not in template_index)
    missing_supporting_templates = sorted(
        path for path in discovered_supporting if path not in support_index
    )
    missing_files = validate_file_refs(tracker)

    today = dt.date.today()
    stale_reviews: dict[str, list[str]] = {}
    missing_reviews: dict[str, list[str]] = {}
    for page in pages:
        page_id = str(page.get("id", ""))
        stale, missing = review_state(page, today)
        if stale:
            stale_reviews[page_id] = stale
        if missing:
            missing_reviews[page_id] = missing

    return {
        "discovered_routes": discovered_routes,
        "discovered_page_templates": discovered_pages,
        "discovered_supporting_templates": discovered_supporting,
        "missing_routes": missing_routes,
        "missing_page_templates": missing_page_templates,
        "missing_supporting_templates": missing_supporting_templates,
        "missing_files": missing_files,
        "stale_reviews": stale_reviews,
        "missing_reviews": missing_reviews,
        "pages_with_test_date": sum(1 for page in pages if page.get("last_tested_date")),
    }


def markdown_table(headers: list[str], rows: list[list[Any]]) -> str:
    out = ["| " + " | ".join(headers) + " |"]
    out.append("| " + " | ".join("---" for _ in headers) + " |")
    for row in rows:
        out.append("| " + " | ".join(str(cell).replace("\n", "<br>") for cell in row) + " |")
    return "\n".join(out)


def render_markdown(tracker: dict[str, Any], results: dict[str, Any]) -> str:
    pages = tracker.get("pages", []) or []
    supporting_templates = tracker.get("supporting_templates", []) or []
    today = dt.date.today().isoformat()

    risk_counts = Counter(str(p.get("business_risk", "unknown")) for p in pages)
    access_counts = Counter(str(p.get("access_level", "unknown")) for p in pages)
    open_issues = []
    for page in pages:
        for issue in page.get("issues") or []:
            if issue.get("status") in {"open", "in progress", "needs recheck"}:
                open_issues.append((page.get("id"), issue))

    rows = []
    for page in pages:
        reviews = page.get("reviews") or {}
        review_summary = ", ".join(
            f"{category.replace('_review', '')}: {(reviews.get(category) or {}).get('status', 'not reviewed')}"
            for category in REVIEW_CATEGORIES
        )
        stale = ", ".join(results["stale_reviews"].get(page.get("id"), []))
        rows.append(
            [
                page.get("id", ""),
                page.get("name", ""),
                page.get("url_path", ""),
                page.get("last_tested_date") or "-",
                page.get("access_level", ""),
                page.get("data_sensitivity", ""),
                page.get("business_risk", ""),
                review_summary,
                stale or "-",
                len(page.get("issues") or []),
            ]
        )

    ambiguous_rows = [
        [
            item.get("id", ""),
            item.get("kind", ""),
            item.get("path_or_route", ""),
            item.get("reason", ""),
            item.get("recommended_follow_up", ""),
        ]
        for item in tracker.get("ambiguous_entries", []) or []
    ]

    supporting_rows = [
        [
            item.get("id", ""),
            item.get("template_type", ""),
            item.get("path", ""),
            item.get("notes", ""),
        ]
        for item in supporting_templates
    ]

    issue_rows = [
        [
            page_id,
            issue.get("issue_id", ""),
            issue.get("review_category", ""),
            issue.get("severity", ""),
            issue.get("status", ""),
            issue.get("title", ""),
            issue.get("owner", ""),
        ]
        for page_id, issue in open_issues
    ]

    audit_rows = [
        ["Discovered page routes", len(results["discovered_routes"])],
        ["Discovered page templates", len(results["discovered_page_templates"])],
        ["Discovered supporting templates", len(results["discovered_supporting_templates"])],
        ["Missing tracker routes", len(results["missing_routes"])],
        ["Missing page template entries", len(results["missing_page_templates"])],
        ["Missing supporting template entries", len(results["missing_supporting_templates"])],
        ["Tracker references to missing files", len(results["missing_files"])],
        ["Pages with stale reviews", len(results["stale_reviews"])],
        ["Pages with not-reviewed categories", len(results["missing_reviews"])],
        ["Pages with last_tested_date set", results["pages_with_test_date"]],
    ]

    checklist_sections = []
    for name, checklist in (tracker.get("review_checklists") or {}).items():
        title = name.replace("_", " ").title()
        checklist_sections.append(f"### {title}\n" + "\n".join(f"- [ ] {item}" for item in checklist))

    freshness_lines = []
    policy = tracker.get("freshness_policy") or {}
    for risk in ("critical", "high", "medium", "low"):
        freshness_lines.append(f"- **{risk}**: {policy.get(risk, FRESHNESS_DAYS[risk])}")
    freshness_lines.append(
        f"- **Pull requests**: {policy.get('pull_request_rule', 'Any page touched in a PR should be checked against this tracker before release.')}"
    )

    status_legend = tracker.get("status_legend") or []
    severity_legend = tracker.get("severity_legend") or []
    capability_rows = [
        [
            capability.get("name", ""),
            capability.get("status", ""),
            capability.get("purpose", ""),
        ]
        for capability in tracker.get("recommended_review_capabilities", []) or []
    ]

    return "\n\n".join(
        [
            "# Page Review Tracker",
            (
                "## Purpose\n"
                "This document is the human-readable view of `docs/page-review-tracker.yml`. "
                "It tracks every known page route, frontend page template, and supporting HTML template "
                "that should be considered during security, accessibility, E2E, and functional reviews."
            ),
            (
                "## How To Update\n"
                "- Update `docs/page-review-tracker.yml`; do not hand-edit generated tables in this report.\n"
                "- Add a new page entry whenever a new Axum page route or `frontend/platform` page template is added.\n"
                "- Set `url_path`, `source_template`, `backend_route`, `related_js`, `related_css`, access level, sensitivity, risk, `last_tested_date`, owner, and notes.\n"
                "- Record a review by updating the relevant review category with status, reviewer, date, commit SHA, due date, evidence, and findings summary.\n"
                "- Record an issue under the affected page's `issues` list with the required issue fields.\n"
                "- Regenerate this report with `python3 scripts/audit_page_review_tracker.py --write-md`.\n"
                "- Audit without writing with `python3 scripts/audit_page_review_tracker.py`."
            ),
            "## Status Legend\n" + "\n".join(f"- `{item}`" for item in status_legend),
            "## Severity Legend\n" + "\n".join(f"- `{item}`" for item in severity_legend),
            "## Freshness Policy\n" + "\n".join(freshness_lines),
            "## Recommended Review Capabilities\n"
            + (
                markdown_table(["Capability", "Status", "Purpose"], capability_rows)
                if capability_rows
                else "No recommended capabilities recorded."
            ),
            "## Review Checklists\n" + "\n\n".join(checklist_sections),
            f"## Audit Snapshot\nGenerated: {today}\n\n" + markdown_table(["Metric", "Count"], audit_rows),
            "## Coverage Summary\n"
            + markdown_table(
                ["Dimension", "Counts"],
                [
                    ["Business risk", ", ".join(f"{k}: {v}" for k, v in sorted(risk_counts.items()))],
                    ["Access level", ", ".join(f"{k}: {v}" for k, v in sorted(access_counts.items()))],
                    ["Open issues", len(open_issues)],
                ],
            ),
            "## Page Inventory\n"
            + markdown_table(
                [
                    "ID",
                    "Name",
                    "URL",
                    "Last Tested",
                    "Access",
                    "Sensitivity",
                    "Risk",
                    "Review Statuses",
                    "Stale",
                    "Issues",
                ],
                rows,
            ),
            "## Open Issues\n"
            + (
                markdown_table(
                    ["Page", "Issue ID", "Category", "Severity", "Status", "Title", "Owner"],
                    issue_rows,
                )
                if issue_rows
                else "No open issues recorded."
            ),
            "## Ambiguous Or Needs Verification\n"
            + (
                markdown_table(["ID", "Kind", "Path/Route", "Reason", "Follow-up"], ambiguous_rows)
                if ambiguous_rows
                else "No ambiguous entries recorded."
            ),
            "## Supporting Templates\n"
            + markdown_table(["ID", "Type", "Path", "Notes"], supporting_rows),
        ]
    ) + "\n"


def print_results(results: dict[str, Any]) -> None:
    print("Page review tracker audit")
    print(f"- discovered page routes: {len(results['discovered_routes'])}")
    print(f"- discovered page templates: {len(results['discovered_page_templates'])}")
    print(f"- discovered supporting templates: {len(results['discovered_supporting_templates'])}")
    print(f"- missing tracker routes: {len(results['missing_routes'])}")
    print(f"- missing page template entries: {len(results['missing_page_templates'])}")
    print(f"- missing supporting template entries: {len(results['missing_supporting_templates'])}")
    print(f"- tracker references to missing files: {len(results['missing_files'])}")
    print(f"- pages with stale review categories: {len(results['stale_reviews'])}")
    print(f"- pages with not-reviewed categories: {len(results['missing_reviews'])}")
    print(f"- pages with last_tested_date set: {results['pages_with_test_date']}")

    detail_groups = [
        ("Missing routes", results["missing_routes"]),
        ("Missing page templates", results["missing_page_templates"]),
        ("Missing supporting templates", results["missing_supporting_templates"]),
        ("Missing file references", results["missing_files"]),
    ]
    for title, values in detail_groups:
        if values:
            print(f"\n{title}:")
            for value in values[:100]:
                print(f"  - {value}")
            if len(values) > 100:
                print(f"  ... and {len(values) - 100} more")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tracker", type=Path, default=TRACKER_PATH)
    parser.add_argument("--write-md", action="store_true", help="Regenerate docs/PAGE_REVIEW_TRACKER.md")
    parser.add_argument("--report", type=Path, default=REPORT_PATH)
    args = parser.parse_args()

    tracker = load_tracker(args.tracker)
    results = audit(tracker)

    if args.write_md:
        markdown = render_markdown(tracker, results)
        args.report.write_text(markdown, encoding="utf-8")
        print(f"Wrote {repo_rel(args.report)}")

    print_results(results)

    required_missing = (
        results["missing_routes"]
        or results["missing_page_templates"]
        or results["missing_supporting_templates"]
        or results["missing_files"]
    )
    return 1 if required_missing else 0


if __name__ == "__main__":
    raise SystemExit(main())
