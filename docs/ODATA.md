# Bounded OData read adapter

The alpha provides `GET /odata/Customers`, `/Quotes`, `/Products`, `/Opportunities`, and `/Activities`. Each enforces the same resource read permission as REST and uses only its module's public projection. Properties use PascalCase; decimal values and exact counts are strings. REST retains record reads, ETags, writes, lookups and business commands.

Supported query options:

| Option | Accepted subset |
| --- | --- |
| `$top` | 0–100, default 25 |
| `$skip` | 0–100000, default 0; arbitrary offsets |
| `$count` | `true` or `false`; exact `@odata.count` when true |
| `$orderby` | Up to five approved properties, `asc`/`desc`; stable ID tie-breaker |
| `$filter` | Typed `eq`, `contains`, `startswith`, parentheses, `and`, `or` |

Strings are single-quoted; double a quote to represent an apostrophe. Booleans, UUIDs, dates and decimals are unquoted. Equality also supports `null`. Names are case-sensitive and limited to each resource's public field map. Text comparison uses the framework's invariant uppercase normalization and literal wildcard escaping. Expression length, text length, term count and nesting are bounded. Duplicate query options and unsupported syntax fail with a 400 Problem Details response.

```text
/odata/Customers?$count=true&$top=25&$skip=0&$orderby=Code asc,Id asc&$filter=(contains(Code,'NORTH') or contains(Name,'NORTH')) and Active eq true
/odata/Products?$count=true&$filter=Category eq 'Service' and UnitPrice eq 125.0000
```

`Customers` exposes Id, Code, Name, Email, Active, ModifiedAt. `Quotes` exposes its summary (Id, Number, CustomerId, CustomerName, Date, Currency, Status, Total), without child lines or persistence metadata. Engagement resources expose the corresponding REST public fields. Internal audit actor, search columns and versions are excluded.

The shared Angular `ODataResourceProvider` can use these endpoints with PascalCase field maps and a PascalCase-to-application mapper. Its record provider remains the REST provider. HTTP integration tests cover all five sets, grouping, typed values, apostrophes, exact count, permissions and rejected options; the CRM demo defaults to REST.

This is a bounded query adapter, **not full OData conformance**. No CSDL `$metadata`, service document, continuation link, `$select`, `$expand`, `$apply`, `$batch`, relational navigation, writes or arbitrary expressions are advertised. Such requests fail explicitly. Broader protocol support can be added independently of the domain and REST stores.
