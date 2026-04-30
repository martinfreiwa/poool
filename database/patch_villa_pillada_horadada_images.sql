-- Attach generated gallery images to Villa Pillada Horadada.
-- Run this in the environment where the asset slug exists.

BEGIN;

UPDATE assets
SET
    short_description = 'A contemporary two-bedroom Bali villa with a private pool, open-plan living spaces, natural stone finishes, and tropical landscaping.',
    description = 'Villa Pillada Horadada is a fully managed two-bedroom Bali villa designed around a private pool courtyard, warm timber detailing, natural stone, and floor-to-ceiling glass that opens the living spaces to the garden.

The gallery shows a turnkey vacation-rental layout: a bright open-plan living room, modern kitchen and dining area, master bedroom, guest bedroom, spa-style bathroom, sun terrace, and landscaped pool deck. Evening lighting and mature planting give the property a calm premium-rental feel while keeping the architecture practical for day-to-day operations.

For investors, the property is positioned as an income-focused leasehold villa in Bali, with professional management handling guest operations, maintenance, and tenant experience.',
    location_description = 'Bali continues to attract international leisure demand, with private villas, pool access, and well-managed tropical interiors remaining core drivers of guest appeal.'
WHERE slug = 'villa-pillada-horadada-fb7856ce-3332-4b6b-bb71-317599f99008';

WITH target_asset AS (
    SELECT id
    FROM assets
    WHERE slug = 'villa-pillada-horadada-fb7856ce-3332-4b6b-bb71-317599f99008'
)
DELETE FROM asset_images
WHERE asset_id IN (SELECT id FROM target_asset);

WITH target_asset AS (
    SELECT id
    FROM assets
    WHERE slug = 'villa-pillada-horadada-fb7856ce-3332-4b6b-bb71-317599f99008'
)
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
SELECT
    target_asset.id,
    image_data.image_url,
    image_data.alt_text,
    image_data.sort_order,
    image_data.is_cover
FROM target_asset
CROSS JOIN (
    VALUES
        (
            '/static/images/properties/villa_pillada_horadada/hero.webp',
            'Villa Pillada Horadada exterior with private pool',
            0,
            TRUE
        ),
        (
            '/static/images/properties/villa_pillada_horadada/pool.webp',
            'Villa Pillada Horadada pool terrace',
            1,
            FALSE
        ),
        (
            '/static/images/properties/villa_pillada_horadada/living.webp',
            'Villa Pillada Horadada open-plan living room',
            2,
            FALSE
        ),
        (
            '/static/images/properties/villa_pillada_horadada/master-bedroom.webp',
            'Villa Pillada Horadada master bedroom',
            3,
            FALSE
        ),
        (
            '/static/images/properties/villa_pillada_horadada/guest-bedroom.webp',
            'Villa Pillada Horadada guest bedroom',
            4,
            FALSE
        ),
        (
            '/static/images/properties/villa_pillada_horadada/kitchen-dining.webp',
            'Villa Pillada Horadada kitchen and dining area',
            5,
            FALSE
        ),
        (
            '/static/images/properties/villa_pillada_horadada/bathroom.webp',
            'Villa Pillada Horadada ensuite bathroom',
            6,
            FALSE
        ),
        (
            '/static/images/properties/villa_pillada_horadada/dusk-exterior.webp',
            'Villa Pillada Horadada exterior at dusk',
            7,
            FALSE
        )
) AS image_data(image_url, alt_text, sort_order, is_cover);

COMMIT;
