CREATE TABLE [identity].recovery_tokens (digest varchar(64) COLLATE Latin1_General_100_BIN2 PRIMARY KEY,user_id uniqueidentifier NOT NULL REFERENCES [identity].users(id),purpose varchar(16) NOT NULL,security_version varchar(32) COLLATE Latin1_General_100_BIN2 NOT NULL,expires bigint NOT NULL);
CREATE INDEX recovery_expires ON [identity].recovery_tokens(expires);
