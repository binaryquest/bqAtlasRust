EXEC('CREATE SCHEMA [identity]');
CREATE TABLE [identity].users (
 id uniqueidentifier NOT NULL PRIMARY KEY, email nvarchar(254) COLLATE Latin1_General_100_BIN2 NOT NULL UNIQUE,
 name nvarchar(200) NOT NULL, password_hash nvarchar(512) NOT NULL, permissions nvarchar(max) NOT NULL,
 security_version nvarchar(32) NOT NULL, confirmed bit NOT NULL DEFAULT 0,
 failures bigint NOT NULL DEFAULT 0, locked_until bigint NOT NULL DEFAULT 0
);
CREATE TABLE [identity].sessions (id nvarchar(128) NOT NULL PRIMARY KEY, data nvarchar(max) NOT NULL, expires bigint NOT NULL);
CREATE INDEX sessions_expires ON [identity].sessions(expires);
