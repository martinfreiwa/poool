import sys

with open('frontend/platform/landing-improved.html', 'r', encoding='utf-8') as f:
    content = f.read()

card4_html = """
        <!-- Card 4: Rice Terrace Retreat -->
        <div class="holo-property-card">
          <div class="holo-card-content-layer">
            <div class="holo-image-container">
              <div class="holo-glass-badge">Long Leasehold</div>
              <img src="/static/images/villa5.webp" class="holo-property-image active" alt="Villa">
              <img src="/static/images/villa6.webp" class="holo-property-image" alt="Villa">
              <button class="holo-nav-btn holo-nav-prev"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6"/></svg></button>
              <button class="holo-nav-btn holo-nav-next"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg></button>
              <div class="holo-dots">
                <div class="holo-dot active"></div><div class="holo-dot"></div>
              </div>
            </div>
            
            <div class="holo-meta-row">
              <div class="holo-meta-item"><img src="/static/images/Bed.svg" width="16" height="16"> 3</div> • 
              <div class="holo-meta-item"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M22 13a3 3 0 0 0-3-3H5a3 3 0 0 0-3 3v2a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2Z"/></svg> 3 Bath</div> • 
              <div class="holo-meta-item">Ubud, ID</div>
            </div>

            <h3 class="holo-title">Rice Terrace Retreat</h3>

            <div class="holo-price-row">
              <div>
                <span class="holo-price-label">Share Price</span>
                <span class="holo-price">USD 420</span>
              </div>
            </div>

            <div class="holo-progress-bar">
              <div class="holo-progress-fill" style="width: 45%;"></div>
            </div>
            <div class="holo-progress-text">45% funded</div>

            <div class="holo-details-box">
              <div class="holo-detail-row">
                <span class="holo-detail-label">Projected return</span>
                <span class="holo-detail-val">7.2%</span>
              </div>
              <div class="holo-detail-row">
                <span class="holo-detail-label">Projected annualised net</span>
                <span class="holo-detail-val">14.8%</span>
              </div>
            </div>

            <a href="/signup" class="ds-btn ds-btn--primary btn-invest">Invest Now</a>
          </div>
        </div>
"""

# Now we need to insert this into the HTML before the closing </div> of redesigned-grid.
# We will find Echo Beach Loft's closing div and insert card4_html.

search_str = '<!-- Card 3: Echo Beach Loft -->'
# Wait, let's just insert it before `</div>\n    </div>\n  </section>\n\n  <!-- ═══ COMMODITIES SECTION ═══ -->`

split_marker = '</div>\n    </div>\n  </section>\n\n  <!-- ═══ COMMODITIES SECTION ═══ -->'
if split_marker in content:
    content = content.replace(split_marker, card4_html + '\n      ' + split_marker)

with open('frontend/platform/landing-improved.html', 'w', encoding='utf-8') as f:
    f.write(content)
print("Added card 4 successfully.")
