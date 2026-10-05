EXEC('CREATE SCHEMA sales');
CREATE TABLE sales.quotes (
 id uniqueidentifier PRIMARY KEY, number nvarchar(40) NOT NULL UNIQUE, customer_id uniqueidentifier NOT NULL,
 customer_code nvarchar(32) NOT NULL, customer_name nvarchar(200) NOT NULL, day date NOT NULL,
 currency nvarchar(3) NOT NULL, status nvarchar(16) NOT NULL, total decimal(20,2) NOT NULL,
 version nvarchar(32) NOT NULL, modified_at datetime2(6) NOT NULL, modified_by nvarchar(256) NOT NULL,
 submitted_at nvarchar(40) NULL
);
CREATE TABLE sales.lines (
 quote_id uniqueidentifier NOT NULL REFERENCES sales.quotes(id) ON DELETE CASCADE, id uniqueidentifier NOT NULL,
 position bigint NOT NULL, description nvarchar(200) NOT NULL, quantity decimal(12,3) NOT NULL,
 unit_price decimal(16,4) NOT NULL, total decimal(20,2) NOT NULL, PRIMARY KEY(quote_id,id)
);
CREATE TABLE sales.operations (id uniqueidentifier PRIMARY KEY,quote_id uniqueidentifier NOT NULL,operation nvarchar(32) NOT NULL,actor nvarchar(256) NOT NULL,occurred_at datetime2(6) NOT NULL,version nvarchar(32) NOT NULL);
CREATE INDEX operations_quote ON sales.operations(quote_id);
CREATE TABLE sales.submissions (quote_id uniqueidentifier NOT NULL,actor nvarchar(256) NOT NULL,[key] nvarchar(32) NOT NULL,version nvarchar(32) NOT NULL,PRIMARY KEY(quote_id,actor,[key]));
