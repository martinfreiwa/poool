# Community Autocomplete and Composer Suggestions

Purpose: Verify composer autocomplete for mentions, hashtags, assets, circle mentions, keyboard behavior, escaping, and stale suggestion handling.

Prerequisites:
- User is logged in.
- Seed data includes searchable users, hashtags, assets, and circles where supported.

Pages and endpoints covered:
- `/community?tab=feed`
- `/community/circle/:slug`
- `/api/community/mentions/suggest`
- `/api/community/hashtags/suggest`
- `/api/community/assets/suggest`
- `/api/community/circles/search`

Steps:
1. Open the feed composer.
2. Type `@` and a known user prefix; verify mention suggestions load.
3. Select a mention with mouse and keyboard and verify inserted text is correct.
4. Type `#` and a known hashtag prefix; verify suggestions load and insert safely.
5. Type `$` or the configured asset trigger; verify asset suggestions load and link/insert as expected.
6. In circle-aware composer flows, type a circle mention prefix and verify only visible/authorized circles are suggested.
7. Verify arrow keys, Enter, Escape, blur, and click-out behavior.
8. Submit a post with selected suggestions and verify rendered links go to the correct route.
9. Verify suggestion labels containing HTML/script render as text only.
10. Verify slow, empty, failed, and stale responses do not insert the wrong suggestion.

Expected Result:
- Autocomplete is keyboard-accessible and does not break normal typing.
- Suggestions are scoped to user visibility and privacy rules.
- Inserted mentions/hashtags/assets link to the correct pages after posting.
- User-generated suggestion labels cannot execute HTML/script.

Edge Cases:
- Empty query.
- Query with spaces, emoji, punctuation, and very long text.
- Suggestion deleted between fetch and selection.
- Private/hidden circle suggestion for unauthorized user.
- Duplicate display names.
- Network failure after menu opens.
