import re

with open('frontend/platform/commodities-preview.html', 'r') as f:
    content = f.read()

# Define the light block
light_block = """
      <!-- Project Details Section -->
      <div style="margin-top: 64px; border-top: 1px solid #e5e7eb; padding-top: 48px; display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 40px;">
        <div>
          <div style="color: #111827; font-weight: 800; font-size: 1.25rem; margin-bottom: 16px; display: flex; align-items: center; gap: 8px;">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#d4af37" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path></svg>
            The Initiative
          </div>
          <p style="color: #4b5563; line-height: 1.6; font-size: 1.125rem;">The National Chilli Reserve is a flagship agricultural stability fund initiated by the Ministry of Agriculture. It aims to stabilize regional price fluctuations by locking in supply agreements natively bridged to global capital.</p>
        </div>
        <div>
          <div style="color: #111827; font-weight: 800; font-size: 1.25rem; margin-bottom: 16px; display: flex; align-items: center; gap: 8px;">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#059669" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect><path d="M7 11V7a5 5 0 0 1 10 0v4"></path></svg>
            Zero Risk Vector
          </div>
          <p style="color: #4b5563; line-height: 1.6; font-size: 1.125rem;">Your capital is 100% safeguarded against sovereign and market turbulence through a binding state-sponsored offtake agreement. Yields are generated regardless of global market crop variations.</p>
        </div>
        <div>
          <div style="color: #111827; font-weight: 800; font-size: 1.25rem; margin-bottom: 16px; display: flex; align-items: center; gap: 8px;">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline></svg>
            Yield Generation
          </div>
          <p style="color: #4b5563; line-height: 1.6; font-size: 1.125rem;">Profits are distributed directly to your POOOL wallet linearly. This ultra-exclusive asset is typically reserved for institutional funds but is now accessible for limited early investor allocations.</p>
        </div>
      </div>
"""

# Define the dark block
dark_block = """
      <!-- Project Details Section -->
      <div style="margin-top: 64px; border-top: 1px solid #334155; padding-top: 48px; display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 40px;">
        <div>
          <div style="color: #f8fafc; font-weight: 800; font-size: 1.25rem; margin-bottom: 16px; display: flex; align-items: center; gap: 8px;">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#d4af37" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path></svg>
            The Initiative
          </div>
          <p style="color: #94a3b8; line-height: 1.6; font-size: 1.125rem;">The National Chilli Reserve is a flagship agricultural stability fund initiated by the Ministry of Agriculture. It aims to stabilize regional price fluctuations by locking in supply agreements natively bridged to global capital.</p>
        </div>
        <div>
          <div style="color: #f8fafc; font-weight: 800; font-size: 1.25rem; margin-bottom: 16px; display: flex; align-items: center; gap: 8px;">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#34d399" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect><path d="M7 11V7a5 5 0 0 1 10 0v4"></path></svg>
            Zero Risk Vector
          </div>
          <p style="color: #94a3b8; line-height: 1.6; font-size: 1.125rem;">Your capital is 100% safeguarded against sovereign and market turbulence through a binding state-sponsored offtake agreement. Yields are generated regardless of global market crop variations.</p>
        </div>
        <div>
          <div style="color: #f8fafc; font-weight: 800; font-size: 1.25rem; margin-bottom: 16px; display: flex; align-items: center; gap: 8px;">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#60a5fa" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline></svg>
            Yield Generation
          </div>
          <p style="color: #94a3b8; line-height: 1.6; font-size: 1.125rem;">Profits are distributed directly to your POOOL wallet linearly. This ultra-exclusive asset is typically reserved for institutional funds but is now accessible for limited early investor allocations.</p>
        </div>
      </div>
"""

# We'll split the content by <section class="option-container"
sections = content.split('<section class="option-container"')

new_content = sections[0]

for idx, section in enumerate(sections[1:], 1):
    # Determine dark or light based on the option title or background
    is_dark = "background: #0f172a" in section or "The Institutional Vault" in section or ("The Gold Standard Flex" in section) or ("The Signature Plaque" in section)
    block = dark_block if is_dark else light_block
    
    # We want to insert the block right before the last closing </div> of the container.
    # The structure ends with:
    #      </div>
    #    </div>
    #  </section>
    # So we'll replace the last `    </div>\n  </section>` with `[block]\n    </div>\n  </section>`
    
    parts = section.rsplit('</div>\n    </div>\n  </section>', 1)
    if len(parts) == 1:
        # maybe it's just </div>\n  </section> for option 5
        parts = section.rsplit('    </div>\n  </section>', 1)
        if len(parts) == 2:
            new_section = parts[0] + block + "    </div>\n  </section>"
        else:
            new_section = section # fallback
    else:
        new_section = parts[0] + "</div>\n" + block + "    </div>\n  </section>"
        
    new_content += '<section class="option-container"' + new_section

with open('frontend/platform/commodities-preview.html', 'w') as f:
    f.write(new_content)

print("Updated with detailed descriptions appended beneath each option's primary cards.")
