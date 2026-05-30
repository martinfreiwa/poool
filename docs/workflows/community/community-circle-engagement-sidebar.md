# Community Circle Engagement Sidebar

Purpose: Verify Circle detail sidebar modules: announcements, events, resources, challenges, onboarding, rules/about content, and partial failures.

Prerequisites:
- User is logged in.
- Seed data includes one circle with sidebar content and one circle without sidebar content.
- Test both member and non-member views when possible.

Pages and endpoints covered:
- `/community/circle/:slug`
- `/api/community/circles/by-slug/:slug`
- `/api/community/circles/:id/announcements`
- `/api/community/circles/:id/events`
- `/api/community/circles/:id/resources`
- `/api/community/circles/:id/challenges`
- `/api/community/circles/:id/onboarding`
- `/api/community/circles/:id/onboarding/:step`

Steps:
1. Open a circle detail page with seeded sidebar content.
2. Verify about/rules text, announcements, events, resources, challenges, and onboarding modules render in the expected order.
3. Click each sidebar item and verify route, anchor, modal, or download behavior.
4. Complete an onboarding step and verify progress/state persists after reload.
5. Open the same circle as a non-member and verify restricted modules hide or show locked states according to access rules.
6. Open a circle with no sidebar content and verify empty states do not collapse the page layout awkwardly.
7. Simulate or observe one failed sidebar API while other modules still render.
8. Verify HTML/script in sidebar-provided content is escaped or sanitized.

Expected Result:
- Sidebar modules load independently and do not block the main feed.
- Member/non-member visibility matches circle access rules.
- Onboarding actions persist and are idempotent.
- Empty and partial-failure states are visible and recoverable.

Edge Cases:
- Unknown slug.
- Circle deleted after page shell loads.
- Empty announcements/events/resources/challenges/onboarding.
- Onboarding step already completed.
- Resource unavailable or unauthorized.
- Very long rules/about/sidebar titles.
