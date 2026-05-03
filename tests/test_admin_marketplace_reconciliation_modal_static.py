from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RECON_HTML = ROOT / "frontend/platform/admin/marketplace/reconciliation.html"
ADMIN_MP_CSS = ROOT / "frontend/platform/static/css/admin-marketplace.css"


def test_reconciliation_resolve_modal_uses_scoped_panel_classes():
    html = RECON_HTML.read_text(encoding="utf-8")
    css = ADMIN_MP_CSS.read_text(encoding="utf-8")

    modal_start = html.index('id="mm-reason-overlay"')
    modal_end = html.index("<!-- Command palette -->", modal_start)
    modal = html[modal_start:modal_end]

    assert "recon-resolve-overlay" in modal
    assert "recon-modal-panel recon-resolve-modal" in modal
    assert "recon-resolve-header" in modal
    assert "recon-resolve-body" in modal
    assert "recon-resolve-actions" in modal
    assert "recon-modal-field" in modal
    assert 'class="mp-modal"' not in modal
    assert "style=" not in modal

    for selector in (
        ".recon-modal-panel",
        ".recon-modal-panel--wide",
        ".recon-resolve-modal",
        ".recon-resolve-body",
        ".recon-modal-field",
        ".recon-resolve-actions",
    ):
        assert selector in css

    assert "width: min(480px, calc(100vw - 32px))" in css
    assert "max-height: calc(100vh - 48px)" in css
    assert "flex-direction: column" in css
    assert "justify-content: flex-end" in css


def test_reconciliation_detail_modal_avoids_shared_mp_modal_collision():
    html = RECON_HTML.read_text(encoding="utf-8")

    modal_start = html.index('id="mm-detail-overlay"')
    modal_end = html.index("<!-- Resolve Reason Modal -->", modal_start)
    modal = html[modal_start:modal_end]

    assert "recon-modal-panel recon-modal-panel--wide" in modal
    assert "recon-modal-header" in modal
    assert 'class="mp-modal"' not in modal
