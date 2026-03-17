#!/bin/bash
COOKIE="auth_session=authenticated; session_id=session_dguvurawipxe"
BASE_DIR="/Users/martin/Downloads/poool"

# Create directories
/bin/mkdir -p "${BASE_DIR}/www.poool.app/id"
/bin/mkdir -p "${BASE_DIR}/platform.poool.app/developer"

echo "Downloading Indonesian Landing Page..."
/usr/bin/curl -s -L -o "${BASE_DIR}/www.poool.app/id/index.html" "https://www.poool.app/id/"
echo "-> /id/index.html saved."

echo "Downloading Platform Pages..."
declare -a PAGES=(
  ""
  "marketplace"
  "commodities-marketplace"
  "wallet"
  "portfolio"
  "cart"
  "support"
  "settings"
  "kyc"
  "developer/dashboard"
  "developer/assets"
  "developer/add-asset"
)

for page in "${PAGES[@]}"; do
  if [ -z "$page" ]; then
    url="https://platform.poool.app/"
    dest="${BASE_DIR}/platform.poool.app/index.html"
    name="/"
  else
    url="https://platform.poool.app/${page}"
    dest="${BASE_DIR}/platform.poool.app/${page}.html"
    name="/${page}"
  fi
  
  /usr/bin/curl -s -L -b "$COOKIE" -o "$dest" "$url"
  size=$(/usr/bin/wc -c < "$dest" | /usr/bin/tr -d ' ')
  echo "-> $name saved ($size bytes)"
done

echo "Done downloading HTML pages."
