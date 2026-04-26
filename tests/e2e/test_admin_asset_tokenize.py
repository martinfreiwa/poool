import os
import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def db_connect():
    return psycopg2.connect(DB_URL)


def create_user(cur, *, email_prefix, roles=()):
    email = f"{email_prefix}-{uuid.uuid4().hex[:8]}@poool.app"
    session_token = str(uuid.uuid4())
    cur.execute(
        """
        INSERT INTO users (email, email_verified, status)
        VALUES (%s, TRUE, 'active')
        RETURNING id, email
        """,
        (email,),
    )
    user_id, email = cur.fetchone()
    cur.execute(
        """
        INSERT INTO user_sessions (user_id, session_token, remember_me, expires_at)
        VALUES (%s, %s, FALSE, NOW() + INTERVAL '1 hour')
        """,
        (user_id, session_token),
    )
    for role in roles:
        cur.execute(
            """
            INSERT INTO user_roles (user_id, role_id, is_active)
            SELECT %s, id, TRUE FROM roles WHERE name = %s
            ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE
            """,
            (user_id, role),
        )
    return {"id": user_id, "email": email, "session_token": session_token}


def create_asset(cur):
    slug = f"e2e-tokenize-{uuid.uuid4().hex[:8]}"
    title = "E2E Tokenize <img src=x onerror=alert(1)> Asset"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published
        )
        VALUES (
            %s, %s, 'E2E tokenization fixture',
            'Seeded by the admin asset tokenization E2E test.', 'real_estate',
            250000000, 50000, 5000, 5000, 'funded', TRUE
        )
        RETURNING id
        """,
        (title, slug),
    )
    asset_id = cur.fetchone()[0]
    cur.execute(
        """
        INSERT INTO asset_documents (asset_id, document_type, title, file_url, file_size_bytes)
        VALUES (%s, 'legal_basis', 'E2E legal basis', 'gs://poool-e2e/legal-basis.pdf', 12345)
        """,
        (asset_id,),
    )
    return asset_id, title


def authenticated_session(session_token, page_path):
    session = requests.Session()
    session.cookies.set("poool_session", session_token)
    response = session.get(f"{BASE_URL}{page_path}", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def audit_count(cur, *, asset_id, actor_id):
    cur.execute(
        """
        SELECT COUNT(*)
        FROM audit_logs
        WHERE actor_user_id = %s
          AND entity_type = 'asset'
          AND entity_id = %s
          AND action = 'blockchain.tokenize'
        """,
        (actor_id, asset_id),
    )
    return cur.fetchone()[0]


@pytest.mark.admin
@pytest.mark.blockchain
def test_admin_asset_tokenize_permissions_csrf_mock_deploy_and_ui(quality_page):
    created_user_ids = []
    asset_id = None

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        normal_admin = create_user(cur, email_prefix="e2e-tokenize-admin", roles=("admin",))
        super_admin = create_user(
            cur,
            email_prefix="e2e-tokenize-super",
            roles=("admin", "super_admin"),
        )
        created_user_ids.extend([normal_admin["id"], super_admin["id"]])
        asset_id, asset_title = create_asset(cur)
        conn.commit()

        page_path = f"/admin/asset-tokenize?id={asset_id}"
        normal_session = requests.Session()
        normal_session.cookies.set("poool_session", normal_admin["session_token"])
        normal_page = normal_session.get(f"{BASE_URL}{page_path}", timeout=10)
        assert normal_page.url.endswith("/admin/")
        denied_api = normal_session.get(
            f"{BASE_URL}/api/admin/blockchain/tokenize/{asset_id}",
            timeout=10,
        )
        assert denied_api.status_code == 403

        no_csrf = requests.Session()
        no_csrf.cookies.set("poool_session", super_admin["session_token"])
        csrf_denied = no_csrf.post(
            f"{BASE_URL}/api/admin/blockchain/tokenize/{asset_id}",
            timeout=10,
        )
        assert csrf_denied.status_code == 403

        session = authenticated_session(super_admin["session_token"], page_path)
        check = session.get(
            f"{BASE_URL}/api/admin/blockchain/tokenize/{asset_id}",
            timeout=10,
        )
        assert check.status_code == 200, check.text
        check_payload = check.json()
        assert check_payload["title"] == asset_title
        assert check_payload["checks"]["all_passed"] is True
        assert check_payload["checks"]["legal_documents_present"] is True
        assert check_payload["checks"]["operator_can_tokenize"] is True

        page, tracker = quality_page
        page.context.add_cookies(
            [{"name": "poool_session", "value": super_admin["session_token"], "url": BASE_URL}]
        )
        tracker.navigate_and_check(f"{BASE_URL}/admin/asset-tokenize")
        seeded_candidate = page.locator(f".tokenize-candidate[href*='{asset_id}']")
        expect(seeded_candidate).to_be_visible()
        assert seeded_candidate.locator("img[src='x']").count() == 0

        tracker.navigate_and_check(f"{BASE_URL}{page_path}")
        expect(page.locator("#page-title")).to_have_text(f"Tokenize: {asset_title}")
        expect(page.locator("#checklist")).to_contain_text("Legal Documents Present")
        expect(page.locator("#checklist")).to_contain_text("Operator Permission Verified")
        assert page.locator("#checklist img[src='x']").count() == 0

        with page.expect_response(
            lambda response: response.url.endswith(f"/api/admin/blockchain/tokenize/{asset_id}")
            and response.request.method == "POST",
            timeout=15_000,
        ) as tokenize_response:
            page.locator("#btn-tokenize").click()
            expect(page.locator(".tokenize-modal")).to_be_visible()
            page.get_by_role("button", name="Deploy Asset").click()
        assert tokenize_response.value.status == 200
        payload = tokenize_response.value.json()
        assert payload["success"] is True
        # The local backend may be configured for either mock deployment or a
        # real test-chain deployment. Both modes must persist the same contract
        # metadata and audit trail; the exact mode is reported by the API.
        assert payload["mocked"] in (True, False)
        assert payload["chain_contract_address"].startswith("0x")

        expect(page.locator(".deploy-result")).to_contain_text("Asset Already Tokenized", timeout=10_000)
        expect(page.get_by_text(payload["chain_contract_address"], exact=True)).to_be_visible()
        assert page.locator(".deploy-result img[src='x']").count() == 0

        cur.execute(
            """
            SELECT chain_token_id, chain_contract_address, chain_tx_hash
            FROM assets
            WHERE id = %s
            """,
            (asset_id,),
        )
        chain_token_id, chain_contract_address, chain_tx_hash = cur.fetchone()
        assert chain_token_id == "1"
        assert chain_contract_address == payload["chain_contract_address"]
        assert chain_tx_hash == payload["chain_tx_hash"]
        assert audit_count(cur, asset_id=asset_id, actor_id=super_admin["id"]) == 1

        duplicate = session.post(
            f"{BASE_URL}/api/admin/blockchain/tokenize/{asset_id}",
            timeout=10,
        )
        assert duplicate.status_code == 400

        tracker.assert_no_critical_errors()
    finally:
        conn.rollback()
        if asset_id is not None:
            cur.execute("DELETE FROM audit_logs WHERE entity_id = %s", (asset_id,))
            cur.execute("DELETE FROM asset_tokenization_jobs WHERE asset_id = %s", (asset_id,))
            cur.execute("DELETE FROM asset_documents WHERE asset_id = %s", (asset_id,))
            cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
        if created_user_ids:
            cur.execute(
                "DELETE FROM audit_logs WHERE actor_user_id = ANY(%s::uuid[])",
                ([str(uid) for uid in created_user_ids],),
            )
            cur.execute(
                "DELETE FROM user_sessions WHERE user_id = ANY(%s::uuid[])",
                ([str(uid) for uid in created_user_ids],),
            )
            cur.execute(
                "DELETE FROM user_roles WHERE user_id = ANY(%s::uuid[])",
                ([str(uid) for uid in created_user_ids],),
            )
            cur.execute(
                "DELETE FROM users WHERE id = ANY(%s::uuid[])",
                ([str(uid) for uid in created_user_ids],),
            )
        conn.commit()
        cur.close()
        conn.close()
