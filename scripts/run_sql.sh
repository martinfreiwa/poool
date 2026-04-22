#!/bin/bash
gcloud run jobs create db-fix-job \
  --image=gcr.io/google.com/cloudsdktool/google-cloud-cli:slim \
  --command="bash" \
  --args="-c","apt-get update && apt-get install -y postgresql-client && PGPASSWORD='Tasse3765!poool' psql -h 127.0.0.1 -p 5432 -U postgres -d poool -c \"UPDATE assets SET chain_contract_address = NULL, chain_tx_hash = NULL, chain_token_id = NULL WHERE id = '15a8138f-69d4-4284-9e92-9e08af4c68e2';\"" \
  --set-cloudsql-instances=my-project-35266-489713:europe-west1:poool-db \
  --region=europe-west1 \
  --project=my-project-35266-489713 \
  --execute-now
