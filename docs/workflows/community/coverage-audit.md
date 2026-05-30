# Community Workflow Coverage Audit

Purpose: Record how Community workflow coverage was audited so future agents can repeat the route/API/link inventory pass before adding or changing workflows.

Audit scope:
- User-facing Community routes from `backend/src/lib.rs`.
- Admin Community routes from `backend/src/admin/mod.rs`.
- Community APIs from `backend/src/community/routes.rs`.
- Community templates and partials under `frontend/platform/`.
- Community and admin Community JavaScript under `frontend/platform/static/js/`.
- Dedicated Community notification settings route under `backend/src/settings/mod.rs`.

Current workflow index:
- Canonical index: `docs/workflows/community/README.md`.
- Top-level pointer: `docs/WORKFLOWS.md`.

Coverage validation commands:

```bash
LC_ALL=en_US.UTF-8 ruby -e 'routes=File.read("backend/src/community/routes.rs", encoding: "UTF-8").scan(/\.route\(\s*"(\/api\/(?:admin\/)?community[^"]*)"/m).flatten.uniq.sort; docs=Dir["docs/workflows/community/*.md"].map{|f| File.read(f, encoding: "UTF-8")}.join("\n"); missing=routes.reject{|r| docs.include?(r)}; puts missing.empty? ? "all community api routes mentioned" : missing.join("\n")'
```

```bash
LC_ALL=en_US.UTF-8 ruby -e 'routes=File.read("backend/src/lib.rs", encoding: "UTF-8").scan(/\.route\(\s*"(\/community[^"]*)"/m).flatten.uniq.sort + File.read("backend/src/admin/mod.rs", encoding: "UTF-8").scan(/\.route\(\s*"(\/admin\/community[^"]*)"/m).flatten.uniq.sort + File.read("backend/src/settings/mod.rs", encoding: "UTF-8").scan(/\.route\(\s*"(\/settings\/notifications\/community[^"]*)"/m).flatten.uniq.sort; docs=Dir["docs/workflows/community/*.md"].map{|f| File.read(f, encoding: "UTF-8")}.join("\n"); missing=routes.uniq.reject{|r| docs.include?(r) || r == "/admin/community/"}; puts missing.empty? ? "all community page routes mentioned" : missing.join("\n")'
```

```bash
for f in $(rg -o '\]\(\./[^)]+\.md\)' docs/workflows/community/README.md | sed 's/.*](\.\///; s/)//' | sort -u); do test -f "docs/workflows/community/$f" || echo "missing $f"; done
```

```bash
rg -n "badge/:code|community/profile/:user_id|/api/community/reports|TODO|FIXME" docs/workflows/community docs/WORKFLOWS.md
```

Expected validation result:
- `all community api routes mentioned`
- `all community page routes mentioned`
- No missing README links.
- No stale route patterns or TODO/FIXME markers from the final search.

Known audit notes:
- Direct `?tab=search`, `?tab=notifications`, `?tab=dms`, `?tab=saved`, and `?tab=members` were rechecked in the 2026-05-29 browser run. Each activates a single visible client panel and hides the HTMX feed panel.
- `frontend/platform/static/js/community-dms.js` now builds profile links with the registered `/community/u/:user_id` route. Recheck the clickable link during the next DM mutation pass with a disposable thread.
- `frontend/platform/partials/community_post_list.html` and `frontend/platform/partials/community_feed.html` link to `/community?tab=members`; the members client panel is now implemented and covered by `browser-run-2026-05-29.md`.

When adding new Community routes:
1. Add or update the specific workflow that owns the route.
2. Update `README.md` surface map, index, pass order, or coverage table if the route introduces a new surface.
3. Run the validation commands above.
4. If the route is intentionally shared with settings/account/admin outside `/community`, mention the owning non-Community route in the relevant Community workflow.
