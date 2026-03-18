---
description: Push changes to GitHub with SQLx cache safety check
---

# Push to GitHub

// turbo-all

This workflow ensures that all changes — including the SQLx offline cache — are committed and pushed safely.

## Steps

1. **Regenerate SQLx offline cache** (catches any new/changed SQL queries):
```bash
cd /Users/martin/Projects/poool/backend && cargo sqlx prepare
```

2. **Verify offline compilation** (simulates Cloud Build environment):
```bash
cd /Users/martin/Projects/poool/backend && SQLX_OFFLINE=true cargo check --release 2>&1 | tail -5
```
If this fails, something is wrong with the `.sqlx` cache — do NOT push.

3. **Stage ALL changes** (backend + frontend + .sqlx cache):
```bash
cd /Users/martin/Projects/poool && git add .
```

4. **Review what will be committed**:
```bash
cd /Users/martin/Projects/poool && git status
```
Verify that `.sqlx/` files are included if any Rust SQL queries changed.

5. **Commit with a descriptive message**:
```bash
cd /Users/martin/Projects/poool && git commit -m "<describe changes>"
```

6. **Push to GitHub**:
```bash
cd /Users/martin/Projects/poool && git push
```
