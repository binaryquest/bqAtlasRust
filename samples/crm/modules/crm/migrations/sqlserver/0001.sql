EXEC('CREATE SCHEMA crm');
CREATE TABLE crm.customers (
 id uniqueidentifier NOT NULL PRIMARY KEY, code nvarchar(32) COLLATE Latin1_General_100_BIN2 NOT NULL UNIQUE,
 name nvarchar(200) NOT NULL, search_name nvarchar(400) COLLATE Latin1_General_100_BIN2 NOT NULL,
 email nvarchar(254) COLLATE Latin1_General_100_BIN2 NULL, active bit NOT NULL,
 version nvarchar(32) NOT NULL, modified_at datetime2(6) NOT NULL, modified_by nvarchar(256) NOT NULL
);
CREATE INDEX customers_search_name ON crm.customers(search_name);
