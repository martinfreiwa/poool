-- Attach the generated apartment gallery to the live demo apartment listing.
-- Idempotent: replaces image rows for this slug if the asset exists.

DO $$
DECLARE
    v_asset_id UUID;
BEGIN
    SELECT id
    INTO v_asset_id
    FROM assets
    WHERE slug = 'demo-apartment-01---investment-6016e69e-15d0-4547-996a-ca560541f3fa'
    LIMIT 1;

    IF v_asset_id IS NULL THEN
        RAISE NOTICE 'demo apartment asset not found, skipping image migration';
        RETURN;
    END IF;

    DELETE FROM asset_images
    WHERE asset_id = v_asset_id;

    INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
    VALUES
        (
            v_asset_id,
            '/static/images/properties/demo_apartment_01/apartment-exterior.png',
            'Modern apartment building exterior with balconies and shared entrance',
            0,
            TRUE
        ),
        (
            v_asset_id,
            '/static/images/properties/demo_apartment_01/apartment-living-room.png',
            'Bright apartment living room with balcony and city view',
            1,
            FALSE
        ),
        (
            v_asset_id,
            '/static/images/properties/demo_apartment_01/apartment-kitchen-dining.png',
            'Compact modern apartment kitchen and dining area',
            2,
            FALSE
        ),
        (
            v_asset_id,
            '/static/images/properties/demo_apartment_01/apartment-bedroom.png',
            'Apartment bedroom with built-in wardrobe and neighboring building view',
            3,
            FALSE
        ),
        (
            v_asset_id,
            '/static/images/properties/demo_apartment_01/apartment-balcony-view.png',
            'Upper-floor apartment balcony with chairs and city view',
            4,
            FALSE
        );
END $$;
