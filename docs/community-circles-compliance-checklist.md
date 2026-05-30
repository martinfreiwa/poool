# POOOL Community Circles Legal And Compliance Checklist

Date: 2026-05-21  
Scope: Production sign-off for investment-community language, roles, content types, and moderation.

## Sign-Off Requirement

POOOL Circles must not be declared production-live until legal/compliance signs off the items below. This checklist is an operational control; it is not legal advice.

## Content Disclaimers

- Market Insight posts show `User opinion, not financial advice`.
- Due Diligence posts are framed as questions or research, not recommendations.
- Risk Discussion posts avoid positive performance framing.
- Official Updates are visually distinct from user opinions.
- Property Updates distinguish official platform/operator information from member commentary.
- Circle About/Rules text includes an investment-risk disclaimer for asset and private investor Circles.

## Role And Badge Language

- `Verified Investor` means verification/participation status, not skill or performance.
- `Asset Holder` means current platform-recorded access/holding status, not endorsement.
- `Verified Expert` is scoped to expertise/AMA participation and does not imply individualized financial advice.
- `Official POOOL` is reserved for platform/admin/system-granted identity only.
- Reputation badges must not imply guaranteed returns, superior performance, or recommendation authority.

## Moderation And User Safety

- Investment-risk keywords are configured for asset and private investor Circles.
- Report reasons are allowlisted and reviewed by authorized roles only.
- First-post approval is considered for private investor and holder-only Circles.
- Moderators can hide/dismiss scoped reports but cannot perform platform-wide bans unless they also have platform-admin permissions.
- Moderator actions are audit logged.
- Escalation paths exist for legal, tax, liquidity, performance-claim, and misleading-return reports.

## Privacy And Access Control

- Private Circle mentions render as `Private Circle` for non-members.
- Hidden Circle mentions do not expose name, slug, or ID to non-members.
- Holder-only and token-gated access is server-validated.
- Resource links do not expose raw storage paths.
- Private resources use authenticated delivery.
- Documents with legal hold are not physically deleted by cleanup workers.

## Official And Expert Content

- Only authorized roles can publish Official Update or Announcement posts.
- Official Answers are restricted to moderators, verified experts, or platform admins.
- Expert answers include context that they are general educational content unless legally approved otherwise.
- AMA content has a moderation path before being promoted as official knowledge.

## Launch Decision Record

Before launch, record:

- reviewer name and role;
- review date;
- approved disclaimer text;
- approved badge definitions;
- approved moderation policy;
- unresolved exceptions;
- launch decision: approved, approved with conditions, or blocked.

## Blockers

Production launch is blocked if any of these remain unresolved:

- Badge wording can be read as performance advice.
- Official/user content is visually ambiguous.
- Private or hidden Circle names leak to unauthorized users.
- Holder-only resources can be accessed without current authorization.
- Investment-performance claims have no moderation path.
- Provider alerting has not passed the staging receipt check.
