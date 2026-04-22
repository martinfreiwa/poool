#!/usr/bin/env python3
"""Fix the corrupted end of IMPLEMENTATION_ROADMAP.md"""
import os

path = 'docs/IMPLEMENTATION_ROADMAP.md'
with open(path, 'r', encoding='utf-8', errors='replace') as f:
    lines = f.readlines()

# Keep lines 0-748 (lines 1-749 in 1-indexed)
clean_lines = lines[:749]

# Add the fixed ending
ending = """                 Phase 11 (Testing) ===    Phase 11 contd ========

Week 14-16       Phase 22 (Banking) ========================================================================

Week 16-18       Phase 15 (Launch) =========================================================================
```

**Legend:** `===` = active work, `---` = idle/waiting, `GATE` = hard dependency

**New Parallelism Opportunities:**
- Phase 21A (Foundry/Solidity) can run fully parallel to ALL backend phases
- Phase 18.10 (Fee Fix) is independent and should be done ASAP
- Phase 20B (Security Hardening) can run parallel to Trading Engine work
- Phase 20C (CI/CD) can run parallel to everything

---

## Critical Warnings

> [!CAUTION]
> **Smart Contract Audit must be commissioned in Week 4!** It has a 4-6 week lead time.
> Without it, Phase 15 (Launch) is blocked.

> [!CAUTION]
> **`backend/src/main.rs` is a bottleneck file.** Multiple phases need to add routes here.
> Only ONE agent may edit `main.rs` at a time. Add routes at the END of the relevant section.

> [!CAUTION]
> **Phase 3 (Trading Engine) is the critical path.** Everything depends on it. Assign your strongest/fastest agent to this phase. Do NOT split Phase 3 across multiple agents -- the files are too interconnected.

> [!CAUTION]
> **P1-FINANCIAL: Phase 18.10 (Platform Fee Float to Decimal Fix) is a P1 bug.** This MUST be the first task executed as it directly affects real money calculations. A f64 multiplication on financial amounts violates the zero-defect financial engineering standard.

> [!CAUTION]
> **Phase 22 (Banking API) requires OCBC bank agreement.** This is an external dependency with potentially weeks of lead time. The PM should initiate the OCBC relationship in Week 1 alongside the Smart Contract Audit commissioning.

---

## Task Count Summary

| Phase | Name | Tasks | Status |
|:---|:---|:---|:---|
| 0 | Infrastructure | 17 | Mixed |
| 1 | Backend Hardening | 11 | Mixed |
| 2 | DB Migrations | 10 | Mixed |
| 3 | Trading Engine | 16 | Mixed |
| 4 | WebSocket Server | 4 | Mixed |
| 5 | Frontend Trading UI | 10 | Mixed |
| 6A | Admin Backend APIs | 15 | 14/15 DONE |
| 6B | Admin Frontend Pages | 14 | 13/14 DONE |
| 7 | Smart Contracts | -- | Future |
| 8 | Blockchain Integration | -- | Future |
| 9 | Dividend System | -- | Future |
| 10 | Integration & Security | -- | DONE |
| 11 | Testing & QA | -- | Future |
| 12-13 | Legal / OJK | -- | External |
| 14 | Community | -- | Mixed |
| 15 | Soft Launch | -- | Future |
| 16 | Primary Issuance | -- | Future |
| 17 | RegTech | 5 | 2/5 DONE |
| **18** | **FI-System & Treasury** | **15** | **0/15** |
| **19** | **Affiliate Subsystem** | **12** | **0/12** |
| **20** | **Core Admin & Operations** | **15** | **0/15** |
| **21** | **Smart Contract & Blockchain** | **18** | **0/18** |
| **22** | **Banking API & Settlement** | **8** | **0/8** |
| | **TOTAL NEW TASKS** | **68** | |
"""

clean_lines.append(ending)
with open(path, 'w', encoding='utf-8') as f:
    f.writelines(clean_lines)
print(f'Fixed! New line count: {sum(1 for _ in open(path))}')
