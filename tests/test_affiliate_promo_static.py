from pathlib import Path
import re


REPO_ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def backend_affiliate_tiers() -> list[tuple[int, str, int]]:
    backend = read("backend/src/rewards/service.rs")
    match = re.search(r"const AFFILIATE_TIERS:.*?=\s*&\[(.*?)\];", backend, re.S)
    assert match, "AFFILIATE_TIERS constant not found"

    tiers: list[tuple[int, str, int]] = []
    for threshold, name, bps in re.findall(r'\((\d+),\s*"([^"]+)",\s*(\d+)\)', match.group(1)):
        tiers.append((int(threshold), name, int(bps)))
    assert tiers
    return tiers


def test_affiliate_promo_tiers_match_backend_contract():
    html = read("frontend/platform/affiliate-promo.html")
    tiers = backend_affiliate_tiers()

    for threshold, name, bps in tiers:
        assert f'data-tier="{name}"' in html
        assert f"name: '{name}', minQualifiedReferrals: {threshold}, bps: {bps}" in html
        assert f"<td>{name}</td>" in html
        assert f"<td><span class=\"tier-rate\">{bps // 100}.{bps % 100:02d}%</span></td>" in html

    assert "4.50%" not in html
    assert "Sovereign" not in html
    assert "trailing 12-mo volume" not in html


def test_affiliate_promo_calculator_uses_integer_bps_and_accessible_controls():
    html = read("frontend/platform/affiliate-promo.html")

    assert "rate: 0." not in html
    assert "monthlyEarningsCents" in html
    assert "formatBps(tier.bps)" in html
    assert 'href="#who-we-want"' in html
    assert 'id="who-we-want"' in html
    assert 'for="calc-investment"' in html
    assert 'for="calc-referrals"' in html
    assert 'aria-describedby="calc-investment-val"' in html
    assert 'aria-describedby="calc-referrals-val"' in html
    assert 'aria-hidden="true" focusable="false"' in html
    assert "Applications are typically reviewed within 1-3 business days" not in html
    assert "48 hours" not in html
