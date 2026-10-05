CREATE SCHEMA crm;
CREATE TABLE crm.customers (
 id uuid PRIMARY KEY, code varchar(32) COLLATE "C" NOT NULL UNIQUE, name varchar(200) NOT NULL,
 search_name varchar(400) COLLATE "C" NOT NULL, email varchar(254), active boolean NOT NULL,
 version varchar(32) NOT NULL, modified_at timestamptz(6) NOT NULL, modified_by varchar(256) NOT NULL
);
CREATE INDEX customers_search_name ON crm.customers(search_name);
