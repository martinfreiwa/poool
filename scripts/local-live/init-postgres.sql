CREATE EXTENSION IF NOT EXISTS pgcrypto;

SELECT 'CREATE DATABASE poool_community'
WHERE NOT EXISTS (
  SELECT 1 FROM pg_database WHERE datname = 'poool_community'
)\gexec

\connect poool_community

CREATE EXTENSION IF NOT EXISTS pgcrypto;
GRANT ALL PRIVILEGES ON DATABASE poool_community TO poool;
