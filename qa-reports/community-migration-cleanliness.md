# Community migrations — fresh-DB cleanliness check (WS1.4)

Created a throwaway database `poool_community_migration_test` and applied every
`database/community/*.sql` migration in lexicographic order with
`-v ON_ERROR_STOP=1`. All 37 migrations applied cleanly.

| Migration | Status |
|---|---|
| 001_posts.sql | ok |
| 002_comments.sql | ok |
| 003_reactions.sql | ok |
| 004_announcement_categories.sql | ok |
| 005_community_profiles.sql | ok |
| 006_indexes.sql | ok |
| 006_social_layer.sql | ok |
| 007_content_reports.sql | ok |
| 008_circles_xp.sql | ok |
| 009_amas.sql | ok |
| 010_reviews.sql | ok |
| 011_challenges.sql | ok |
| 012_notifications.sql | ok |
| 013_moderation.sql | ok |
| 014_shadowban.sql | ok |
| 015_link_previews.sql | ok |
| 016_ban_appeals.sql | ok |
| 017_circle_requests.sql | ok |
| 018_community_audit_log.sql | ok |
| 019_bookmarks.sql | ok |
| 020_polls.sql | ok |
| 021_hashtags.sql | ok |
| 025_token_gated_circles.sql | ok |
| 026_community_xp_nonnegative.sql | ok |
| 027_block_mute.sql | ok |
| 028_comment_edits.sql | ok |
| 029_comment_reactions.sql | ok |
| 030_notification_prefs.sql | ok |
| 031_notification_preferences.sql | ok |
| 031_verified_owner_requests.sql | ok |
| 032_comment_threading.sql | ok |
| 032_verification_requests.sql | ok |
| 033_comment_replies.sql | ok |
| 033_community_settings.sql | ok |
| 034_dms.sql | ok |
| 034_verified_owner_profile.sql | ok |
| 035_profile_views.sql | ok |

## Observations

- Two pairs of migrations share a number: `031_notification_preferences.sql` ↔ `031_verified_owner_requests.sql`, `032_comment_threading.sql` ↔ `032_verification_requests.sql`, `033_comment_replies.sql` ↔ `033_community_settings.sql`, `034_dms.sql` ↔ `034_verified_owner_profile.sql`. They apply in alphabetical order. No conflicts in this run, but future numbering should keep one file per number to avoid ordering surprises if any pair gets reshuffled by tooling.
- The duplicate numbering would be worth normalizing in a follow-up housekeeping pass, but renaming committed migrations would force every existing DB to re-run them or get out of sync, so we leave them alone.

## Procedure

```sh
createdb poool_community_migration_test
for f in $(ls database/community/*.sql | sort); do
  psql -d poool_community_migration_test -f "$f" -v ON_ERROR_STOP=1 || echo "FAIL: $f"
done
dropdb poool_community_migration_test
```
