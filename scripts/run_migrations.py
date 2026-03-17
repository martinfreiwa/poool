#!/usr/bin/env python3
"""
POOOL Database Migration Runner
Applies all SQL migrations in order against the target database.
Usage:
  python3 scripts/run_migrations.py
  DATABASE_URL=postgres://user:pass@host/db python3 scripts/run_migrations.py --dry-run
"""

import os
import sys
import argparse
import subprocess
from pathlib import Path

MIGRATIONS_DIR = Path(__file__).parent.parent / "database"

MIGRATION_ORDER = [
    "001_initial_schema.sql",
    "001a_prep_users.sql",
    "002_seed_data.sql",
    "002_payment_methods.sql",
    "003_settings_extensions.sql",
    "004_rewards_schema.sql",
    "005_payments_checkout.sql",
    "006_admin_settings.sql",
    "007_support_ticket_replies.sql",
    "008_email_marketing.sql",
    "008_email_system.sql",
    "009_legal_consents.sql",
    "010_advanced_rbac.sql",
    "011_tax_reporting.sql",
    "012_payment_disputes.sql",
    "013_admin_background_and_idempotency.sql",
    "014_optimization_indexes.sql",
]


def get_database_url() -> str:
    url = os.environ.get("DATABASE_URL", "")
    if not url:
        print("ERROR: DATABASE_URL environment variable is not set.", file=sys.stderr)
        print("       Export it or add it to your .env file.", file=sys.stderr)
        sys.exit(1)
    return url


def run_migration(db_url: str, migration_file: Path, dry_run: bool) -> bool:
    if not migration_file.exists():
        print(f"  SKIP  {migration_file.name} (file not found)")
        return True

    print(f"  {'DRY-RUN' if dry_run else 'APPLY '} {migration_file.name} ...", end=" ", flush=True)

    if dry_run:
        print("OK (skipped)")
        return True

    result = subprocess.run(
        ["psql", db_url, "-f", str(migration_file)],
        capture_output=True,
        text=True,
    )

    if result.returncode == 0:
        print("OK")
        return True
    else:
        print("FAILED")
        print(f"         stdout: {result.stdout[:300]}")
        print(f"         stderr: {result.stderr[:300]}")
        return False


def main():
    parser = argparse.ArgumentParser(description="POOOL Migration Runner")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="List migrations without applying them",
    )
    parser.add_argument(
        "--from",
        dest="from_migration",
        default=None,
        help="Start from this migration file (e.g. 006_admin_settings.sql)",
    )
    args = parser.parse_args()

    db_url = get_database_url()
    print(f"\nPOOOL Migration Runner")
    print(f"Database: {db_url.split('@')[-1] if '@' in db_url else db_url}")
    print(f"Mode:     {'DRY RUN' if args.dry_run else 'LIVE'}")
    print(f"Migrations: {MIGRATIONS_DIR}\n")

    start_applying = args.from_migration is None
    failed = 0
    applied = 0

    for name in MIGRATION_ORDER:
        if not start_applying:
            if name == args.from_migration:
                start_applying = True
            else:
                print(f"  SKIP  {name} (before --from)")
                continue

        migration_path = MIGRATIONS_DIR / name
        ok = run_migration(db_url, migration_path, args.dry_run)
        if ok:
            applied += 1
        else:
            failed += 1
            print(f"\nAborting — fix the error above before re-running.\n")
            sys.exit(1)

    print(f"\n{'='*50}")
    print(f"Done: {applied} migration(s) {'listed' if args.dry_run else 'applied'}, {failed} failed.")
    if not args.dry_run and failed == 0:
        print("Database is up to date.")


if __name__ == "__main__":
    main()
