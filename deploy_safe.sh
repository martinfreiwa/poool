#!/bin/bash
set -e

echo "🚀 Deploying new revision without traffic..."
gcloud run deploy poool-backend \
    --source . \
    --region europe-west1 \
    --project my-project-35266-489713 \
    --allow-unauthenticated \
    --no-traffic \
    --tag staging

echo "🔍 Fetching staging URL..."
sleep 3
STAGING_URL=$(gcloud run services describe poool-backend --region europe-west1 --project my-project-35266-489713 --format="json" | grep -o '"url": "[^"]*"' | grep 'staging---' | cut -d'"' -f4 | head -n 1)

if [ -z "$STAGING_URL" ]; then
    echo "❌ Could not parse staging URL."
    exit 1
fi

echo "🏥 Checking health at $STAGING_URL/health"
for i in {1..3}; do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$STAGING_URL/health" || echo "failed")
    if [ "$STATUS" = "200" ]; then
        break
    fi
    echo "Attempt $i: Status $STATUS... retrying in 5 seconds."
    sleep 5
done

if [ "$STATUS" = "200" ]; then
    echo "✅ Health check passed (HTTP 200). Routing 100% traffic to new staging revision..."
    gcloud run services update-traffic poool-backend \
        --region europe-west1 \
        --project my-project-35266-489713 \
        --to-tags staging=100
    echo "🎉 Deployment completely successful."
else
    echo "🧯 Health check final failure (HTTP $STATUS)! Traffic NOT updated."
    exit 1
fi
