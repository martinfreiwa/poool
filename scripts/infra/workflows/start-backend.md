---
description: Start the POOOL Rust backend server locally
---

## Start Backend Server

// turbo-all

1. Make sure PostgreSQL is running:
```bash
brew services start postgresql@16
```

2. Start the Rust backend:
```bash
cd /Users/martin/Projects/poool/backend && cargo run
```

The server will be available at **http://localhost:8888**

### Key URLs:
- Login: http://localhost:8888/auth/login
- Signup: http://localhost:8888/auth/signup
- Marketplace: http://localhost:8888/marketplace (requires login)

### If port 8888 is already in use:
```bash
lsof -i :8888 -t | xargs kill -9
```

### Watch mode (auto-reload on code changes):
```bash
cargo watch -x run
```
