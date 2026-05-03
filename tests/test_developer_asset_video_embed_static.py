from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_developer_asset_detail_embeds_video_tours_inline():
    html = (ROOT / "frontend/platform/developer/asset-detail.html").read_text(encoding="utf-8")
    js = (ROOT / "frontend/platform/static/js/developer-asset-detail.js").read_text(encoding="utf-8")
    css = (ROOT / "frontend/platform/static/css/developer-asset-detail.css").read_text(encoding="utf-8")

    assert 'id="video-embed"' in html
    assert 'id="video-link"' in html
    assert "Open video" in html

    assert "function renderVideoEmbed" in js
    assert "function normalizeVideoUrl" in js
    assert "function getYouTubeVideoId" in js
    assert "function getVimeoVideoId" in js
    assert "https://www.youtube-nocookie.com/embed/" in js
    assert "https://player.vimeo.com/video/" in js
    assert "document.createElement(\"iframe\")" in js
    assert "document.createElement(\"video\")" in js
    assert "[\"http:\", \"https:\"]" in js
    assert "document.getElementById(\"video-link\").href = a.video_url" not in js

    assert ".ad-video-embed" in css
    assert ".ad-video-embed iframe" in css
    assert ".ad-video-fallback" in css
