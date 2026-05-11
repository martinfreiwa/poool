/**
 * community-announcements.js — Phase 3 task 23.
 *
 * Previously this file delegated clicks on [data-community-ann-read-more] and
 * switched the tab to Feed as a stand-in for an announcement detail page.
 * Announcements are now full posts (post_type='announcement') so the list
 * partial links directly to /community/post/:id, which the server already
 * renders with comments and reactions via page_community_post. Nothing to
 * delegate here.
 *
 * Kept as a no-op module so it remains in the community.html extra_js list
 * without surprising anyone who deploys before the new template ships.
 */
(function () {
  // Intentionally empty.
})();
