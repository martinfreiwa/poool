from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_community_page_loads_challenges_controller():
    html = read("frontend/platform/community.html")
    js = read("frontend/platform/static/js/community-challenges.js")
    partial = read("frontend/platform/partials/community_challenges.html")

    assert "'community-challenges'" in html
    assert "window.communityChallenges = function" in js
    assert 'fetch("/api/community/challenges"' in js
    assert 'x-data="communityChallenges()"' in partial
    assert "challengeProgressStyle(ch)" in partial


def test_ama_fragment_and_controller_share_dom_contract():
    partial = read("frontend/platform/partials/community_ama.html")
    js = read("frontend/platform/static/js/community-amas.js")

    required_ids = [
        "ama-loading",
        "ama-empty",
        "ama-content",
        "ama-status-badge",
        "ama-title",
        "ama-description",
        "ama-date-time",
        "ama-expert-name",
        "ama-expert-avatar",
        "ama-expert-title",
        "ama-questions-list",
        "ama-question-input",
        "ama-question-submit-btn",
    ]
    for element_id in required_ids:
        assert f'id="{element_id}"' in partial
        assert element_id in js

    stale_selectors = [
        "ama-hero",
        "ama-questions-section",
        "ama-past-section",
        "ama-question-modal",
        "ama-question-charcount",
    ]
    for selector in stale_selectors:
        assert selector not in js

    assert "loadAmas();" in js
    assert "replaceChildren" in js
    assert "textContent" in js


def test_circle_dynamic_user_data_uses_dom_text_nodes():
    js = read("frontend/platform/static/js/community-circles.js")

    forbidden_patterns = [
        "${c.name}",
        "${c.avatar_emoji",
        "${req.user_name",
        "${inv.inviter_id",
        "container.innerHTML = html",
        "onclick=\"handleJoinCircle",
        "onclick=\"handleRequestJoinCircle",
        "onclick=\"handleAcceptInvite",
        "onclick=\"handleApproveRequest",
    ]
    for pattern in forbidden_patterns:
        assert pattern not in js

    assert "title.textContent = c.name" in js
    assert "emoji.textContent = c.avatar_emoji" in js
    assert "name.textContent = req.user_name" in js
    assert "createButton('Join'" in js
    assert "createButton('Accept'" in js
    assert "createButton('✓ Approve'" in js
    assert "loadAll();" in js


def test_circle_modals_have_accessible_dialog_contracts():
    partial = read("frontend/platform/partials/community_circle.html")
    page = read("frontend/platform/community.html")

    for modal_id in [
        "create-circle-modal",
        "invite-modal",
        "circle-settings-modal",
    ]:
        assert f'id="{modal_id}"' in partial
        modal_start = partial.index(f'id="{modal_id}"')
        modal_snippet = partial[modal_start: modal_start + 1200]
        assert 'role="dialog"' in modal_snippet
        assert 'aria-modal="true"' in modal_snippet
        assert 'aria-labelledby=' in modal_snippet
        assert 'aria-describedby=' in modal_snippet
        assert 'aria-hidden="true"' in modal_snippet
        # WS2.1 inline-style purge: max-height + overflow now live in CSS,
        # not on the element. Just confirm the modal has a class anchor.
        assert 'class="' in modal_snippet

    for control_id in [
        "circle-name-input",
        "circle-desc-input",
        "circle-emoji-input",
        "settings-circle-name",
        "settings-circle-desc",
        "settings-circle-emoji",
        "settings-circle-public",
    ]:
        assert f'for="{control_id}"' in partial or f'aria-label="Public circle"' in partial

    assert "window._communityModalPreviousFocus" in page
    assert "modal.setAttribute('aria-hidden', 'false')" in page
    assert "modal.setAttribute('aria-hidden', 'true')" in page
    assert "event.key === 'Escape'" in page
    assert "event.key !== 'Tab'" in page
    assert "last.focus()" in page
    assert "first.focus()" in page


def test_announcement_fragment_uses_server_rendered_contract():
    tab = read("frontend/platform/partials/community_announcements.html")
    list_template = read("frontend/platform/partials/community_announcements_list.html")
    js = read("frontend/platform/static/js/community-announcements.js")
    main = read("backend/src/main.rs")

    for category in [
        "new_commodity",
        "dividend",
        "platform_update",
        "market_news",
        "farm_update",
    ]:
        assert f"category={category}" in tab
        assert category in list_template
        assert f'"{category}"' in main

    stale_categories = ["new_commodities", "dividends", "platform_updates"]
    for category in stale_categories:
        assert category not in tab
        assert category not in list_template

    handler = main.split("async fn community_announcements_list_htmx", 1)[1].split(
        "/// GET /community", 1
    )[0]

    assert "Result<axum::response::Response, crate::error::AppError>" in handler
    assert "Invalid announcement category." in handler
    assert ".await?" in handler
    assert "unwrap_or_default" not in handler

    assert "/api/community/feed" not in js
    # WS task 23: announcements now link directly to the SSR post detail
    # page instead of dispatching a tab-switch event via JS, so the
    # "Read more" affordance is a real <a> tag.
    assert '<a class="ann-read-more" href="/community/post/' in list_template
    assert "onclick=" not in list_template
    assert "data-community-ann-read-more" not in list_template
