CREATE SCHEMA identity;
CREATE TABLE identity.users (
 id uuid PRIMARY KEY, email text NOT NULL UNIQUE, name text NOT NULL,
 password_hash text NOT NULL, permissions text NOT NULL, security_version text NOT NULL,
 confirmed boolean NOT NULL DEFAULT false, failures bigint NOT NULL DEFAULT 0,
 locked_until bigint NOT NULL DEFAULT 0
);
CREATE TABLE identity.sessions (id text PRIMARY KEY, data text NOT NULL, expires bigint NOT NULL);
CREATE INDEX sessions_expires ON identity.sessions(expires);
