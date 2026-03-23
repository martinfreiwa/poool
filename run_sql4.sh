#!/bin/bash
gcloud run jobs create db-fix-job4 \
  --image=gcr.io/google.com/cloudsdktool/google-cloud-cli:slim \
  --command="bash" \
  --args="-c","apt-get update && apt-get install -y postgresql-client && PGPASSWORD='Tasse3765!poool' psql -h /cloudsql/my-project-35266-489713:europe-west1:poool-db -U postgres -d poool -c '\d sessions;'" \
  --set-cloudsql-instances=my-project-35266-489713:europe-west1:poool-db \
  --region=europe-west1 \
  --project=my-project-35266-489713 \
  --execute-now
