import re

with open("frontend/platform/static/js/portfolio-data.js", "r") as f:
    code = f.read()

# Replace empty state TR with DIV
old_empty = """<tr class="portfolio-assets-row">
          <td colspan="6" style="padding:48px; text-align:center;">
            <div style="display:flex; flex-direction:column; align-items:center; gap:12px; justify-content:center;">
              <img src="/static/images/home-smile.svg" alt="No assets" width="48" height="48" style="opacity:0.4;">
              <span style="color:#667085; font-size:14px;">No investments found.</span>
              <a href="/marketplace" style="color:#0000ff; font-size:14px; text-decoration:none;">Browse the Marketplace →</a>
            </div>
          </td>
        </tr>"""

new_empty = """<div class="portfolio-assets-row" style="justify-content:center; padding:48px;">
            <div style="display:flex; flex-direction:column; align-items:center; gap:12px; justify-content:center;">
              <img src="/static/images/home-smile.svg" alt="No assets" width="48" height="48" style="opacity:0.4;">
              <span style="color:#667085; font-size:14px;">No investments found.</span>
              <a href="/marketplace" style="color:#0000ff; font-size:14px; text-decoration:none;">Browse the Marketplace →</a>
            </div>
        </div>"""

code = code.replace(old_empty, new_empty)

# Replace table row renderer
old_row_start = """<tr class="data-table__row" onclick="window.location.href='/property/${slug}'" style="cursor:pointer;">"""
old_row_match = re.compile(re.escape(old_row_start) + r"(.*?)(?=</tr>`;\}\)\.join\(\"\"\);)", re.DOTALL)

new_row = """<div class="portfolio-assets-row" onclick="window.location.href='/property/${slug}'" style="cursor:pointer;">
        <div class="portfolio-assets-cell property-col">
          <div style="display:flex; align-items:center; gap:16px;">
            <img src="${cover}" alt="${title}" style="width: 56px; height: 40px; border-radius: 6px; object-fit: cover;" onerror="this.outerHTML='<div class=\\'property-image-placeholder\\'><svg width=\\'20\\' height=\\'20\\' viewBox=\\'0 0 24 24\\' fill=\\'none\\' stroke=\\'currentColor\\' stroke-width=\\'1.5\\' stroke-linecap=\\'round\\' stroke-linejoin=\\'round\\'><rect x=\\'3\\' y=\\'3\\' width=\\'18\\' height=\\'18\\' rx=\\'2\\' ry=\\'2\\'></rect><circle cx=\\'8.5\\' cy=\\'8.5\\' r=\\'1.5\\'></circle><polyline points=\\'21 15 16 10 5 21\\'></polyline></svg></div>'">
            <div style="font-weight: 700; color: #101828; font-size: 14px; line-height: 1.4; max-width: 200px;">
              ${title}<br/>${chainBadge}
            </div>
          </div>
        </div>
        <div class="portfolio-assets-cell investment-col" style="display:flex; flex-direction:column; align-items:flex-start; gap:4px; padding-top:4px;">
          <div style="font-weight: 700; color: #101828; font-size: 14px;">${escHtml(inv.currentValueDisplay)}</div>
          <span class="ds-badge" style="background: #FFFFFF; color: #475467; border: 1px solid #E9EAEB; padding: 2px 6px; font-weight: 600; font-size: 11px; display:inline-flex; align-items:center; gap:2px; border-radius: 4px;">
            <svg width="8" height="8" viewBox="0 0 12 12" fill="none"><path d="M3.5 8.5L8.5 3.5M8.5 3.5H3.5M8.5 3.5V8.5" stroke="#475467" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
            ${escHtml(inv.appreciationDisplay)}
          </span>
        </div>
        <div class="portfolio-assets-cell rental-col">
          <div style="font-weight: 700; color: #101828; font-size: 14px;">${escHtml(inv.totalRentalDisplay)}</div>
        </div>
        <div class="portfolio-assets-cell status-col">
          ${buildStatusBadgeHtml(statusCss, statusLabel)}
        </div>
        <div class="portfolio-assets-cell actions-col" onclick="event.stopPropagation();">
          ${(inv.isWithin48h && inv.originalStatus === 'funding_in_progress') ? `
          <button class="ds-btn ds-btn--ghost ds-btn--sm"
            style="color: #D92D20; border: 1px solid #FDA29B; background: #FEF3F2; margin-right: 8px;"
            onclick="window.cancelInvestment('${inv.id}')"
            id="cancel-btn-${inv.id}">
            Refund
          </button>
          ` : ''}
          <button class="ds-btn ds-btn--ghost ds-btn--sm"
            style="border: 1px solid #E9EAEB; border-radius: 8px; font-weight: 600; color: #475467; background:#FFFFFF;"
            onclick="window.location.href='/property/${slug}'">
            See Details
          </button>
        </div>
      </div>"""

v1 = code
code = old_row_match.sub(new_row, code)

if v1 == code:
    print("WARNING: Regex replace failed.")
else:
    print("Replaced data row structure.")

with open("frontend/platform/static/js/portfolio-data.js", "w") as f:
    f.write(code)

