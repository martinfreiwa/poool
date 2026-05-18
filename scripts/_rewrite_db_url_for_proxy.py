#!/usr/bin/env python3
"""
Rewrite a Cloud SQL DATABASE_URL so it connects through a local Cloud SQL
Auth Proxy listening on 127.0.0.1:5432.

Handles two common shapes of secret:

  1. TCP form        postgres://user:pass@some-host[:port]/db[?args]
  2. Unix socket form postgres://user:pass@/db?host=/cloudsql/INSTANCE[&args]

In both cases, output:
  postgres://user:pass@127.0.0.1:5432/db?<args without host=>&sslmode=disable

Reads from stdin, writes to stdout.
"""
import sys
import urllib.parse as up


def rewrite(url: str) -> str:
    u = up.urlsplit(url)
    # Preserve userinfo verbatim (already percent-encoded in the secret).
    # urlsplit gives us the raw netloc; carve off everything after '@'.
    raw_netloc = u.netloc
    userinfo = ""
    if "@" in raw_netloc:
        userinfo = raw_netloc.rsplit("@", 1)[0] + "@"
    netloc = userinfo + "127.0.0.1:5432"

    # Strip any host= query (Cloud SQL socket pointer) and replace sslmode.
    qs = up.parse_qsl(u.query, keep_blank_values=True)
    qs = [(k, v) for (k, v) in qs if k.lower() != "host"]
    qs = [(k, v) for (k, v) in qs if k.lower() != "sslmode"]
    qs.append(("sslmode", "disable"))

    new = u._replace(netloc=netloc, query=up.urlencode(qs))
    return up.urlunsplit(new)


if __name__ == "__main__":
    src = sys.stdin.read().strip()
    if not src:
        print("empty input", file=sys.stderr)
        sys.exit(1)
    print(rewrite(src))
