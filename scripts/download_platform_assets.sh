#!/bin/bash
COOKIE="auth_session=authenticated; session_id=session_dguvurawipxe"
BASE="https://platform.poool.app"
DEST="/Users/martin/Downloads/poool/platform.poool.app"

download() {
  local url="$1"
  local filepath="${DEST}${url}"
  local dir=$(/usr/bin/dirname "$filepath")
  /bin/mkdir -p "$dir"
  /usr/bin/curl -s -L -b "$COOKIE" -o "$filepath" "${BASE}${url}"
  local size=$(/usr/bin/wc -c < "$filepath" 2>/dev/null || echo 0)
  echo "  ${url} -> ${size} bytes"
}

echo "=== Downloading CSS ==="
for css in \
  /static/css/main.css \
  /static/css/marketplace.css \
  /static/css/sidebar-navigation.css \
  /static/css/profile-dropdown.css \
  /static/css/wallet.css \
  /static/css/portfolio.css \
  /static/css/cart.css \
  /static/css/property-item-card.css \
  /static/css/community-card.css \
  /static/css/kyc-banner.css \
  /static/css/htmx-fixes.css \
  /static/css/mobile-burger-menu.css \
  /static/css/mobile-header.css \
  /static/css/mobile-kyc-banner.css \
  /static/css/mobile-profile-dropdown.css \
  /static/css/sidebar-developer.css \
  /static/css/assets-table-populated.css \
  "/static/css/bem/sidebar.css"; do
  download "$css"
done

echo "=== Downloading JS ==="
for js in \
  /static/js/htmx-init.js \
  /static/js/marketplace.js \
  /static/js/mobile-navigation.js \
  /static/js/profile-dropdown.js; do
  download "$js"
done

echo "=== Downloading images ==="
for img in \
  "/images/Logo Pool.svg" \
  "/images/Logo premium.svg" \
  /images/Bed.svg \
  "/images/Featured icon.png" \
  /images/ID.png \
  /images/Image.png \
  /images/award-05.svg \
  /images/building-06.svg \
  /images/building-07.svg \
  /images/coins-stacked-02.svg \
  /images/coins-swap-01.svg \
  /images/dollar.svg \
  /images/file-check-02.svg \
  /images/home-03.svg \
  /images/home-05.svg \
  /images/line-chart-up-02.svg \
  /images/settings-01.svg \
  /images/shopping-cart-01.svg \
  /images/star-01.svg \
  /images/star-06.svg \
  /images/trophy-01.svg \
  /images/wallet-02.svg \
  "/static/images/message-chat-circle grey.svg" \
  "/static/images/Menu developer/Assets.svg" \
  "/static/images/Menu developer/Dashboard.svg" \
  "/static/images/Menu developer/Notifications.svg" \
  "/static/images/Menu developer/Ranking.svg" \
  "/static/images/Menu developer/Settings.svg" \
  "/static/images/Menu developer/Support.svg"; do
  download "$img"
done

echo ""
echo "=== Now downloading additional assets from other pages ==="

# Check wallet page for additional assets
for extrapage in wallet portfolio cart support settings kyc commodities-marketplace; do
  extras=$(/usr/bin/grep -oh 'href="/[^"]*\.css"\|src="/[^"]*\.js"\|src="/[^"]*\.svg"\|src="/[^"]*\.png"\|src="/[^"]*\.webp"' "${DEST}/${extrapage}.html" 2>/dev/null | /usr/bin/sed 's/^[^"]*"//;s/"$//' | /usr/bin/sort -u)
  for asset in $extras; do
    if [ ! -f "${DEST}${asset}" ]; then
      download "$asset"
    fi
  done
done

echo ""
echo "=== Done! ==="
