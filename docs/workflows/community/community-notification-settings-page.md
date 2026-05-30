# Community Notification Settings Page

Purpose: Verify the dedicated Community notification settings page, preference load/save behavior, fallback states, and alignment with Community tab preferences.

Prerequisites:
- User is logged in.
- Record the starting notification preference state and restore it after the pass.

Pages and endpoints covered:
- `/settings/notifications/community`
- `/community?tab=notifications`
- `/api/community/notifications/preferences`

Steps:
1. Open `/settings/notifications/community` directly.
2. Verify all preference rows render with current server state.
3. Toggle one test-safe preference and save.
4. Reload the settings page and verify persistence.
5. Open `/community?tab=notifications` and verify the same preference state is reflected there.
6. Toggle the preference from the Community notifications panel and verify the settings page reflects the change.
7. Restore the original preference state.
8. Verify loading, empty/fallback, validation-error, network-error, and session-expired states.

Expected Result:
- The dedicated settings page and Community notifications panel share the canonical preference API.
- Saves persist once and do not overwrite unrelated preferences.
- Legacy notification preference endpoints are not used.
- User receives clear feedback for success and failure.

Edge Cases:
- Preference object has unknown keys.
- Preference object is missing a key.
- Rapid save clicks.
- Server rejects malformed boolean values.
- Logged-out direct access.
- Network failure after optimistic UI update.
