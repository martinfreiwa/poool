import re

with open('frontend/platform/partials/community_feed.html', 'r') as f:
    content = f.read()

replacement = """      <!-- Create Post Inline UI -->
      <div class="community-create-post" style="padding: 16px; display: flex; flex-direction: column; gap: 16px; background: #fff; border: 1px solid var(--card-border-color); border-radius: 12px; margin-bottom: 24px;">
        
        <!-- User Info & Post Type Pill -->
        <div style="display: flex; align-items: center; gap: 12px;">
          <div id="fb-compose-avatar" class="ds-avatar ds-avatar--md" style="width: 40px; height: 40px; border-radius: 50%; display: flex; align-items: center; justify-content: center; background: #e4e6eb; color: #050505; font-size: 18px; font-weight: 600;">U</div>
          <div style="display: flex; flex-direction: column; align-items: flex-start;">
            <span id="fb-compose-name" style="font-weight: 600; font-size: 15px; color: #050505; margin-bottom: 2px;">User</span>
            
            <!-- Privacy Dropdown -->
            <div style="position: relative; display: inline-block;">
              <select id="fb-post-type-select" onchange="document.getElementById('post-type-input').value = this.value;" style="appearance: none; -webkit-appearance: none; background: #e4e6eb; border: none; padding: 4px 24px 4px 8px; border-radius: 6px; font-size: 13px; font-weight: 600; color: #050505; cursor: pointer;">
                <option value="general">🌍 General</option>
                <option value="market_insight">📈 Insight</option>
                <option value="review">⭐ Review</option>
              </select>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#050505" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="position: absolute; right: 8px; top: 8px; pointer-events: none;">
                <polyline points="6 9 12 15 18 9"></polyline>
              </svg>
            </div>
            <input type="hidden" id="post-type-input" value="general">
          </div>
        </div>

        <!-- Text Area -->
        <textarea id="post-content-input" rows="3" style="width: 100%; border: none; outline: none; box-shadow: none; font-size: 18px; line-height: 1.4; resize: none; padding: 0; font-family: inherit; color: #1c1e21;" placeholder="What's on your mind?"></textarea>
        <div class="ds-helper-text" id="post-disclaimer-warning" style="display: none; color: var(--btn-danger-bg); margin-top: 8px;">
          ⚠️ Your post appears to discuss investments. A disclaimer will be automatically added.
        </div>

        <!-- Image Previews -->
        <div id="post-image-previews" style="display:flex; gap:8px; overflow-x:auto; min-height: 0;"></div>
        
        <!-- UX.11: Poll Creator -->
        <div id="poll-creator" class="poll-creator" style="display: none; border: 1px solid #ced0d4; border-radius: 8px; padding: 16px;">
          <div class="poll-creator-header" style="display: flex; justify-content: space-between; margin-bottom: 12px;">
            <h4 style="margin: 0; font-size: 15px;">📊 Create a Poll</h4>
            <button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" onclick="togglePollCreator()" style="border: none; background: transparent; cursor: pointer;">✕</button>
          </div>
          <input type="text" id="poll-question-input" class="ds-input" placeholder="Ask a question..." style="margin-bottom: 12px; border-color: #ced0d4;">
          <div class="poll-option-inputs" id="poll-options-inputs" style="display: flex; flex-direction: column; gap: 8px;">
            <input type="text" class="ds-input" placeholder="Option 1" style="border-color: #ced0d4;" oninput="updatePollOption(0, this.value)">
            <input type="text" class="ds-input" placeholder="Option 2" style="border-color: #ced0d4;" oninput="updatePollOption(1, this.value)">
          </div>
          <div style="display: flex; align-items: center; justify-content: space-between; margin-top: 12px;">
            <button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" onclick="addPollOption()">+ Add</button>
            <select id="poll-expiry-select" class="ds-input" style="width: auto; padding: 4px 8px; font-size: 12px;">
              <option value="">No expiry</option>
              <option value="24" selected>1 day</option>
              <option value="168">1 week</option>
            </select>
          </div>
        </div>

        <!-- Add to your post -->
        <div style="border: 1px solid #ced0d4; border-radius: 8px; padding: 8px 16px; display: flex; align-items: center; justify-content: space-between; box-shadow: 0 1px 2px rgba(0,0,0,0.05);">
          <span style="font-weight: 600; font-size: 14px; color: #050505;">Add to your post</span>
          <div style="display: flex; gap: 4px; align-items: center;">
            <input type="file" id="post-image-file-input" accept="image/jpeg,image/png,image/webp" style="display:none;" onchange="uploadPostImage(event)">
            <!-- Photo Upload Icon -->
            <button type="button" style="background: none; border: none; cursor: pointer; padding: 8px; border-radius: 50%; display: flex; align-items: center; justify-content: center; transition: background 0.2s;" onmouseover="this.style.background='#f2f2f2'" onmouseout="this.style.background='transparent'" onclick="document.getElementById('post-image-file-input').click()" title="Photo/Video">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#45bd62" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline>
              </svg>
            </button>
            <!-- Tag People Icon -->
            <button type="button" style="background: none; border: none; cursor: pointer; padding: 8px; border-radius: 50%; display: flex; align-items: center; justify-content: center; transition: background 0.2s;" onmouseover="this.style.background='#f2f2f2'" onmouseout="this.style.background='transparent'" onclick="triggerComposerTag('@')" title="Tag People">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#1877f2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M16 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="8.5" cy="7" r="4"></circle><line x1="20" y1="8" x2="20" y2="14"></line><line x1="23" y1="11" x2="17" y2="11"></line>
              </svg>
            </button>
            <!-- Tag Asset Icon -->
            <button type="button" style="background: none; border: none; cursor: pointer; padding: 8px; border-radius: 50%; display: flex; align-items: center; justify-content: center; transition: background 0.2s;" onmouseover="this.style.background='#f2f2f2'" onmouseout="this.style.background='transparent'" onclick="triggerComposerTag('$')" title="Tag Asset">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#e83f5b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="12" y1="1" x2="12" y2="23"></line><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path>
              </svg>
            </button>
            <!-- Poll Icon -->
            <button type="button" style="background: none; border: none; cursor: pointer; padding: 8px; border-radius: 50%; display: flex; align-items: center; justify-content: center; transition: background 0.2s;" onmouseover="this.style.background='#f2f2f2'" onmouseout="this.style.background='transparent'" onclick="togglePollCreator()" title="Create Poll">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#f5c33b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M18 20V10"></path><path d="M12 20V4"></path><path d="M6 20v-6"></path>
              </svg>
            </button>
          </div>
        </div>
        <div class="ds-helper-text" id="post-image-uploading" style="display: none; color: var(--brand-blue);">Uploading image...</div>
        <div>
          <button type="button" class="ds-btn ds-btn--primary" id="submit-post-btn" onclick="submitUserPost()" style="width: 100%; font-size: 15px; font-weight: 600; padding: 10px; border-radius: 6px;">Post</button>
        </div>
      </div>"""

pattern = re.compile(r'<!-- Create Post -->\s*<div class="community-create-post">.*?</div>\s*</div>\s*</div>\s*<div id="community-feed-container"', re.DOTALL)
new_content = pattern.sub(replacement + '\n\n      <div id="community-feed-container"', content)

with open('frontend/platform/partials/community_feed.html', 'w') as f:
    f.write(new_content)
