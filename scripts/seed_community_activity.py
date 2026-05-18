#!/usr/bin/env python3
"""
Seed community activity (posts, comments, reactions) using existing community
profiles. Content references real assets from the platform DB.

Two modes:

  --mode backfill   Wipe + regenerate N posts spread over a date window.
                    Local dev only. Refuses to run against DATABASE_URL.

  --mode daily      Additive. Adds today's batch (1-2 posts, 2-3 comments,
                    5-6 reactions). Safe for production. Tags every row
                    with content_tags = {'seed:auto'} so cleanup is trivial.

Connections:
  - PLATFORM_DATABASE_URL  (default postgres:///poool)        — reads assets
  - COMMUNITY_DATABASE_URL (default postgres:///poool_community) — writes posts
  - In prod, both fall back to DATABASE_URL (single combined DB).

Usage:
    # Local 12-month backfill (destructive):
    python3 scripts/seed_community_activity.py --mode backfill --posts 500

    # Production daily increment:
    DATABASE_URL=postgres://... python3 scripts/seed_community_activity.py --mode daily
"""
from __future__ import annotations

import argparse
import os
import random
import sys
import uuid
from datetime import datetime, timedelta, timezone

import psycopg2
import psycopg2.extras

POST_TYPES = ["general", "market_insight", "milestone", "review", "farm_update"]
POST_TYPE_WEIGHTS = [50, 20, 12, 12, 6]
REACTION_TYPES = ["fire", "insightful", "clap", "green"]
REACTION_WEIGHTS = [50, 25, 20, 5]

ASSET_TITLE_BLOCKLIST_SUBSTR = ("test", "demo apartment", "ac-asset-")
SEED_MARKER = "seed:auto"

POST_TEMPLATES_GENERAL = [
    "Just topped up my position in {asset}. Slow and steady wins.",
    "{asset} is honestly the most underrated asset on the platform right now.",
    "Anyone else holding {asset}? Curious how you're sizing it.",
    "Closed out a small slice of {asset} after the last payout. Reinvesting in {asset2}.",
    "{asset} vs {asset2} — which one are you leaning into this quarter?",
    "Three months in on {asset} and the cashflow has been remarkably stable.",
    "Sold my last tokens of {asset} today. Good run, on to the next.",
    "Looking to add {asset} to my portfolio. Anyone with first-hand experience?",
    "Quietly accumulating {asset} on dips. Long-term thesis still intact.",
    "Took some profit on {asset} this week — using it to bid into {asset2}.",
]
POST_TEMPLATES_MARKET = [
    "Yield on {asset} has held up better than I expected through the last cycle.",
    "Distributions on {asset} hit my wallet on time again. Consistency matters.",
    "Tokens available on {asset} dropping fast — secondary market is tight.",
    "Comparing the cap rates on {asset} and {asset2}. The spread surprised me.",
    "Occupancy numbers for {asset} look healthier than last quarter's update.",
    "{asset} is starting to behave like the staple of my real-estate sleeve.",
    "Coffee and cacao positions are quietly outperforming my villa allocations.",
    "Bali real estate still feels mispriced versus comparable European yields.",
]
POST_TEMPLATES_MILESTONE = [
    "Hit my first €10k in distributions across {asset} and {asset2}. Took two years.",
    "Reached 100 tokens of {asset} today. Long road from where I started.",
    "Fully exited {asset} for a clean return. Letting it ride into {asset2}.",
    "First full year on the platform — boring portfolio, exciting returns.",
    "Crossed €25k total invested. {asset} is still my largest single position.",
    "Compounded payouts paid for this month's mortgage. Plan is working.",
]
POST_TEMPLATES_REVIEW = [
    "Honest take on {asset}: solid yield, slow appreciation, would buy again.",
    "{asset} review — operator communication is the best I've seen on POOOL.",
    "Mixed feelings on {asset}. Yield is fine, but the location story moved.",
    "If you're new and want one safe pick: {asset}. Not exciting, just works.",
    "{asset} reporting is clear, monthly, and on time. That's all I want.",
    "I'd put {asset} in the 'set and forget' bucket. Boring in the best way.",
]
POST_TEMPLATES_FARM = [
    "Harvest update on {asset} came through — yield numbers above forecast.",
    "Weather on the {asset} plot was rough this month, expecting a softer payout.",
    "Visited the {asset} site last week. Operator was clearly hands-on.",
    "Cacao quality from {asset} is genuinely premium — saw the lab numbers.",
    "Rice cycle on {asset} closed strong. Reinvesting payouts into more tokens.",
]
TEMPLATES_BY_TYPE = {
    "general": POST_TEMPLATES_GENERAL,
    "market_insight": POST_TEMPLATES_MARKET,
    "milestone": POST_TEMPLATES_MILESTONE,
    "review": POST_TEMPLATES_REVIEW,
    "farm_update": POST_TEMPLATES_FARM,
}

COMMENT_TEMPLATES = [
    "Same boat here — {asset} has been the most reliable line in my dashboard.",
    "How long have you been holding? I'm two payouts in on {asset}.",
    "Curious about your exit plan. Hold-forever or rotate when liquidity opens?",
    "Agreed. {asset} doesn't get the airtime it deserves.",
    "Watching this one. The operator updates pushed me over the edge.",
    "What sold you on {asset} over the alternatives?",
    "I went the other way and rotated out last month. Different theses, both fine.",
    "Bookmarking. Doing my own diligence on {asset} this week.",
    "Yield-on-cost on mine is creeping above 8%. Hard to argue with that.",
    "Tempted, but waiting for the next offering. Liquidity is the limit for me.",
    "Operator transparency is what keeps me in.",
    "Sized this conservatively but glad I held. Compounds quietly.",
    "Echoing this. Distributions hit like clockwork on my side too.",
    "Did the same — small position to start, scaled in after the first payout.",
    "How's the FX angle hitting you? Asking as a EUR investor.",
]


def get_conns():
    primary = os.environ.get("DATABASE_URL")
    platform_url = (
        os.environ.get("PLATFORM_DATABASE_URL")
        or primary
        or "postgres:///poool"
    )
    community_url = (
        os.environ.get("COMMUNITY_DATABASE_URL")
        or primary
        or "postgres:///poool_community"
    )
    return psycopg2.connect(platform_url), psycopg2.connect(community_url)


def load_assets(conn):
    with conn.cursor() as cur:
        cur.execute("SELECT id, title FROM assets WHERE title IS NOT NULL")
        return [
            (aid, title)
            for aid, title in cur.fetchall()
            if not any(b in title.lower() for b in ASSET_TITLE_BLOCKLIST_SUBSTR)
        ]


def load_profiles(conn):
    with conn.cursor() as cur:
        cur.execute(
            "SELECT user_id FROM community_profiles "
            "WHERE NOT is_community_banned "
            "AND COALESCE(is_shadowbanned, false) = false"
        )
        return [row[0] for row in cur.fetchall()]


def weighted_choice(items, weights):
    return random.choices(items, weights=weights, k=1)[0]


def make_content(post_type, assets):
    a, b = random.sample(assets, 2)
    text = random.choice(TEMPLATES_BY_TYPE[post_type]).format(asset=a[1], asset2=b[1])
    referenced_asset_id = a[0] if random.random() < 0.6 else None
    return text, referenced_asset_id


def make_comment(post_asset_title, all_assets):
    title = post_asset_title or random.choice(all_assets)[1]
    return random.choice(COMMENT_TEMPLATES).format(asset=title)


def build_author_pool(profiles, power_user_count=150):
    random.shuffle(profiles)
    n = min(power_user_count, len(profiles) // 2)
    return profiles[:n], profiles[n:]


def pick_author(power, long_tail):
    if random.random() < 0.70 and power:
        return random.choice(power)
    return random.choice(long_tail) if long_tail else random.choice(power)


def generate(posts_target, window_start, window_end, assets, profiles, marker):
    power, long_tail = build_author_pool(profiles)
    posts, comments, reactions = [], [], []
    seen_reactions = set()

    span_seconds = max(int((window_end - window_start).total_seconds()), 1)

    for _ in range(posts_target):
        post_id = uuid.uuid4()
        author = pick_author(power, long_tail)
        post_type = weighted_choice(POST_TYPES, POST_TYPE_WEIGHTS)
        content, asset_id = make_content(post_type, assets)
        ts = window_start + timedelta(seconds=random.randint(0, span_seconds))
        posts.append({
            "id": post_id,
            "user_id": author,
            "post_type": post_type,
            "content": content,
            "asset_id": asset_id,
            "created_at": ts,
            "content_tags": [marker],
        })

        reaction_count = min(int(random.lognormvariate(1.2, 0.9)), 15)
        for r in random.sample(profiles, k=min(reaction_count, len(profiles))):
            rtype = weighted_choice(REACTION_TYPES, REACTION_WEIGHTS)
            key = (post_id, r, rtype)
            if key in seen_reactions:
                continue
            seen_reactions.add(key)
            r_ts = ts + timedelta(seconds=random.randint(60, 14 * 86400))
            if r_ts > window_end:
                r_ts = window_end
            reactions.append({
                "post_id": post_id, "user_id": r,
                "reaction_type": rtype, "created_at": r_ts,
            })

        comment_count = min(int(random.lognormvariate(0.5, 0.9)), 8)
        for _ in range(comment_count):
            commenter = pick_author(power, long_tail)
            title = next((t for aid, t in assets if aid == asset_id), None)
            c_ts = ts + timedelta(seconds=random.randint(120, 30 * 86400))
            if c_ts > window_end:
                c_ts = window_end
            comments.append({
                "id": uuid.uuid4(), "post_id": post_id,
                "user_id": commenter, "content": make_comment(title, assets),
                "created_at": c_ts,
            })

    return posts, comments, reactions


def insert(community_conn, posts, comments, reactions, wipe_marker=None):
    with community_conn:
        with community_conn.cursor() as cur:
            if wipe_marker == "ALL":
                cur.execute("DELETE FROM reactions")
                cur.execute("DELETE FROM comment_reactions")
                cur.execute("DELETE FROM post_hashtags")
                cur.execute("DELETE FROM comments")
                cur.execute("DELETE FROM bookmarks")
                cur.execute("DELETE FROM posts")
                cur.execute(
                    "UPDATE community_profiles SET post_count = 0, "
                    "follower_count = 0, following_count = 0"
                )

            psycopg2.extras.execute_values(
                cur,
                """
                INSERT INTO posts
                  (id, user_id, post_type, content, asset_id,
                   reaction_count, comment_count, content_tags,
                   created_at, updated_at)
                VALUES %s
                """,
                [
                    (
                        str(p["id"]), str(p["user_id"]), p["post_type"],
                        p["content"],
                        str(p["asset_id"]) if p["asset_id"] else None,
                        0, 0, p["content_tags"],
                        p["created_at"], p["created_at"],
                    )
                    for p in posts
                ],
                page_size=500,
            )

            if reactions:
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO reactions
                      (post_id, user_id, reaction_type, created_at)
                    VALUES %s
                    ON CONFLICT (post_id, user_id, reaction_type) DO NOTHING
                    """,
                    [
                        (str(r["post_id"]), str(r["user_id"]),
                         r["reaction_type"], r["created_at"])
                        for r in reactions
                    ],
                    page_size=1000,
                )

            if comments:
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO comments
                      (id, post_id, user_id, content, created_at)
                    VALUES %s
                    """,
                    [
                        (str(c["id"]), str(c["post_id"]), str(c["user_id"]),
                         c["content"], c["created_at"])
                        for c in comments
                    ],
                    page_size=500,
                )

            # Recompute denorm counts for affected posts.
            post_ids = [str(p["id"]) for p in posts]
            if post_ids:
                cur.execute(
                    """
                    UPDATE posts p
                    SET comment_count = sub.cnt
                    FROM (
                      SELECT post_id, COUNT(*) AS cnt
                      FROM comments WHERE post_id = ANY(%s::uuid[])
                      GROUP BY post_id
                    ) sub
                    WHERE p.id = sub.post_id
                    """,
                    (post_ids,),
                )

            # Recompute post_count for all distinct authors touched.
            author_ids = list({str(p["user_id"]) for p in posts})
            if author_ids:
                cur.execute(
                    """
                    UPDATE community_profiles cp
                    SET post_count = sub.cnt
                    FROM (
                      SELECT user_id, COUNT(*) AS cnt
                      FROM posts WHERE user_id = ANY(%s::uuid[])
                      GROUP BY user_id
                    ) sub
                    WHERE cp.user_id = sub.user_id
                    """,
                    (author_ids,),
                )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["backfill", "daily"], default="daily")
    parser.add_argument("--posts", type=int, default=None,
                        help="backfill: total posts (default 500); daily: posts today (default 1-2)")
    parser.add_argument("--days", type=int, default=365,
                        help="backfill window size in days (default 365)")
    parser.add_argument("--end-date", default=None,
                        help="ISO date for window end (default today UTC)")
    parser.add_argument("--seed", type=int, default=None)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    if args.seed is not None:
        random.seed(args.seed)

    platform_conn, community_conn = get_conns()

    assets = load_assets(platform_conn)
    profiles = load_profiles(community_conn)

    if len(assets) < 2:
        print(f"ABORT: need >=2 non-test assets, found {len(assets)}", file=sys.stderr)
        sys.exit(2)
    if len(profiles) < 10:
        print(f"ABORT: need >=10 unbanned community_profiles, found {len(profiles)}", file=sys.stderr)
        sys.exit(2)

    end = (
        datetime.fromisoformat(args.end_date).replace(tzinfo=timezone.utc)
        if args.end_date
        else datetime.now(timezone.utc)
    )

    if args.mode == "backfill":
        # Safety: refuse to wipe a non-local DB.
        url = os.environ.get("COMMUNITY_DATABASE_URL") or os.environ.get("DATABASE_URL") or ""
        is_local = (
            url == ""  # default postgres:///poool_community
            or "localhost" in url
            or "127.0.0.1" in url
            or url.startswith("postgres:///")
        )
        if not is_local:
            print("ABORT: backfill mode refuses to wipe a non-local DB.", file=sys.stderr)
            sys.exit(2)
        posts_target = args.posts or 500
        start = end - timedelta(days=args.days)
        wipe = "ALL"
    else:  # daily
        # Default: 1-2 posts, weighted toward 2.
        posts_target = args.posts if args.posts is not None else random.choice([1, 2, 2])
        # Today's window in UTC.
        start = end - timedelta(hours=24)
        wipe = None

    posts, comments, reactions = generate(
        posts_target, start, end, assets, profiles, SEED_MARKER
    )

    print(
        f"[{args.mode}] window={start.isoformat()}..{end.isoformat()} "
        f"assets={len(assets)} profiles={len(profiles)} "
        f"-> posts={len(posts)} comments={len(comments)} reactions={len(reactions)}"
    )

    if args.dry_run:
        for p in posts[:3]:
            print(p)
        return

    insert(community_conn, posts, comments, reactions, wipe_marker=wipe)
    print("OK")


if __name__ == "__main__":
    main()
