-- Creates a separate database for the NQRust Identity Portal
-- This runs inside the nqrust-identity-db container on first startup

CREATE DATABASE nqrust_portal;

\connect nqrust_portal

CREATE TABLE IF NOT EXISTS license_activations (
    id                SERIAL PRIMARY KEY,
    license_key       VARCHAR(100)  NOT NULL UNIQUE,
    user_id           VARCHAR(255),
    customer          VARCHAR(255),
    customer_id       VARCHAR(255),
    product           VARCHAR(255),
    product_id        VARCHAR(255),
    status            VARCHAR(50)   NOT NULL DEFAULT 'active',
    features          JSONB         NOT NULL DEFAULT '[]',
    expires_at        TIMESTAMPTZ,
    activated_at      TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    last_verified_at  TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    raw_response      JSONB
);

CREATE INDEX IF NOT EXISTS idx_license_key     ON license_activations (license_key);
CREATE INDEX IF NOT EXISTS idx_license_user_id ON license_activations (user_id);
