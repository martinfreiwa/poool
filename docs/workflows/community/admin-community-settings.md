# Admin Community Settings

Purpose: Verify admin-managed Community settings, disabled-state controls, validation, persistence, and user-facing effects.

Prerequisites:
- Admin account has Community settings permissions.
- Record starting settings and restore them after the pass.

Pages and endpoints covered:
- `/admin/community/settings.html`
- `/api/admin/community/settings`
- `/community`
- `/community/partials/:tab`

Steps:
1. Open `/admin/community/settings.html`.
2. Verify all settings load from `/api/admin/community/settings`.
3. Change one test-safe setting and save.
4. Reload admin settings and verify persistence.
5. Verify the corresponding user-facing Community behavior changes after reload.
6. Toggle Community disabled/unavailable mode only in a controlled local/test environment.
7. Verify `/community` and partials reflect disabled state, then restore the original setting.
8. Try invalid values for booleans, numbers, URLs, text limits, and unknown keys.
9. Verify read-only admin and normal user access is blocked.

Expected Result:
- Settings save atomically and persist after reload.
- User-facing effects match setting names and do not require manual cache clearing.
- Invalid or unknown settings are rejected without overwriting valid settings.
- Original state is restored at the end.

Edge Cases:
- Concurrent admin saves.
- Partial settings payload.
- Missing CSRF token.
- Setting references deleted announcement/challenge/AMA.
- Disable Community while a user is on a client-side tab.
