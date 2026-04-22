import sys

with open('frontend/platform/landing-improved.html', 'r', encoding='utf-8') as f:
    content = f.read()

# Add CSS link
css_link = '<link rel="stylesheet" href="/static/css/landing-redesign.css">\n  <link rel="stylesheet" href="/static/css/landing-property-cards.css">'
content = content.replace('<link rel="stylesheet" href="/static/css/landing-property-cards.css">', css_link)

import re
# Find the featured properties section
# and replace it.

marketplace_html = """
  <!-- ═══ FEATURED PROPERTIES (Holographic Redesign) ═══ -->
  <section class="redesigned-section" id="marketplace">
    <div class="container">
      <div class="redesigned-header">
        <h2 class="redesigned-title">Featured Properties</h2>
        <p class="redesigned-subtitle">Hand-picked premium real estate with verified returns</p>
      </div>

      <div class="redesigned-grid">
        <!-- Card 1: Sunset Luxury Villa -->
        <div class="holo-property-card">
          <div class="holo-card-content-layer">
            <div class="holo-image-container">
              <div class="holo-glass-badge">Standard Leasehold</div>
              <img src="/static/images/villa1.webp" class="holo-property-image active" alt="Villa">
              <img src="/static/images/villa1_2.webp" class="holo-property-image" alt="Villa">
              <button class="holo-nav-btn holo-nav-prev"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6"/></svg></button>
              <button class="holo-nav-btn holo-nav-next"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg></button>
              <div class="holo-dots">
                <div class="holo-dot active"></div><div class="holo-dot"></div>
              </div>
            </div>
            
            <div class="holo-meta-row">
              <div class="holo-meta-item"><img src="/static/images/Bed.svg" width="16" height="16"> 4</div> • 
              <div class="holo-meta-item"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M22 13a3 3 0 0 0-3-3H5a3 3 0 0 0-3 3v2a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2Z"/></svg> 4 Bath</div> • 
              <div class="holo-meta-item">Canggu, ID</div>
            </div>

            <h3 class="holo-title">Sunset Luxury Villa</h3>

            <div class="holo-price-row">
              <div>
                <span class="holo-price-label">Share Price</span>
                <span class="holo-price">USD 500</span>
              </div>
            </div>

            <div class="holo-progress-bar">
              <div class="holo-progress-fill" style="width: 68%;"></div>
            </div>
            <div class="holo-progress-text">68% funded</div>

            <div class="holo-details-box">
              <div class="holo-detail-row">
                <span class="holo-detail-label">Projected return</span>
                <span class="holo-detail-val">6.0%</span>
              </div>
              <div class="holo-detail-row">
                <span class="holo-detail-label">Projected annualised net</span>
                <span class="holo-detail-val">12.4%</span>
              </div>
            </div>

            <a href="/signup" class="ds-btn ds-btn--primary btn-invest">Invest Now</a>
          </div>
        </div>

        <!-- Card 2: Ocean Breeze Penthouse -->
        <div class="holo-property-card">
          <div class="holo-card-content-layer">
            <div class="holo-image-container">
              <div class="holo-glass-badge">Freehold</div>
              <img src="/static/images/villa4_1.webp" class="holo-property-image active" alt="Villa">
              <img src="/static/images/villa4_2.webp" class="holo-property-image" alt="Villa">
              <button class="holo-nav-btn holo-nav-prev"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6"/></svg></button>
              <button class="holo-nav-btn holo-nav-next"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg></button>
              <div class="holo-dots">
                <div class="holo-dot active"></div><div class="holo-dot"></div>
              </div>
            </div>
            
            <div class="holo-meta-row">
              <div class="holo-meta-item"><img src="/static/images/Bed.svg" width="16" height="16"> 2</div> • 
              <div class="holo-meta-item"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M22 13a3 3 0 0 0-3-3H5a3 3 0 0 0-3 3v2a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2Z"/></svg> 2 Bath</div> • 
              <div class="holo-meta-item">Seminyak, ID</div>
            </div>

            <h3 class="holo-title">Ocean Breeze Penthouse</h3>

            <div class="holo-price-row">
              <div>
                <span class="holo-price-label">Share Price</span>
                <span class="holo-price">USD 380</span>
              </div>
            </div>

            <div class="holo-progress-bar">
              <div class="holo-progress-fill" style="width: 91%;"></div>
            </div>
            <div class="holo-progress-text">91% funded</div>

            <div class="holo-details-box">
              <div class="holo-detail-row">
                <span class="holo-detail-label">Projected return</span>
                <span class="holo-detail-val">3.8%</span>
              </div>
              <div class="holo-detail-row">
                <span class="holo-detail-label">Projected annualised net</span>
                <span class="holo-detail-val">11.2%</span>
              </div>
            </div>

            <a href="/signup" class="ds-btn ds-btn--primary btn-invest">Invest Now</a>
          </div>
        </div>

        <!-- Card 3: Echo Beach Loft -->
        <div class="holo-property-card">
          <div class="holo-card-content-layer">
            <div class="holo-image-container">
              <div class="holo-glass-badge">Standard Leasehold</div>
              <img src="/static/images/villa3_2.webp" class="holo-property-image active" alt="Villa">
              <img src="/static/images/villa3_1.webp" class="holo-property-image" alt="Villa">
              <button class="holo-nav-btn holo-nav-prev"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6"/></svg></button>
              <button class="holo-nav-btn holo-nav-next"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg></button>
              <div class="holo-dots">
                <div class="holo-dot active"></div><div class="holo-dot"></div>
              </div>
            </div>
            
            <div class="holo-meta-row">
              <div class="holo-meta-item"><img src="/static/images/Bed.svg" width="16" height="16"> 2</div> • 
              <div class="holo-meta-item"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M22 13a3 3 0 0 0-3-3H5a3 3 0 0 0-3 3v2a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2Z"/></svg> 2 Bath</div> • 
              <div class="holo-meta-item">Canggu, ID</div>
            </div>

            <h3 class="holo-title">Echo Beach Loft</h3>

            <div class="holo-price-row">
              <div>
                <span class="holo-price-label">Share Price</span>
                <span class="holo-price">USD 280</span>
              </div>
            </div>

            <div class="holo-progress-bar">
              <div class="holo-progress-fill" style="width: 8%;"></div>
            </div>
            <div class="holo-progress-text">8% funded</div>

            <div class="holo-details-box">
              <div class="holo-detail-row">
                <span class="holo-detail-label">Projected return</span>
                <span class="holo-detail-val">8.0%</span>
              </div>
              <div class="holo-detail-row">
                <span class="holo-detail-label">Projected annualised net</span>
                <span class="holo-detail-val">16.2%</span>
              </div>
            </div>

            <a href="/signup" class="ds-btn ds-btn--primary btn-invest">Invest Now</a>
          </div>
        </div>
      </div>
    </div>
  </section>

  <!-- ═══ COMMODITIES SECTION ═══ -->
  <section class="redesigned-section commodities-section" id="commodities">
    <div class="container">
      <div class="redesigned-header">
        <h2 class="redesigned-title">Fractional Commodities</h2>
        <p class="redesigned-subtitle">Diversify your portfolio with physical assets backed by real-world value.</p>
      </div>

      <div class="redesigned-grid">
        <!-- Gold -->
        <div class="holo-property-card">
          <div class="holo-card-content-layer">
            <div class="holo-image-container" style="height: 180px;">
              <div class="holo-glass-badge" style="background: rgba(255,215,0,0.9); color: #000;">Precious Metal</div>
              <img src="/static/images/gold.webp" class="holo-property-image active" alt="Gold Vault" onerror="this.src='data:image/svg+xml,%3Csvg xmlns=\\'http://www.w3.org/2000/svg\\' width=\\'400\\' height=\\'200\\' style=\\'background:%232a2a2a\\'/%3E'">
            </div>

            <h3 class="holo-title">Physical Gold ( vaulted )</h3>

            <div class="holo-price-row">
              <div>
                <span class="holo-price-label">Price per 1g</span>
                <span class="holo-price">USD 75.30</span>
              </div>
            </div>

            <div class="holo-details-box">
              <div class="holo-detail-row">
                <span class="holo-detail-label">Historical 1-yr return</span>
                <span class="holo-detail-val" style="color:var(--brand-greeny-green)">+12.4%</span>
              </div>
              <div class="holo-detail-row">
                <span class="holo-detail-label">Storage Fee</span>
                <span class="holo-detail-val">0.5% p.a.</span>
              </div>
            </div>

            <a href="/signup" class="ds-btn ds-btn--primary btn-invest">Buy Gold</a>
          </div>
        </div>

        <!-- Rare Whiskey -->
        <div class="holo-property-card">
          <div class="holo-card-content-layer">
            <div class="holo-image-container" style="height: 180px;">
              <div class="holo-glass-badge">Alternative Asset</div>
              <img src="/static/images/whiskey.webp" class="holo-property-image active" alt="Rare Whiskey Collection" onerror="this.src='data:image/svg+xml,%3Csvg xmlns=\\'http://www.w3.org/2000/svg\\' width=\\'400\\' height=\\'200\\' style=\\'background:%23321\\'/%3E'">
            </div>

            <h3 class="holo-title">Rare Highland Whiskey Cask</h3>

            <div class="holo-price-row">
              <div>
                <span class="holo-price-label">Share Price</span>
                <span class="holo-price">USD 150</span>
              </div>
            </div>

            <div class="holo-details-box">
              <div class="holo-detail-row">
                <span class="holo-detail-label">Estimated Yield</span>
                <span class="holo-detail-val" style="color:var(--brand-greeny-green)">+8.5%</span>
              </div>
              <div class="holo-detail-row">
                <span class="holo-detail-label">Maturation Date</span>
                <span class="holo-detail-val">2030</span>
              </div>
            </div>

            <a href="/signup" class="ds-btn ds-btn--primary btn-invest">Invest</a>
          </div>
        </div>

      </div>
    </div>
  </section>
"""

# Extract the block to replace
start_idx = content.find('<!-- ═══ FEATURED PROPERTIES ═══ -->')
end_idx = content.find('<!-- ═══ HOW IT WORKS ═══ -->')

if start_idx != -1 and end_idx != -1:
    content = content[:start_idx] + marketplace_html + '\n\n  ' + content[end_idx:]

with open('frontend/platform/landing-improved.html', 'w', encoding='utf-8') as f:
    f.write(content)
print("Updated landing-improved.html successfully.")
