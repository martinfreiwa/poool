#!/usr/bin/env python3
"""
Optional live/staging read-only smoke checks.

Runs only when LIVE_READ_ONLY_BASE_URL is set. The checks use unauthenticated
GET requests only: public pages must render real content, protected pages must
redirect to login or deny access, and no form/API mutation is attempted.
"""

import os
from urllib.parse import urljoin

import pytest
import requests


BASE_URL = os.environ.get("LIVE_READ_ONLY_BASE_URL", "").rstrip("/")

PUBLIC_PAGES = [
    "/",
    "/blog",
    "/auth/login",
    "/terms",
    "/legal/terms",
    "/privacy-policy",
    "/privacy",
    "/legal/privacy",
    "/imprint",
    "/legal/imprint",
    "/aml-kyc-policy",
    "/legal/aml-kyc-policy",
    "/cookies",
    "/legal/cookies",
    "/feedback",
    "/feedback/rate",
    "/feedback/submit",
    "/referrals",
    "/glossary",
    "/changelog",
]

PROTECTED_PAGES = [
    "/marketplace",
    "/wallet",
    "/portfolio",
    "/transactions",
    "/leaderboard",
    "/settings",
    "/support",
    "/community",
    "/developer/dashboard",
    "/developer/assets",
    "/developer/submissions",
    "/developer/operations",
    "/developer/affiliate-team",
    "/admin/",
    "/admin/users",
    "/admin/kyc",
    "/admin/orders",
    "/admin/deposits",
    "/admin/marketplace/",
    "/admin/community/",
    "/admin/audit-logs",
]

LEGAL_PAGES = {
    "/terms",
    "/legal/terms",
    "/privacy-policy",
    "/privacy",
    "/legal/privacy",
    "/imprint",
    "/legal/imprint",
    "/aml-kyc-policy",
    "/legal/aml-kyc-policy",
    "/cookies",
    "/legal/cookies",
}


pytestmark = pytest.mark.skipif(
    not BASE_URL,
    reason="set LIVE_READ_ONLY_BASE_URL to run live/staging read-only smoke",
)


def fetch(path: str) -> requests.Response:
    return requests.get(
        urljoin(f"{BASE_URL}/", path.lstrip("/")),
        allow_redirects=True,
        timeout=20,
        headers={"User-Agent": "POOOL workflow read-only smoke"},
    )


@pytest.mark.parametrize("path", PUBLIC_PAGES)
def test_public_read_only_pages_render_without_404_template(path):
    response = fetch(path)

    assert response.status_code < 500, (
        f"{path} returned {response.status_code} at {response.url}"
    )
    assert "<html" in response.text.lower(), f"{path} did not return HTML"

    if path in LEGAL_PAGES:
        assert "404 page not found" not in response.text.lower(), (
            f"{path} rendered the 404 template at {response.url}"
        )


@pytest.mark.parametrize("path", PROTECTED_PAGES)
def test_protected_pages_do_not_leak_without_session(path):
    response = fetch(path)
    final_url = response.url.rstrip("/")
    body = response.text.lower()

    denied = response.status_code in {401, 403, 404}
    redirected_to_login = final_url.endswith("/auth/login") or "login" in body

    assert denied or redirected_to_login, (
        f"{path} was not visibly protected; status={response.status_code}, "
        f"final_url={response.url}"
    )
