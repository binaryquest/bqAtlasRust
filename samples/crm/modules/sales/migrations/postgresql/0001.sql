CREATE SCHEMA sales;
CREATE TABLE sales.quotes (
 id uuid PRIMARY KEY, number varchar(40) NOT NULL UNIQUE, customer_id uuid NOT NULL,
 customer_code varchar(32) NOT NULL, customer_name varchar(200) NOT NULL, day date NOT NULL,
 currency varchar(3) NOT NULL, status varchar(16) NOT NULL, total numeric(20,2) NOT NULL,
 version varchar(32) NOT NULL, modified_at timestamptz(6) NOT NULL, modified_by varchar(256) NOT NULL,
 submitted_at varchar(40)
);
CREATE TABLE sales.lines (
 quote_id uuid NOT NULL REFERENCES sales.quotes(id) ON DELETE CASCADE, id uuid NOT NULL,
 position bigint NOT NULL, description varchar(200) NOT NULL, quantity numeric(12,3) NOT NULL,
 unit_price numeric(16,4) NOT NULL, total numeric(20,2) NOT NULL, PRIMARY KEY(quote_id,id)
);
CREATE TABLE sales.operations (id uuid PRIMARY KEY,quote_id uuid NOT NULL,operation varchar(32) NOT NULL,actor varchar(256) NOT NULL,occurred_at timestamptz(6) NOT NULL,version varchar(32) NOT NULL);
CREATE INDEX operations_quote ON sales.operations(quote_id);
CREATE TABLE sales.submissions (quote_id uuid NOT NULL,actor varchar(256) NOT NULL,key varchar(32) NOT NULL,version varchar(32) NOT NULL,PRIMARY KEY(quote_id,actor,key));
