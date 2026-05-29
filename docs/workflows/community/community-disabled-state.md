# Community Disabled State

Purpose: Verify Community disabled/unavailable rendering, protected access, partial fallback behavior, and recovery when Community is re-enabled.

Prerequisites:
- Environment or seed state can simulate Community disabled/unavailable mode.
- User is logged in; also test logged-out access.

Pages and endpoints covered:
- `/community`
- `/community/partials/:tab`
- `frontend/platform/partials/community_disabled.html`
- `/api/admin/community/settings`

Steps:
1. Disable Community through the supported admin setting or test fixture.
2. Open `/community` as a logged-in user.
3. Verify disabled copy, iconography, and available navigation actions render without broken tab controls.
4. Open each Community partial route and verify it returns a safe disabled state or protected redirect.
5. Verify Community APIs that should be blocked return explicit disabled/authorization responses.
6. Verify pages outside Community still work.
7. Re-enable Community and reload `/community`.
8. Verify normal shell, tabs, and API calls recover without stale disabled cache.
9. Test logged-out direct access while disabled and verify login/protection behavior remains correct.

Expected Result:
- Disabled state is explicit and does not expose partial Community content.
- Re-enable restores normal Community behavior without requiring a browser cache clear.
- The disabled state does not break global navigation.

Edge Cases:
- Disable while user is composing a post.
- Disable while HTMX partial is loading.
- Disable while notifications/messages panel is open.
- Re-enable with stale browser tab.
- Admin setting update fails halfway.
