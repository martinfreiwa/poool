---
description: Systematic codebase defect-discovery and remediation protocol. Multi-phase parallel-agent methodology with formal severity rubric, verification controls, and bounded remediation. Pause checkpoints between phases.
argument-hint: "[optional scope predicate, e.g. 'backend only', 'auth + payments', 'frontend XSS only']"
methodology-version: "2.0"
methodology-date: "2026-05-19"
---

# Codebase Defect-Discovery and Remediation Protocol (CDDRP)

## Abstract

This document specifies a five-phase, multi-agent protocol for discovering, verifying, classifying, and remediating defects in a software codebase. The protocol prioritises **signal-to-noise**, **false-positive control**, **separation of discovery from remediation**, and **bounded user-checkpointed risk**. It is designed to be executed by the Claude Code agent harness with up to ~80 parallel sub-agents per run, partitioning work across three orthogonal axes: domain band, scan dimension, and tooling. The protocol terminates in a remediation phase that applies only fixes meeting strict safety preconditions (verified finding, no WIP file conflict, no design ambiguity), with all other findings escalated to the user with reproduction evidence and recommended action.

**Inputs.** A git working tree; an optional scope predicate (`$ARGUMENTS`); access to the existing audit corpus at `docs/security-audits/` for deduplication.

**Outputs.** (1) A consolidated audit report in `docs/security-audits/YYYY-MM-DD-cddrp-run-N.md` containing every finding with file:line, severity, evidence, and verification verdict; (2) a remediation log of fixes landed in the working tree; (3) a residual-risk register of findings deferred pending user decision; (4) build-verification status.

**Non-goals.** This protocol does not commit changes, push to remote, or modify CI/CD. The user retains final commit and merge authority.

---

## 1. Objective and Scope

### 1.1 Primary objective

Identify and remediate the maximum number of high-impact defects per unit of wall-clock time, subject to a hard false-positive-tolerance constraint (no fix lands without explicit verification of the underlying defect on current `HEAD`) and a hard safety constraint (no fix lands in a file with uncommitted modifications, except by explicit user override).

### 1.2 Secondary objectives

- **Coverage measurement.** Every file under the scope predicate is examined by at least one Phase-1 discovery agent.
- **Verifiability.** Every reported finding is reproducible from the audit report alone, without re-reading agent transcripts.
- **Compositionality.** Findings that share a fix shape are grouped into *remediation bundles* (§2.4) and addressed once.
- **Continuity.** The output report is consumable by a future invocation of this protocol for delta-discovery and dedup.

### 1.3 Scope predicate

If `$ARGUMENTS` is non-empty, restrict Phase-1 domain bands to those matching the predicate (substring or boolean expression on band labels). Otherwise audit all bands.

---

## 2. Definitions and Taxonomy

### 2.1 Finding lifecycle

A *finding* is a (file, line-range, defect-class, evidence) tuple. It progresses through five states:

| State | Definition | Transition trigger |
|---|---|---|
| **DISCOVERED** | Initially surfaced by a Phase-1 agent. | Phase 1 completion. |
| **CONFIRMED** | Reproduced on current `HEAD` by a Phase-2 verifier; defect exists and is exploitable/observable. | Phase 2 completion. |
| **FALSE_POSITIVE** | Verifier could not reproduce; the code does not exhibit the claimed defect (e.g. agent misread, pre-mitigation, framework guarantee). | Phase 2 completion. |
| **STALE** | Was real at audit time but already fixed in HEAD before remediation. | Phase 2 completion. |
| **DOWNGRADED** | Real but its impact is bounded by an independent control (DB CHECK constraint, framework middleware, OS-level guard). Severity reduced by one tier. | Phase 2 completion. |
| **NEEDS_REPRO** | Plausible but requires an executable proof-of-concept the verifier could not construct read-only. | Phase 2 completion. |
| **REMEDIATED** | Fix landed in working tree; build passes. | Phase 3 completion. |
| **DEFERRED** | CONFIRMED but excluded from remediation by §4.6.1 criteria. | Phase 3 completion. |

### 2.2 Severity rubric

Severity is assigned by the discovery agent and reaffirmed (or revised) by the verifier. Five tiers, modelled on CVSS v3.1 qualitative bands:

| Tier | CVSS-equivalent | Operational definition |
|---|---|---|
| **Critical** | 9.0–10.0 | Pre-auth or low-priv exploit yielding unauthorised state mutation, secret disclosure, RCE, complete authn/authz bypass, fund loss, or persistent stored XSS with platform-wide reach. |
| **High** | 7.0–8.9 | Authn-required exploit yielding privilege escalation, unbounded resource consumption (DoS), data leak across tenants, race condition on financial state, supply-chain risk (floating unsigned dependency), or defense-in-depth failure adjacent to a Critical finding. |
| **Medium** | 4.0–6.9 | Bounded information leak, dead code in security-sensitive paths, defensive guard missing (other controls present), exploit requires implausible preconditions, single-tenant data integrity issue. |
| **Low** | 0.1–3.9 | Severe smell with no demonstrable exploit, real TODO/FIXME, dead-code attribute on live function, missing `rel="noopener"`, byte-level off-by-one with no security impact. |
| **Informational** | 0.0 | Style, naming, documentation. *Not in scope of this protocol.* |

Discovery agents are instructed to err toward higher severity; verifiers apply the downgrade criteria in §4.4.3.

### 2.3 Verdict taxonomy

Verification outputs one of six verdicts per finding:

| Verdict | Meaning | Effect on lifecycle |
|---|---|---|
| `CONFIRMED` | Defect present on HEAD; exploitable/observable as described. | → CONFIRMED |
| `FALSE_POSITIVE` | Defect not present, or claim materially misrepresents the code. | → FALSE_POSITIVE; reason logged. |
| `STALE` | Defect was present at audit time but already fixed. | → STALE; cite the fixing change if locatable. |
| `DOWNGRADED` | Defect present but impact bounded by independent control. | → DOWNGRADED; new severity recorded. |
| `NEEDS_REPRO` | Plausible defect but read-only verification insufficient. | → NEEDS_REPRO; PoC plan recorded. |
| `PARTIAL` | A subset of the cited claims hold; rest are false-positive or stale. | Split into per-claim verdicts. |

### 2.4 Remediation bundle

A *bundle* is a non-empty set of findings sharing a single fix shape (substitution pattern, helper extraction, common middleware addition). Bundles are catalogued in Phase 2 (§4.4.2) and addressed in Phase 3 with prescriptive sub-agent prompts. Bundle membership is one-to-one: a finding belongs to at most one bundle; ungrouped findings are remediated individually.

### 2.5 Defect classes (taxonomy used by discovery prompts)

Aligned with CWE top-25 and OWASP top-10:

- **CWE-89** SQL injection / dynamic query construction
- **CWE-79** XSS (stored, reflected, DOM)
- **CWE-918** SSRF
- **CWE-352** CSRF
- **CWE-22** Path traversal
- **CWE-434** Unrestricted file upload
- **CWE-862 / CWE-863** Missing or incorrect authorization
- **CWE-287** Improper authentication (incl. bypass)
- **CWE-190** Integer overflow / wraparound
- **CWE-362** Race condition / TOCTOU
- **CWE-209 / CWE-532** Information exposure through error messages / log files
- **CWE-639** Authorization bypass through user-controlled key (IDOR)
- **CWE-770** Allocation of resources without limits (DoS)
- **CWE-601** Open redirect
- **CWE-94** Code injection (eval, deserialisation)
- **CWE-200** Information disclosure (PII in logs, internal-error reflection)
- **CWE-798** Hardcoded credentials
- **CWE-307** Improper rate-limiting

---

## 3. Threat Model (Scope Boundary)

The protocol assumes a hostile actor with one of three privilege tiers:

1. **Unauthenticated external** — may reach any internet-exposed route; may craft arbitrary request bodies; cannot present a valid session cookie.
2. **Authenticated low-privilege** — holds a valid user session; may also have a "developer" or "affiliate" role; cannot present an admin role.
3. **Authenticated admin (any role)** — holds a valid admin session under one of the project's admin roles, but not necessarily the most-privileged role (`super_admin`) nor every fine-grained permission.

The protocol explicitly **excludes** the following from scope:
- Compromise of the host machine, kernel, or container runtime.
- Compromise of a third-party dependency's signing keys (modelled separately under supply-chain risk).
- Insider threat with database-shell access.
- Social engineering against operators.

Findings outside these privilege tiers (e.g. "if an attacker has root on the server, X") are reported as **Informational** and not in remediation scope.

---

## 4. Methodology

### 4.1 Phase 0 — Preconditions and instrumentation

**Inputs:** the working tree at session start.
**Outputs:** a precondition manifest used by subsequent phases.

#### 4.1.1 Working-tree characterisation

Execute, in parallel:

```bash
git status --short
git diff --stat HEAD
find <backend-root> -name '*.<lang-ext>' | wc -l
find <frontend-root> -type f \( -name '*.html' -o -name '*.js' -o -name '*.ts' -o -name '*.css' \) | wc -l
ls docs/security-audits/ 2>/dev/null
```

Record:
- **WIP-set W**: files with `M` or `D` in `git status`. These are off-limits to Phase 3 remediation (§4.6.1).
- **Repo size S** (file counts) for capacity planning.
- **Prior-audit set P** (paths under `docs/security-audits/`) for Phase 1 deduplication.

#### 4.1.2 Project-specific configuration

Read `~/.claude/projects/<project-id>/memory/` for:
- WIP-handling preferences (e.g. `feedback_wip_entanglement.md`).
- Build / lint / test commands and environment variables.
- Prior remediation history (`reference_*.md`).

If the user must be queried during the run, use the Spokenly MCP per the user's global instructions; never query via plain text.

### 4.2 Phase 1 — Discovery (parallel multi-agent audit)

**Inputs:** the working tree; the scope predicate; the prior-audit set P.
**Outputs:** a consolidated audit report at `docs/security-audits/YYYY-MM-DD-cddrp-run-N.md` (N = sequential run number), containing all DISCOVERED findings.

#### 4.2.1 Coverage matrix

Discovery agents are launched in a three-axis coverage matrix:

- **Axis A (Domain)** — ~25 cells partitioning the file tree by module / page-group. Each cell is assigned to exactly one agent; cells are non-overlapping and exhaustive over the scope.
- **Axis B (Scan-dimension)** — 8 cells, each scanning the entire codebase for one defect class (§2.5). Cells are orthogonal to Axis A.
- **Axis C (Tooling)** — 5–8 cells, each running one external tool or grep-pattern and parsing its output.

Default cardinalities are specified in *Appendix A*. The total agent count for Phase 1 is the sum: |A| + |B| + |C|, typically 38–50 agents.

#### 4.2.2 Per-agent specification

Every Phase-1 agent receives a prompt with the following invariant structure:

```
ROLE:        general-purpose read-only auditor
SCOPE:       <cell scope>
DEDUP-SET:   docs/security-audits/*.md  (do not re-flag entries here)
SEVERITY:    apply rubric §2.2; err toward higher tier
DEFECT-CLASSES: §2.5 (filter to those relevant to the cell)
OUT-OF-SCOPE: style, naming, docs, formatting, missing tests
              (unless test absence is the defect on a Critical path)

OUTPUT SCHEMA (strict markdown table):

| Severity | File:Line | CWE / Class | Issue | Why it matters |
|---|---|---|---|---|

Order rows by descending severity. Cap at 40 rows. Note empty severity
tiers explicitly. End with:

  TOP-3-FIRST: <file:line> — <one-line action> (×3)
  COVERAGE-NOTES: <files or sub-paths examined and found clean>

PROHIBITED: edits, commits, network writes, shell mutations.
```

The agent is invoked with `run_in_background: true`.

#### 4.2.3 Consolidation procedure

When all Phase-1 agents complete:

1. Merge per-agent tables into a single severity-sorted master table.
2. Dedupe by `(file, line, defect-class)` key (within ±5 lines tolerance).
3. Write the consolidated report to `docs/security-audits/YYYY-MM-DD-cddrp-run-N.md` with the structure specified in *Appendix B*.

### 4.3 Checkpoint α — User authorisation to verify

Present the user with:
- Aggregate counts: Critical / High / Medium / Low.
- Top-10 fix-first list.
- Estimated Phase-2 duration (∝ |Critical ∪ High|).

Query via Spokenly: *"Run verification pass? (recommended; expected false-positive rate is 10–20%)"*

Options: `yes` (→ §4.4), `skip` (→ §4.6 with `verified=False` on all findings; remediation restricted to `safe-only` mode), `stop` (→ §4.8).

### 4.4 Phase 2 — Verification (parallel multi-agent re-examination)

**Inputs:** the consolidated audit report.
**Outputs:** updated report with verdicts (§2.3) on every CONFIRMED-tier candidate, plus a bundle catalogue (§2.4).

#### 4.4.1 Verifier coverage

Three sub-armies launched in parallel:

- **2A — Per-band verifiers.** One verifier per Phase-1 domain cell, re-checking all Crit + High findings in that cell.
- **2B — Bundle hunters.** One agent per pattern in *Appendix C*, performing exhaustive codebase grep for that pattern, returning a complete catalogue (not a sample).
- **2C — End-to-end specialists.** Cross-cutting flow auditors (money flow, auth flow, upload flow, admin RBAC, cross-DB consistency). One per flow.

Total Phase-2 agent count: ~25 (2A) + ~8 (2B) + ~5 (2C) = ~38 agents.

#### 4.4.2 Per-verifier specification

```
ROLE:    general-purpose read-only verifier
INPUT:   the consolidated audit report at <path>
SUBSET:  findings in cell <X> with Severity in {Critical, High}

PROCEDURE per finding:
  1. Read the cited file at the cited line ±20 lines.
  2. Determine whether the cited code matches the described pattern.
  3. Determine whether the impact is exploitable on HEAD given §3 threat model.
  4. Apply §4.4.3 downgrade heuristics.
  5. Emit one of {CONFIRMED, FALSE_POSITIVE, STALE, DOWNGRADED, NEEDS_REPRO,
     PARTIAL} with one-line evidence (current code excerpt or guard reference).

OUTPUT SCHEMA:

| Finding (file:line) | Verdict | New severity (if changed) | Evidence |
|---|---|---|---|

Conclude with summary line:
  CONFIRMED: N · FALSE_POSITIVE: M · STALE: K · DOWNGRADED: D · NEEDS_REPRO: P
```

#### 4.4.3 Downgrade heuristics (applied during verification)

A finding is **DOWNGRADED** by one severity tier when one of the following independent controls bounds its impact:

| Heuristic | Example |
|---|---|
| **DB integrity constraint** catches the failure mode | "`UPDATE wallets SET balance = balance - $1` lacks `AND balance >= $1`" → DB CHECK `balance >= 0` converts silent corruption to loud error. |
| **In-transaction status guard** prevents the race | "Cancel-withdraw double-refund" → `SELECT … FOR UPDATE` + status check inside same tx prevents replay. |
| **Single-task worker self-serialises** | "Settlement worker has no concurrency cap" → sequential `loop { sleep }` (not `tokio::time::interval`) cannot interleave. |
| **Framework / middleware bounds blast radius** | "Handler lacks `require_permission`" → router applies auth middleware before reaching handler; downgrade if middleware enforces the same gate. |
| **Server-side pre-sanitisation** defeats the claimed sink | "Template uses `\| safe`" → display-data builder runs `escape_html()` before template render; sink is safe. |
| **OS / kernel guard** | "`unsafe` block reads file" → file permissions enforce the access boundary in production. |

A finding is **FALSE_POSITIVE** when the cited code does not exhibit the claimed defect at all (agent misread, claim refers to wrong line, claim describes a sibling function).

#### 4.4.4 Bundle catalogue specification (sub-army 2B)

Each bundle-hunter agent returns a *complete* enumeration of one pattern:

```
PATTERN:    <name from Appendix C>
INVENTORY:  table of every occurrence (file:line) with one-line context
FIX SHAPE:  the unified substitution / extraction that closes all occurrences
COMPLEXITY: trivial | moderate | large
PR SCOPE:   single PR | split into PR1/PR2/PR3 with criteria
```

#### 4.4.5 Specialist flow audit (sub-army 2C)

Each specialist agent traces one end-to-end flow and reports inter-step defects:

```
FLOW:        e.g. "deposit → wallet credit → cart → checkout → order → settlement → dividend payout"
STEPS:       enumerated, each annotated with the file:line that implements it
ISSUES:      transaction-boundary gaps, missing locks, idempotency gaps, missing audit log
RECOMMENDATIONS: ordered by step
```

#### 4.4.6 Verification consolidation

Update the audit report (in-place) with:

- **Aggregate verification table**: counts of each verdict.
- **FALSE_POSITIVE registry**: full list with reason.
- **DOWNGRADED registry**: old → new severity.
- **NEEDS_REPRO registry**: with proposed PoC step.
- **Bundle catalogue**: per *Appendix C* schema.
- **Specialist flow report**: per §4.4.5.

### 4.5 Checkpoint β — User authorisation to remediate

Present the user with:
- Verification deltas (confirmed vs false-positive vs downgraded).
- Top-ranked remediation bundles by impact × low-complexity.
- Estimated Phase-3 surface (file count, line count).

Query via Spokenly: *"Proceed to remediation? Select scope."*

Options:
- `safe-only` — only findings meeting all §4.6.1 preconditions.
- `safe + bundles` — also extract the trivial-complexity bundles.
- `safe + bundles + critical singletons` — also tackle hand-selected confirmed Criticals not in any bundle.
- `defer` — produce report only; remediation deferred.

Default selection if user is ambiguous: `safe + bundles`.

### 4.6 Phase 3 — Remediation

**Inputs:** the verified audit report; the selected scope from §4.5.
**Outputs:** edits to the working tree; a remediation log; a deferred-finding registry.

#### 4.6.1 Remediation preconditions

A finding F is eligible for automated remediation iff **all** the following hold:

1. **F.state = CONFIRMED** (or DOWNGRADED with severity ≥ Low).
2. **F.file ∉ W** (working-tree WIP set from §4.1.1). If user explicitly overrides, this constraint is lifted.
3. **F.fix-shape is determinate** — no business decision required (e.g. choice of permission slug from > 1 plausible candidate, choice of retention horizon, choice of rate-limit value).
4. **F.fix-surface ≤ 1 file** for solo remediation, or **F ∈ Bundle B with B.complexity = trivial** for parallel mechanical remediation.
5. **F.fix does not require restoration of intentionally-disabled functionality** (e.g. comment says "temporarily disabled"; user must approve restoration).

Findings failing any of (1)–(5) are recorded in the deferred-finding registry with the failing precondition cited.

#### 4.6.2 Per-fix procedure (solo remediation)

For each eligible solo-remediation finding:

1. Read the cited file ±30 lines.
2. Confirm the defect still matches the verifier's evidence (audit can be stale by Phase 3).
3. Identify the minimal change that closes the defect.
4. Edit conservatively:
   - Preserve existing comment style.
   - Match existing error-handling idiom in the same file / module.
   - No new abstractions for hypothetical future cases.
   - No backwards-compatibility shims unless an active caller requires them.
5. If the finding turns out to be a false-positive on closer reading (Phase 2 verifier missed something), record as FALSE_POSITIVE and skip.

#### 4.6.3 Parallel mechanical remediation (bundle execution)

Bundles with `complexity = trivial` may be remediated by parallel sub-agents. Each sub-agent receives a prompt with:

- The exact bundle inventory (file:line list).
- The exact substitution pattern or helper extraction signature.
- A list of file paths explicitly off-limits (W ∪ user-specified exclusions).
- Instructions to run `<lint/build command>` on completion and report status.

Up to 8 bundle sub-agents may run concurrently if their file sets are disjoint. If two bundles overlap on a file, they are serialised.

#### 4.6.4 Non-automated remediation (escalation)

Findings requiring design decisions, business input, or restoration of intentionally-disabled functionality are formatted as escalation items:

```
ESCALATION: <finding-id>
SEVERITY:   <tier>
LOCATION:   <file:line>
DESCRIPTION: <one-paragraph defect description>
BLOCKERS:   <which §4.6.1 precondition fails>
PROPOSED OPTIONS: <2-3 alternatives, each with trade-off>
RECOMMENDED: <Claude's recommendation with rationale>
USER DECISION REQUIRED: yes
```

These are presented to the user at Phase 5.

### 4.7 Phase 4 — Build verification

**Inputs:** the modified working tree.
**Outputs:** build status per language/runtime.

For each language present:

| Language | Verification command | Notes |
|---|---|---|
| Rust | `cargo check --quiet` against live DB (`DATABASE_URL=...`); `cargo clippy --quiet` for warnings | If `sqlx::query!` macros were added, `SQLX_OFFLINE=true` will fail until `cargo sqlx prepare` runs; note as commit prerequisite. |
| TypeScript / JavaScript with build | `npm run typecheck` or equivalent | Skip if no build step (raw HTML/JS). |
| Go | `go build ./...` | |
| Python | `mypy <pkg>` if configured, else `python -m py_compile` per touched file | |

Test suites are **not** run by default (long; may have unrelated WIP failures). Note as user follow-up.

Build status is reported as:
- **CLEAN** — no errors, no new warnings on touched files.
- **CLEAN with pre-existing errors** — failures localised to files not modified in this run.
- **REGRESSION** — failures introduced by this run; remediation rolled back or escalated.

### 4.8 Phase 5 — Reporting

**Inputs:** all artefacts from prior phases.
**Outputs:** the final session report to the user.

The report contains:

1. **Aggregate metrics**:
   - Findings by severity (counts and percentages).
   - Verification verdict distribution.
   - False-positive rate (target: ≤ 20%; flag if exceeded for prompt-quality review).
   - Remediation: count remediated, count deferred, by severity.

2. **Touched-files manifest** with one-line per-file rationale.

3. **Audit-report path** (§4.2.3).

4. **Build status** (§4.7).

5. **Deferred-finding registry** with reason per finding.

6. **Next-action menu** with prioritised options.

---

## 5. Quality Controls

### 5.1 Coverage measurement

Every file under the scope predicate must be claimed by exactly one Phase-1 Axis-A agent and additionally scanned by all applicable Axis-B and Axis-C agents. Coverage is measured ex-post by:

```
expected_files = set(scope_predicate)
covered_files  = union(agent.coverage_notes for agent in phase_1_agents)
coverage_pct   = |covered_files ∩ expected_files| / |expected_files|
```

A coverage_pct below 0.90 triggers a follow-up partition-repair pass.

### 5.2 Inter-rater agreement

When two or more discovery agents independently flag the same `(file, line ±5)` location, agreement is recorded. Findings with ≥ 2 independent flaggers are upgraded one severity tier (subject to Phase-2 downgrade heuristics). Findings flagged by only one agent retain their original severity.

### 5.3 False-positive rate

After Phase 2, compute:

```
fp_rate = |FALSE_POSITIVE| / (|CONFIRMED| + |FALSE_POSITIVE| + |DOWNGRADED|)
```

Target: ≤ 20%. If exceeded, the Phase-1 prompt template (§4.2.2) is suspected of over-aggression and should be revised before the next run.

### 5.4 Reproducibility

Every finding's evidence field (§2.3) must be sufficient for a reader to reproduce the determination without re-running the agents. Concretely: an evidence string must cite the current code excerpt or the controlling middleware/guard/constraint.

### 5.5 Run versioning

Each run produces `docs/security-audits/YYYY-MM-DD-cddrp-run-N.md` where N is the next available integer for that date. The frontmatter records `methodology-version` from this document; the next protocol revision increments the major version, enabling delta-analysis between runs.

---

## 6. Operating Principles (Cross-Cutting Invariants)

1. **Discovery and remediation are strictly separated.** No fix lands before Phase 2 verification of the underlying finding, except by explicit user override.

2. **Working-tree integrity is paramount.** Files in W (uncommitted modifications) are never edited automatically. Phase 3 may add new findings to the deferred registry rather than risk a merge tangle with the user's WIP.

3. **Parallelism is the primary scaling lever.** Phase 1 and Phase 2 each launch 30–50 background agents; the main thread coordinates and prepares user-facing artefacts. Sequential execution is reserved for nuanced solo remediation.

4. **Prescriptive sub-agent prompts.** Discovery agents receive defect-class taxonomies; verification agents receive specific findings; remediation agents receive exact substitution patterns. Vague prompts produce vague outputs.

5. **Severity is dynamic, not sacred.** Phase 2 downgrade heuristics (§4.4.3) may bound impact; agreement across multiple discovery agents (§5.2) may amplify it.

6. **Checkpoints are mandatory.** Phases 1→2, 2→3 are user-gated. The user may stop, narrow scope, or override defaults at each checkpoint via Spokenly.

7. **No destructive operations without explicit instruction.** Per Claude Code global policy, the protocol never runs `git commit`, `git push`, `rm -rf`, or DDL beyond the run_migrations path.

8. **The audit corpus is cumulative.** Each run dedupes against prior reports under `docs/security-audits/`. Findings already documented are not re-reported unless their state has changed (e.g. previously DEFERRED, now eligible).

---

## Appendix A — Default Coverage Matrix (POOOL repository)

### A.1 Axis A — Domain bands (25 cells)

**Backend (15):** A1 Auth · A2 Admin (excl. mod.rs if WIP) · A3 KYC + Compliance + Legal · A4 Payments + payment_methods + Cart · A5 Wallet · A6 Marketplace · A7 Dividends + Rewards · A8 Blockchain · A9 Developer · A10 Assets + IPFS + Storage · A11 Community · A12 Support + Inbox + Email + Blog · A13 Infra core (lib/main/db/cache/config/templates/metrics/error) · A14 Common + Leaderboard + Portfolio + Settings · A15 Backend tests + bin

**Frontend (10):** A16 Developer pages · A17 Auth pages · A18 Money pages · A19 Marketplace pages · A20 Community pages · A21 Admin pages · A22 Marketing + landing + rest · A23 Shared components + head · A24 Static JS (non-developer) · A25 Static CSS (non-developer)

### A.2 Axis B — Scan dimensions (8 cells)

B1 SQL-injection + dynamic-query · B2 XSS sinks · B3 SSRF / outbound HTTP · B4 Authz / permission-check gaps · B5 Money math / integer overflow · B6 Resource leaks / unbounded queries · B7 Auth / session / CSRF · B8 File upload / path traversal

### A.3 Axis C — Tooling sweep (5–8 cells)

C1 `cargo clippy` · C2 `cargo audit` · C3 `npm audit` (if applicable) · C4 Hardcoded-secret grep · C5 TODO/FIXME triage · C6 DB schema vs query consistency · C7 Migration replay sanity · C8 Test coverage gap on Critical paths

---

## Appendix B — Audit Report Schema

```markdown
---
methodology: CDDRP
methodology-version: 2.0
run-date: YYYY-MM-DD
run-number: N
scope: <scope predicate or "full">
prior-audits-deduped: <list of paths>
---

# Run N — Audit + Verification Report

## §1 Aggregate metrics
- Discovery: <Crit/High/Med/Low counts>
- Verification: <Confirmed/FP/Stale/Downgraded/NeedsRepro counts>
- False-positive rate: X.X%
- Remediation: Y fixed · Z deferred

## §2 Critical findings (verified)
| File:Line | CWE | Issue | Evidence | Action |

## §3 High findings (verified)
…

## §4 Medium findings
(condensed paragraphs, full lists in agent transcripts)

## §5 Low findings
…

## §6 False-positive registry
| Finding | Reason |

## §7 Downgrade registry
| Finding | Old → New severity | Bounding control |

## §8 Needs-repro registry
| Finding | Proposed PoC |

## §9 Bundle catalogue
For each bundle:
- Pattern · Inventory · Fix shape · Complexity · PR scope

## §10 End-to-end flow audits (specialist findings)
For each flow: trace + inter-step issues + recommendations

## §11 Remediation log
| Finding | File:Line | Fix landed | Verifier |

## §12 Deferred-finding registry
| Finding | Severity | Blocker (§4.6.1 precondition that failed) |

## §13 Next-action recommendations
```

---

## Appendix C — Default Bundle Patterns

The following recurring patterns have been observed in this codebase and serve as default targets for Sub-army 2B (§4.4.4). The list is not exhaustive; bundle hunters may propose additional patterns.

| ID | Pattern | Fix shape |
|---|---|---|
| F1 | `AdminUser` extractor without `require_permission` call in handler body | Insert `admin.require_permission(&state.db, "<slug>").await?;` |
| F2 | `field.bytes().await` before size check in multipart handler | Replace with `read_field_capped(&mut field, MAX, "label")` (helper at `backend/src/storage/upload_helpers.rs`) |
| F3 | `innerHTML =` sink with user/API data | Switch to `textContent`, or pre-escape via shared util |
| F4 | Multiple `escapeHtml` / `escapeAttr` variants across frontend JS | Consolidate to one util module; import everywhere |
| F5 | `parseFloat(x) * 100` for money-cents math | Replace with `parseAmountCents(x)` helper |
| F6 | Hardcoded operator emails / domains | Move to `platform_settings` table; resolve at runtime |
| F7 | `#[allow(dead_code)]` on actually-live functions | Strip the attribute (live caller exists) |
| F8 | `target="_blank"` without `rel="noopener noreferrer"` | Append `rel="noopener noreferrer"` |

---

## Appendix D — Known Remediated Patterns (Prior Runs)

To prevent re-flagging, the following remediation campaigns have been completed:

- **R1 (2026-05-19):** F1 critical subset (6 handlers: `pause`, `unpause`, `pin_metadata`, `retention_run`, `retention_arm`, `api_admin_search`).
- **R2 (2026-05-19):** F2 (helper extracted; 5 callsites converted).
- **R3 (2026-05-19):** F8 (12 sites).
- **R4 (2026-05-19):** Singleton fixes — UTF-8-safe notification title truncation, `ON CONFLICT (name) DO NOTHING` on role create, stale `#[allow(dead_code)]` on `delete_object`, LIKE-wildcard escape in `search_circles`, chunked upload for blog assets, removal of dead `serve_admin_protected`, anonymisation of PII in test comments, `escape_html` on commodity descriptions, CORS `strip_prefix` anchoring, fail-closed migrations, account-deletion confirmation phrase actually sent and validated, `rows_affected` guard on wallet rebind, 3 auth log lines redacted of email PII, 500 HTML body sanitised (3 sites in `routes_helper.rs`).

### Known DEFERRED (pending user decision)

- 2FA login-time bypass (`backend/src/auth/routes.rs:253, :1556`) — code reads `is_2fa_verified=true`; comment states "temporarily disabled". User must authorise restoration.
- Settlement private key passed as `cast --private-key` CLI argument — refactor to env / stdin requires verification against `cast` tool semantics.
- Dividend per-holder math (`backend/src/dividends/service.rs:245`) — needs redesign with regression test.
- `/uploads` directory served unauthenticated (`backend/src/lib.rs:1722`) — needs auth-gated static-file design.
- DM rate-limit and privacy-flag bypass (`backend/src/community/routes.rs:4336+`) — needs per-pair vs per-recipient rate-limit design.
- SSRF in OpenGraph parser (`backend/src/community/routes.rs:229`) — needs private-IP allowlist design.

---

## References

- **OWASP Top 10:2021.** Open Web Application Security Project, 2021.
- **CWE — Common Weakness Enumeration.** MITRE, current revision.
- **CVSS v3.1 Specification Document.** FIRST.org, 2019.
- **NIST SP 800-53 Rev. 5 — Security and Privacy Controls.** National Institute of Standards and Technology, 2020.
- **OWASP ASVS v4.0.3 — Application Security Verification Standard.** OWASP, 2021.
- **STRIDE Threat Model.** Microsoft Security Development Lifecycle, current revision.
