/**
 * Community ban-appeal banner + modal (14.8.1).
 *
 * On community page load, fetches the current user's community profile
 * via /api/community/profile/me and reveals the ban banner when the user
 * is community-banned. If a pending appeal already exists, the banner
 * shows "Appeal under review" instead of the submit CTA.
 *
 * Wire-up: the banner + modal markup live in frontend/platform/community.html.
 * The submit endpoint is POST /api/community/appeals (text 10-2000 chars).
 */
(function () {
  'use strict';

  const APPEAL_MIN = 10;
  const APPEAL_MAX = 2000;

  function csrfTokenFromCookie() {
    const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
    return match ? decodeURIComponent(match[1]) : '';
  }

  function showBanner({ reason, hasPendingAppeal }) {
    const banner = document.getElementById('community-ban-banner');
    if (!banner) return;
    const reasonEl = document.getElementById('community-ban-banner-reason');
    const appealBtn = document.getElementById('community-ban-appeal-btn');
    const pendingEl = document.getElementById('community-ban-pending-state');

    if (reasonEl) {
      reasonEl.textContent = reason && reason.trim().length > 0
        ? reason
        : 'Posting, commenting, and reactions are paused on your account.';
    }
    if (hasPendingAppeal) {
      if (appealBtn) appealBtn.hidden = true;
      if (pendingEl) pendingEl.hidden = false;
    } else {
      if (appealBtn) appealBtn.hidden = false;
      if (pendingEl) pendingEl.hidden = true;
    }
    banner.hidden = false;
  }

  function hideBanner() {
    const banner = document.getElementById('community-ban-banner');
    if (banner) banner.hidden = true;
  }

  function updateCounter(textarea, counterEl) {
    if (!textarea || !counterEl) return;
    counterEl.textContent = String(textarea.value.length);
  }

  function showAppealError(message) {
    const errEl = document.getElementById('ban-appeal-error');
    if (!errEl) return;
    errEl.textContent = message;
    errEl.hidden = false;
  }

  function hideAppealError() {
    const errEl = document.getElementById('ban-appeal-error');
    if (!errEl) return;
    errEl.textContent = '';
    errEl.hidden = true;
  }

  async function submitAppeal() {
    const textarea = document.getElementById('ban-appeal-text');
    const submitBtn = document.getElementById('ban-appeal-submit-btn');
    if (!textarea || !submitBtn) return;

    const appealText = textarea.value.trim();
    hideAppealError();

    if (appealText.length < APPEAL_MIN) {
      showAppealError(`Please write at least ${APPEAL_MIN} characters so the moderators can review your appeal.`);
      return;
    }
    if (appealText.length > APPEAL_MAX) {
      showAppealError(`Appeal is too long. Maximum ${APPEAL_MAX} characters.`);
      return;
    }

    submitBtn.disabled = true;
    submitBtn.textContent = 'Submitting…';
    try {
      const res = await fetch('/api/community/appeals', {
        method: 'POST',
        credentials: 'same-origin',
        headers: {
          'Content-Type': 'application/json',
          'X-CSRF-Token': csrfTokenFromCookie(),
        },
        body: JSON.stringify({ appeal_text: appealText }),
      });
      if (res.status === 409) {
        showAppealError('You already have a pending appeal. The team will review it shortly.');
        return;
      }
      if (!res.ok) {
        let detail = '';
        try {
          const payload = await res.json();
          detail = payload.error || payload.message || '';
        } catch (_e) { /* ignore */ }
        showAppealError(detail || 'We could not submit your appeal. Please try again.');
        return;
      }
      if (typeof window.closeCommunityModal === 'function') {
        window.closeCommunityModal('ban-appeal-modal');
      }
      textarea.value = '';
      const counterEl = document.getElementById('ban-appeal-counter');
      updateCounter(textarea, counterEl);
      if (typeof window.showToast === 'function') {
        window.showToast('Appeal submitted. The moderation team will review it.');
      }
      showBanner({ reason: null, hasPendingAppeal: true });
    } catch (e) {
      console.error('Ban appeal submission failed', e);
      showAppealError('Network error. Please try again.');
    } finally {
      submitBtn.disabled = false;
      submitBtn.textContent = 'Submit Appeal';
    }
  }

  async function init() {
    // Banner relies on /api/community/profile/me — gracefully ignore failures.
    let profile;
    try {
      const res = await fetch('/api/community/profile/me', { credentials: 'same-origin' });
      if (!res.ok) return;
      profile = await res.json();
    } catch (_e) {
      return;
    }

    if (!profile || profile.is_community_banned !== true) {
      hideBanner();
      return;
    }

    showBanner({
      reason: profile.ban_reason || null,
      hasPendingAppeal: profile.has_pending_appeal === true,
    });

    // Wire textarea counter + submit button now that the modal is reachable.
    const textarea = document.getElementById('ban-appeal-text');
    const counterEl = document.getElementById('ban-appeal-counter');
    if (textarea && counterEl) {
      textarea.addEventListener('input', function () {
        updateCounter(textarea, counterEl);
        if (textarea.value.length >= APPEAL_MIN) hideAppealError();
      });
      updateCounter(textarea, counterEl);
    }

    const submitBtn = document.getElementById('ban-appeal-submit-btn');
    if (submitBtn) {
      submitBtn.addEventListener('click', submitAppeal);
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
