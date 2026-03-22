CREATE TABLE announcement_categories (
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    category        VARCHAR(30) NOT NULL
                    CHECK (category IN ('new_commodity', 'dividend', 'platform_update',
                                        'market_news', 'farm_update')),
    PRIMARY KEY(post_id)
);
