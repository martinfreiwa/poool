# Community a11y audit — WS2.7

Static grep-based pass. A full axe-core run still requires a browser
session and is left for a follow-up.

## Findings

| Severity | Surface | Issue | Status |
|---|---|---|---|
| info  | community.html, community-profile.html | `<button><svg></button>` without text — every such button audited and all have `aria-label` set. | pass |
| info  | All `role="dialog"` modals — every modal has `aria-modal="true"` and `aria-labelledby`. | pass |
| info  | All `<img>` elements have `alt` attributes (either descriptive or empty for decorative). | pass |
| info  | No `tabindex > 0` anywhere — focus order follows DOM. | pass |
| **fix**   | community-profile tablist | Tabs missing `aria-controls` pointing at panels; missing roving tabindex; no arrow-key navigation. | **fixed in this commit** |
| **fix**   | community-profile panels | Missing `role="tabpanel"` and `tabindex="0"` so screen-reader virtual cursor can land on them. | **fixed in this commit** |

## Fixes shipped

1. Every `.community-profile-tab` now carries `aria-controls="community-profile-panel-<name>"` and a roving `tabindex` (0 on the active tab, -1 on the rest). `setActiveTab` keeps it in sync.
2. Every `.community-profile-panel` is marked `role="tabpanel"` with `tabindex="0"` so the panel itself is focusable.
3. The tablist binds a `keydown` listener: ArrowRight/Left wraps through tabs, Home/End jump to first/last; activating a tab also moves focus to it.

## Deferred

- Full axe-core sweep across community.html, community-profile.html, community-hashtag.html, and every admin/community/*.html page. Needs a browser session — programmatic run is straightforward (`axe.run(document)` via preview_eval) but the result set is too noisy to triage without a live page open.
- Colour-contrast spot checks per WCAG AA 4.5:1 across light + dark themes. The token system handles most of this; the remaining suspects are the hand-coded `#EEF4FF` and `#D1E0FF` in `.community-hashtag-banner` which should remap to design tokens.
- Mobile screen-reader smoke (VoiceOver iOS / TalkBack) of the new profile page tab navigation.
