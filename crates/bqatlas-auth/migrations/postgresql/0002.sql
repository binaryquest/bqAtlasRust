CREATE TABLE identity.recovery_tokens (digest text PRIMARY KEY,user_id uuid NOT NULL REFERENCES identity.users(id),purpose text NOT NULL,security_version text NOT NULL,expires bigint NOT NULL);
CREATE INDEX recovery_expires ON identity.recovery_tokens(expires);
