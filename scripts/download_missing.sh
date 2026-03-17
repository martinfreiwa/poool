#!/bin/bash
BASE="https://www.poool.app"
OUT="/Users/martin/Downloads/poool/www.poool.app"

URLS=(
  "/en/chunk-HC2LBUVO.js"
  "/svg/background-pattern-grid.svg"
  "/svg/background-buy-shares.svg"
  "/svg/logo-blue.svg"
  "/svg/logo.svg"
  "/svg/play-circle.svg"
  "/svg/video-overlay-card.svg"
  "/svg/poool-text-logo.svg"
  "/svg/quote-dots.svg"
  "/svg/3-circles-logo.svg"
  "/svg/pool-grow-logo.svg"
  "/svg/world-map/japan.svg"
  "/svg/world-map/usa.svg"
  "/svg/world-map/arrow-jagged-up.svg"
  "/svg/world-map/chart-1.svg"
  "/svg/world-map/chart-2.svg"
  "/svg/world-map/bali.svg"
  "/svg/world-map/gb.svg"
  "/svg/world-map/thailand.svg"
  "/svg/world-map/arrow-up.svg"
  "/svg/world-map/oae.svg"
  "/svg/world-map/de.svg"
  "/svg/world-map/icon-home.svg"
  "/svg/world-map/arrow-jagged-up-accent-green.svg"
  "/svg/join-community/building.svg"
  "/svg/join-community/coins-stacked.svg"
  "/svg/join-community/bank-note.svg"
  "/svg/join-community/file-shield.svg"
  "/svg/join-community/coins-swap.svg"
  "/svg/join-community/presentation-chart.svg"
  "/svg/join-community/users-check.svg"
  "/svg/join-community/globe.svg"
  "/svg/investment-amount-page-block/visa.svg"
  "/svg/investment-amount-page-block/wallet.svg"
  "/svg/investment-amount-page-block/master_card.svg"
  "/png/flag-estonia-circle.png"
  "/png/ankr-logo.png"
  "/png/usd1-logo.png"
  "/png/monique-howeth-mobile.png"
  "/png/Team.png"
  "/png/license.png"
  "/png/Jonathan Rizky.png"
  "/png/properties-features/best-price.png"
  "/png/properties-features/insurance.png"
  "/png/Nomad Palm Residence.jpg"
  "/png/Azure Echo House.jpg"
  "/png/Luna Bay Villa.jpg"
  "/png/Khai.jpg"
  "/webp/poool-community.webp"
  "/webp/video-overlay-property-card.webp"
  "/webp/page-block-ownreship/bpn.webp"
  "/webp/team/ryan-fang.webp"
  "/webp/team/monique-howeth.webp"
  "/webp/team/Jonas Freiwald.webp"
  "/webp/team/Dmitry Sikorski.webp"
  "/webp/team/Sean Reno.webp"
  "/webp/team/Patrick Werner.webp"
  "/webp/team/Daniel Todorov.png"
  "/webp/team/Nikita Kokhanevych.png"
  "/webp/team/Mykyta Hlukhuvskyi.png"
  "/webp/team/Nataly Vovque.png"
  "/webp/avatars/Monique Howeth.webp"
  "/webp/avatars/Jonas Thomsen.webp"
  "/webp/avatars/Tobias Weber.webp"
  "/webp/avatars/Mathias Damsgaard.webp"
  "/webm/webm_phone_2.webm"
  "/webm/webm_phone_1.webm"
  "/webm/webm_villa_no-bg.webm"
  "/webm/webm_globe.webm"
  "/webm/webm_villa.webm"
)

SUCCESS=0
FAIL=0

for url in "${URLS[@]}"; do
  dir=$(dirname "${OUT}${url}")
  mkdir -p "$dir"
  
  encoded_url=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$url', safe='/:'))")
  
  echo -n "Downloading: $url ... "
  HTTP_CODE=$(curl -s -o "${OUT}${url}" -w "%{http_code}" "${BASE}${encoded_url}")
  
  if [ "$HTTP_CODE" = "200" ]; then
    echo "OK ($(wc -c < "${OUT}${url}" | tr -d ' ') bytes)"
    SUCCESS=$((SUCCESS + 1))
  else
    echo "FAILED ($HTTP_CODE)"
    rm -f "${OUT}${url}"
    FAIL=$((FAIL + 1))
  fi
done

echo ""
echo "Done! Success: $SUCCESS, Failed: $FAIL"
