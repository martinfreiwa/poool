# POOOL Workflows

This document stores repeatable local workflows for agents and developers working on POOOL.

Keep workflows practical and executable. Do not store passwords, API keys, session cookies, or other secrets in this file.

## Workflow Template

Use this structure for new workflows:

```md
## <Workflow Name>

Purpose: <What this workflow achieves>

Prerequisites:
- <Required local service, account, or state>

Steps:
1. <Step one>
2. <Step two>
3. <Step three>

Expected Result:
- <How to know the workflow succeeded>

Notes:
- <Important edge cases or follow-up checks>
```

## Workflow Areas

- Community workflows live in [`docs/workflows/community/README.md`](./workflows/community/README.md).
- Keep Community workflows separate from account/settings, marketplace, developer, and deployment workflows.

## Local Browser Login

Purpose: Log into the local POOOL platform through the in-app browser.

Prerequisites:
- Local backend is running on `http://localhost:8888`.
- PostgreSQL is running and the local `poool` database is reachable.
- A valid user account exists. Example account used in this session: `support@traffic-creator.com`.
- Password is available from the user's secure source. Do not write it into this file.

Steps:
1. Open `http://localhost:8888/` in the in-app browser.
2. Click `Sign in` in the page header.
3. On `/auth/login`, fill the `Email` field with the target account email.
4. Fill the `Password` field with the account password.
5. Click `Log in`.
6. Wait for the redirect to complete.

Expected Result:
- Browser lands on `http://localhost:8888/marketplace`.
- The sidebar or account menu shows the logged-in account email.

Notes:
- If login stalls or fails before the form submits, verify that the backend process is still running.
- If the backend cannot start, verify local PostgreSQL first with `pg_isready -h localhost -p 5432 -d poool`.
- If PostgreSQL reports a stale `postmaster.pid`, confirm no real Postgres process is using the data directory before removing the stale lock file.

### Coverage Matrix

Run these variants when validating login coverage:

| Case | Steps | Expected Result |
|------|-------|-----------------|
| Valid login | Log in with a known active account. | Redirects to `/marketplace`; account menu shows the expected email. |
| Invalid password | Submit a known email with a wrong password. | Login stays on `/auth/login`; a generic authentication error is shown; no session is created. |
| Unknown email | Submit an email that does not exist. | Login stays on `/auth/login`; response does not reveal whether the email exists. |
| Empty fields | Submit without email and/or password. | Browser or server validation blocks submit. |
| Protected redirect | Open `/settings` while logged out. | User is redirected to `/auth/login`. |
| Session reuse | Open `/settings` after successful login. | Settings page loads without asking for credentials again. |
| Logout then protected page | Log out, then open `/settings`. | User is redirected to `/auth/login`. |
| Remember me off | Log in without `Remember me`. | Session cookie is short-lived/default session duration. |
| Remember me on | Log in with `Remember me`. | Session cookie uses the longer configured duration. |
| Rate limit | Repeat failed logins enough times to trigger auth rate limiting. | Login returns a rate-limit message without panics or account leakage. |

## Profile Settings Persistence Check

Purpose: Verify that editable settings on the local profile/settings page can be changed, saved, reloaded, and read back correctly.

Prerequisites:
- Local backend is running on `http://localhost:8888`.
- PostgreSQL is running and the local `poool` database is reachable.
- A valid test account exists. Example account used in this session: `support@traffic-creator.com`.
- Password is available from the user's secure source. Do not write it into this file.
- Use test-safe values that can be reverted. Avoid changing real legal, KYC, payout, 2FA, wallet, password, account deletion, or production-sensitive data unless explicitly requested.
- A small valid test image is available for upload testing, for example a local `.webp`, `.png`, or `.jpg` under 2 MB.
- To fully test Developer Identity, Developer Logo, and Developer Links, use an account with developer access. On a non-developer account, those actions should be blocked with a clear authorization error.

Steps:
1. Open `http://localhost:8888/` in the in-app browser.
2. Log in through `/auth/login` with the target test account.
3. Confirm the login succeeded by checking that the browser lands on `/marketplace` and the account menu shows the expected email.
4. Navigate to `http://localhost:8888/settings`.
5. Verify the Settings page shell and navigation inventory:
   - Page title is `Account Settings`.
   - Search input is present.
   - Settings side navigation includes `Core Profile`, `Address`, `Identity`, `Security`, `Web3 Wallet`, `Preferences`, `Leaderboard`, `Social`, `Developer Identity`, `Developer Links`, and `Data & Privacy`.
   - Every side-nav entry scrolls or jumps to the intended section without breaking the layout.
6. In `Core Profile`, change each editable safe field:
   - `First Name`
   - `Middle Name`
   - `Last Name`
   - `Gender`
   - `Phone Number`
7. Upload a profile photo through `Change Photo` using the test image.
8. Click `Save Profile` if the UI requires a separate profile save, and wait for the save request or success state to finish.
9. In `Residential Address`, change each editable field:
   - `Address Line 1`
   - `Address Line 2`
   - `City`
   - `State / Region`
   - `Postal / ZIP Code`
   - `Country`
10. Click `Save Address` and wait for the save request or success state to finish.
11. In `Identity Vault`, only change fields that are safe in the current test context:
   - `Date of Birth`
   - `Nationality`
   - `Tax ID`
12. Click `Save Identity Details` and wait for the save request or success state to finish.
13. In `Security & Access`, verify the section shell and current account state without changing sensitive settings:
   - Account email is shown.
   - Email verification state is shown.
   - Password row and `Change Password` action are present.
   - Two-factor auth state and action are present.
   - Linked account state and action are present.
   - Do not submit password, 2FA, or OAuth changes unless explicitly requested.
14. In `Web3 Wallet`, verify the section shell without connecting a wallet:
   - Section title and explanatory copy are visible.
   - Wallet connect/rebind/copy controls render according to current wallet state.
   - Do not connect, rebind, or sign wallet messages unless explicitly requested.
15. In `Preferences`, change:
   - `Language`
   - `Timezone`
   - `Currency`
   - `Email Alerts`
   - `Push Notifications`
16. Click `Update Preferences` and wait for the save request or success state to finish.
17. In `Leaderboard`, change:
   - `Show on Leaderboard`
   - `Show Avatar`
   - `Display Name`
   - `Community Bio`
18. Click `Save Leaderboard Settings` and wait for the save request or success state to finish.
19. In `Social Profiles`, change:
   - `Twitter / X`
   - `LinkedIn`
   - `Instagram`
   - `Telegram`
   - `Discord`
   - `Website`
20. Click `Save Social Links` and wait for the save request or success state to finish.
21. Verify `Info & Learning` links:
   - `Rate us`
   - `Feedback`
   - `Refer`
   - `Glossary`
   - Each link has the expected destination and does not overlap or break the settings layout.
22. Verify `Learn with POOOL` content:
   - `See All` link is present.
   - Learning cards render with image, category, title, and valid destination.
   - Cards do not overlap the settings form on desktop or mobile viewports.
23. In the `Developer Identity` card, verify the section shell before editing:
   - Card title is `Developer Identity`.
   - Supporting copy explains that company branding and public information are visible to investors.
   - `Developer Logo`, `Company Name`, `Developer Description`, and `Save Developer Profile` are present.
   - No removed preview-only UI should be required for saving.
24. In `Developer Identity`, test developer-only fields:
   - Upload `Developer Logo` using the test image.
   - Change `Company Name`.
   - Change `Developer Description`.
   - Click `Save Developer Profile`.
25. In the `Developer Links` card, verify the section shell before editing:
   - Card title is `Developer Links`.
   - Supporting copy explains that public links are shown on the developer page and asset listings.
   - Website, GitHub, Twitter / X, LinkedIn, and YouTube fields are present.
   - `Save Developer Links` is present.
26. In `Developer Links`, change:
   - `Website`
   - `GitHub`
   - `Twitter / X`
   - `LinkedIn`
   - `YouTube`
27. Click `Save Developer Links` and wait for the save request or success state to finish.
28. If the account is not a developer account, verify that Developer Logo, Developer Identity, and Developer Links are rejected with a clear message such as `Developer settings are only available to developer accounts.`
29. In `Data & Privacy`, verify both data-rights actions:
   - `Download export` is present and usable.
   - `Delete account...` is present but not executed unless explicitly requested.
30. Click `Download export`.
31. Verify that the export downloads successfully and is valid JSON.
32. Verify the export includes expected top-level account data such as profile, settings, wallets, transactions, KYC records, and investments, as available for the test account.
33. Reload `/settings`.
34. Re-open each changed section and compare every field, dropdown, switch, uploaded image URL, link section, security state, Web3 state, data-rights action, and developer field with the test values or expected read-only state.
35. Navigate away from settings, for example to `/marketplace`, then return to `/settings` and verify the same values and section shells again.

Expected Result:
- Every saved field keeps the exact value after reload.
- Every saved dropdown keeps the selected option after reload.
- Every saved switch keeps its checked or unchecked state after reload.
- Uploaded profile photo is shown after reload.
- Uploaded developer logo is shown after reload when the account has developer access.
- Developer Identity and Developer Links persist for developer accounts.
- Developer-only actions are blocked with a clear authorization message for non-developer accounts.
- Security, Web3 Wallet, Info & Learning, Learn with POOOL, and Data & Privacy sections render correctly and remain stable after navigation/reload.
- Data export downloads successfully and parses as JSON.
- Account menu display updates if name-related fields are expected to affect it.
- No save operation silently fails, resets fields, or shows stale pre-save values.

Notes:
- Do not test `Change Password`, `Enable 2FA`, `Connect`, `Connect Wallet`, or `Delete account...` as part of this persistence workflow unless the user explicitly requests those security-sensitive or destructive actions.
- `Download export` is included because it is a read/export workflow, but treat the downloaded JSON as sensitive personal data and do not commit it.
- Use clearly recognizable test values, for example suffixes like `Persistence Test`, so changed values are easy to compare and later revert.
- If a save button stays disabled, confirm the field values are actually different from the current saved state.
- If a section shows a success toast but values disappear after reload, treat it as a persistence bug and capture the section name, exact field, before value, after value, and browser console/network errors.
- If changing `Identity Vault` fields would affect legal or KYC state for the chosen account, skip those fields and document them as intentionally not tested.
- If upload testing fails, capture the file type, file size, endpoint response, and whether the failure came from MIME validation, file size validation, storage, or authorization.

### Coverage Matrix

Run these variants when validating profile-settings coverage:

| Area | Case | Expected Result |
|------|------|-----------------|
| Settings shell | Verify page title, search, and every settings side-nav entry. | All sections are discoverable and navigation does not break layout. |
| Persistence | Save each section, reload `/settings`, then navigate away and back. | All fields, dropdowns, switches, and images retain saved values. |
| API parity | Compare UI values with `GET /api/settings`. | UI and API agree for every saved field. |
| Database parity | Query the related profile/settings rows after save. | Database rows match the saved values. |
| Cleanup | Restore previous values after test completion when needed. | Test account is left in a known state. |
| Avatar upload | Upload valid `.webp`, `.png`, and `.jpg` files under 5 MB. | Upload succeeds and avatar renders after reload. |
| Avatar validation | Upload SVG, wrong MIME, and oversized files. | Upload is rejected with a clear validation error. |
| Developer logo | Upload a valid logo under 2 MB using a developer account. | Logo persists and renders after reload. |
| Developer identity card | Verify the full `Developer Identity` card shell, including title, subtitle, logo upload area, company field, description field, and save action. | Card is visible for eligible accounts, has no obsolete preview dependency, and exposes the expected edit controls. |
| Developer authorization | Try Developer Logo/Profile/Links with a non-developer account. | Actions are rejected with a clear authorization message. |
| Developer identity | Save `Company Name` and `Developer Description` with a developer account. | Values persist after reload and API readback. |
| Developer links card | Verify the full `Developer Links` card shell, including title, subtitle, Website, GitHub, Twitter/X, LinkedIn, YouTube, and save action. | Card is visible for eligible accounts and exposes the expected edit controls. |
| Developer links | Save Website, GitHub, Twitter/X, LinkedIn, and YouTube with a developer account. | Values persist after reload and API readback. |
| Social validation | Save invalid URLs in social URL fields. | Invalid values are blocked client-side or rejected server-side. |
| Length validation | Save too-long bio and developer description. | UI/server enforces max length and preserves stable layout. |
| Preferences | Toggle notifications on and off across runs. | Switch state persists after reload and API readback. |
| Security | Verify email, password, 2FA, and linked-account rows without submitting sensitive changes. | Security state is visible and actions are present, but no sensitive mutation runs by default. |
| Web3 Wallet | Verify wallet section and controls without connecting or signing. | Wallet state is visible and connect/rebind/copy controls match the account state. |
| Info & Learning | Verify Rate us, Feedback, Refer, Glossary, Learn with POOOL cards, and See All. | Links/cards render with valid destinations and stable layout. |
| Data export | Download export and parse JSON. | Download succeeds; JSON includes expected top-level keys and no raw secrets. |
| Account deletion | Verify `Delete account...` exists but do not execute it. | Destructive action is discoverable but excluded from default workflow execution. |
| Cross-session | Save values, log out, log back in, then re-open `/settings`. | Values still persist across sessions. |
