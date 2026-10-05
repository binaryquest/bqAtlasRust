//! Real-engine HTTP qualification. Explicitly opt in with BQATLAS_TEST_DATABASE_URL.
use bqatlas_auth::{AuthState, DatabaseSessionStore};
use bqatlas_db::{Database, Sql, Value};
use chrono::{NaiveDate, SubsecRound, Utc};
use futures_util::FutureExt;
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value as JsonValue, json};
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
};
use uuid::Uuid;

mod auth_flows;

struct Browser {
    client: Client,
    base: String,
    csrf: String,
}
impl Browser {
    async fn new(base: &str) -> Self {
        let mut browser = Self {
            client: Client::builder()
                .cookie_store(true)
                .build()
                .expect("client"),
            base: base.into(),
            csrf: String::new(),
        };
        browser.refresh_csrf().await;
        browser
    }
    async fn refresh_csrf(&mut self) {
        let body: JsonValue = self
            .client
            .get(format!("{}/api/v1/session/csrf", self.base))
            .send()
            .await
            .expect("csrf response")
            .error_for_status()
            .expect("csrf status")
            .json()
            .await
            .expect("csrf json");
        self.csrf = body["token"].as_str().expect("token").into();
    }
    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<JsonValue>,
        version: Option<&str>,
    ) -> reqwest::Response {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.base))
            .header("X-BQATLAS-CSRF", &self.csrf);
        if let Some(body) = body {
            request = request.json(&body);
        }
        if let Some(version) = version {
            request = request.header("If-Match", format!("\"{version}\""));
        }
        request.send().await.expect("HTTP response")
    }
    async fn login(&mut self, email: &str, password: &str) {
        let response = self
            .call(
                Method::POST,
                "/auth/login",
                Some(json!({"email":email,"password":password})),
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        self.refresh_csrf().await;
    }
}
async fn qualification(db: Database) {
    db.migrate(&bqatlas_server::migrations())
        .await
        .expect("migrations");
    db.migrate(&bqatlas_server::migrations())
        .await
        .expect("idempotent migrations");
    db.check_migrations(&bqatlas_server::migrations())
        .await
        .expect("schema check");
    let mut auth = AuthState::new(db.clone()).await.expect("auth");
    let password = "Strong-Rust!123";
    assert!(
        auth.bootstrap(
            "Development",
            "admin@example.test",
            password,
            &bqatlas_server::permissions()
        )
        .await
        .expect("bootstrap")
    );
    assert!(
        !auth
            .bootstrap("Development", "admin@example.test", "Other-Rust!123", &[])
            .await
            .expect("preserve existing user")
    );
    assert!(
        auth.bootstrap("Production", "production@example.test", password, &[])
            .await
            .is_err()
    );
    auth.bootstrap(
        "Development",
        "reader@example.test",
        password,
        &["crm.customers.read".into(), "crm.customers.lookup".into()],
    )
    .await
    .expect("reader");
    auth.bootstrap(
        "Development",
        "lookup@example.test",
        password,
        &["crm.customers.lookup".into()],
    )
    .await
    .expect("lookup user");
    let mail = std::sync::Arc::new(auth_flows::TestMailer::default());
    auth.recovery = Some(
        bqatlas_auth::recovery::RecoveryConfig::new(mail.clone(), "http://127.0.0.1:5201", true)
            .expect("mail configuration"),
    );
    auth.bootstrap("Development", "recovery@example.test", password, &[])
        .await
        .expect("recovery user");
    let app = bqatlas_server::application_with_auth(db.clone(), false, auth)
        .await
        .expect("application");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let base = format!("http://{}", listener.local_addr().expect("address"));
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .expect("server")
    });
    let mut admin = Browser::new(&base).await;
    assert_eq!(
        admin
            .call(Method::GET, "/api/v1/manifest", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let no_csrf = Client::new()
        .post(format!("{base}/auth/login"))
        .json(&json!({"email":"admin@example.test","password":password}))
        .send()
        .await
        .expect("missing csrf");
    assert_eq!(no_csrf.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        no_csrf.json::<JsonValue>().await.expect("problem")["code"],
        "csrf_failed"
    );
    admin.login("admin@example.test", password).await;
    let session: JsonValue = admin
        .call(Method::GET, "/api/v1/session", None, None)
        .await
        .json()
        .await
        .expect("session");
    assert_eq!(session["authenticated"], true);
    assert_eq!(session["authMode"], "local");
    let manifest: JsonValue = admin
        .call(Method::GET, "/api/v1/manifest", None, None)
        .await
        .json()
        .await
        .expect("manifest");
    assert_eq!(manifest["contractVersion"], "1.0");
    assert_eq!(manifest["resources"][0]["id"], "crm.customers");
    assert_eq!(
        manifest["resources"][0]["oDataEndpoint"],
        "/odata/Customers"
    );
    let malformed_path = admin
        .call(Method::GET, "/api/v1/crm/customers/not-a-guid", None, None)
        .await;
    assert_eq!(malformed_path.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        malformed_path
            .headers()
            .get("content-type")
            .expect("problem content type"),
        "application/problem+json"
    );
    assert_eq!(
        malformed_path
            .headers()
            .get("cache-control")
            .expect("no cache"),
        "no-store"
    );
    crm_workflow(&db, &admin).await;
    if std::env::var("BQATLAS_TEST_EXTRA_RESOURCE").is_ok() {
        generated_directory(&admin).await;
    }
    let path = "/api/v1/crm/customers";
    let invalid = admin
        .call(
            Method::POST,
            path,
            Some(json!({"code":"bad code","name":"","email":"wrong","active":true})),
            None,
        )
        .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let invalid: JsonValue = invalid.json().await.expect("validation");
    assert!(invalid["errors"]["code"].is_array());
    assert!(invalid["errors"]["name"].is_array());
    let response=admin.call(Method::POST,path,Some(json!({"code":" ac-01 ","name":"Alpha %_[ Café","email":"alpha@example.test","active":true,"version":"client-owned"})),None).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let etag = response.headers()["etag"]
        .to_str()
        .expect("etag")
        .to_owned();
    let customer: JsonValue = response.json().await.expect("customer");
    let id = customer["data"]["id"].as_str().expect("id");
    let version = customer["version"].as_str().expect("version");
    assert_eq!(etag, format!("\"{version}\""));
    assert_eq!(customer["data"]["code"], "AC-01");
    assert!(
        !customer["data"]
            .as_object()
            .expect("dto")
            .contains_key("modifiedBy")
    );
    let record_path = format!("{path}/{id}");
    let duplicate = admin
        .call(
            Method::POST,
            path,
            Some(json!({"code":"AC-01","name":"Duplicate","active":true})),
            None,
        )
        .await;
    assert_eq!(duplicate.status(), StatusCode::BAD_REQUEST);
    for search in ["%_[", "CAFÉ", "ac-01"] {
        let response = admin
            .call(
                Method::POST,
                &format!("{path}/query"),
                Some(json!({"page":0,"pageSize":25,"search":search})),
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.json::<JsonValue>().await.expect("page")["total"],
            1
        );
    }
    let response = admin
        .call(
            Method::POST,
            &format!("{path}/query"),
            Some(json!({"sort":[{"field":"name; DROP TABLE crm.customers","direction":"asc"}]})),
            None,
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let input = json!({"code":"AC-01","name":"Updated","email":null,"active":true});
    assert_eq!(
        admin
            .call(Method::PUT, &record_path, Some(input.clone()), None)
            .await
            .status(),
        StatusCode::PRECONDITION_REQUIRED
    );
    let stale = "00000000000000000000000000000000";
    assert_eq!(
        admin
            .call(Method::PUT, &record_path, Some(input.clone()), Some(stale))
            .await
            .status(),
        StatusCode::PRECONDITION_FAILED
    );
    let (one, two) = tokio::join!(
        admin.call(
            Method::PUT,
            &record_path,
            Some(input.clone()),
            Some(version)
        ),
        admin.call(
            Method::PUT,
            &record_path,
            Some(input.clone()),
            Some(version)
        )
    );
    assert!(
        (one.status() == StatusCode::OK && two.status() == StatusCode::PRECONDITION_FAILED)
            || (two.status() == StatusCode::OK && one.status() == StatusCode::PRECONDITION_FAILED)
    );
    let current: JsonValue = admin
        .call(Method::GET, &record_path, None, None)
        .await
        .json()
        .await
        .expect("record");
    let version = current["version"].as_str().expect("version");
    let mut reader = Browser::new(&base).await;
    reader.login("reader@example.test", password).await;
    assert_eq!(
        reader
            .call(Method::POST, path, Some(input.clone()), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut lookup = Browser::new(&base).await;
    lookup.login("lookup@example.test", password).await;
    let manifest: JsonValue = lookup
        .call(Method::GET, "/api/v1/manifest", None, None)
        .await
        .json()
        .await
        .expect("manifest");
    assert_eq!(
        manifest["resources"].as_array().expect("resources").len(),
        0
    );
    assert_eq!(
        lookup
            .call(Method::GET, &record_path, None, None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let projection: JsonValue = lookup
        .call(Method::GET, &format!("{path}/lookup/{id}"), None, None)
        .await
        .json()
        .await
        .expect("lookup");
    assert_eq!(projection["code"], "AC-01");
    assert!(
        !projection
            .as_object()
            .expect("projection")
            .contains_key("email")
    );
    let inactive = admin
        .call(
            Method::PUT,
            &record_path,
            Some(json!({"code":"AC-01","name":"Inactive","active":false})),
            Some(version),
        )
        .await;
    assert_eq!(inactive.status(), StatusCode::OK);
    let inactive: JsonValue = inactive.json().await.expect("inactive");
    assert!(
        lookup
            .call(Method::GET, &format!("{path}/lookup/{id}"), None, None)
            .await
            .json::<JsonValue>()
            .await
            .expect("null")
            .is_null()
    );
    assert_eq!(
        admin
            .call(Method::DELETE, &record_path, None, Some(version))
            .await
            .status(),
        StatusCode::PRECONDITION_FAILED
    );
    assert_eq!(
        admin
            .call(
                Method::DELETE,
                &record_path,
                None,
                inactive["version"].as_str()
            )
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        admin
            .call(Method::GET, &record_path, None, None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let mut other_session = Browser::new(&base).await;
    other_session.login("admin@example.test", password).await;
    let bad = admin
        .call(
            Method::POST,
            "/auth/change-password",
            Some(json!({"currentPassword":"wrong","newPassword":"New-Strong!123"})),
            None,
        )
        .await;
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        admin
            .call(
                Method::POST,
                "/auth/change-password",
                Some(json!({"currentPassword":password,"newPassword":"New-Strong!123"})),
                None
            )
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        other_session
            .call(Method::GET, "/api/v1/manifest", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    admin.refresh_csrf().await;
    assert_eq!(
        admin
            .call(
                Method::POST,
                "/auth/login",
                Some(json!({"email":"admin@example.test","password":password})),
                None
            )
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    admin.login("admin@example.test", "New-Strong!123").await;
    assert_eq!(
        admin
            .call(Method::GET, "/health/ready", None, None)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        admin
            .client
            .get(format!("{base}/api/v1/manifest"))
            .header("Authorization", "Bearer invalid")
            .send()
            .await
            .expect("bearer")
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        admin
            .call(Method::POST, "/auth/logout", None, None)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        admin
            .call(Method::GET, "/api/v1/manifest", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    // Storage cannot resurrect a logged-out session when a parallel older response saves it.
    let store = DatabaseSessionStore(db.clone());
    let mut record = Record {
        id: Id::default(),
        data: Default::default(),
        expiry_date: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    store.create(&mut record).await.expect("create session");
    store.delete(&record.id).await.expect("delete");
    store.save(&record).await.expect("old save");
    assert!(store.load(&record.id).await.expect("load").is_none());
    // Real driver round trips and transaction rollback, including exact decimal values.
    let mut tx = db.begin().await.expect("transaction");
    tx.execute(Sql::new("CREATE TABLE bqatlas.driver_probe(id uuid PRIMARY KEY, amount numeric(20,4), day date, happened timestamptz(6))","CREATE TABLE bqatlas.driver_probe(id uniqueidentifier PRIMARY KEY, amount decimal(20,4), day date, happened datetime2(6))"),&[]).await.expect("probe table");
    let id = Uuid::new_v4();
    let amount = "9007199254740993.1255".parse().expect("decimal");
    let date = NaiveDate::from_ymd_opt(2026, 9, 30).expect("date");
    let now = Utc::now().trunc_subsecs(6);
    tx.execute(
        Sql::new(
            "INSERT INTO bqatlas.driver_probe VALUES($1,$2,$3,$4)",
            "INSERT INTO bqatlas.driver_probe VALUES(@P1,@P2,@P3,@P4)",
        ),
        &[
            Value::Uuid(id),
            Value::Decimal(amount),
            Value::Date(date),
            Value::Timestamp(now),
        ],
    )
    .await
    .expect("probe insert");
    let rows = tx
        .query(
            Sql::new(
                "SELECT * FROM bqatlas.driver_probe",
                "SELECT * FROM bqatlas.driver_probe",
            ),
            &[],
        )
        .await
        .expect("probe read");
    assert_eq!(rows[0].uuid("id").expect("UUID"), id);
    assert_eq!(rows[0].decimal("amount").expect("decimal"), amount);
    assert_eq!(rows[0].date("day").expect("date"), date);
    assert_eq!(rows[0].timestamp("happened").expect("timestamp"), now);
    tx.rollback().await.expect("rollback");
    if db.provider() == "sqlserver" {
        let timed = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            db.query(
                Sql::new(
                    "SELECT 1::bigint AS healthy",
                    "WAITFOR DELAY '00:00:02'; SELECT CAST(1 AS bigint) AS healthy",
                ),
                &[],
            ),
        )
        .await;
        assert!(timed.is_err());
        assert_eq!(
            db.query(
                Sql::new(
                    "SELECT 1::bigint AS healthy",
                    "SELECT CAST(1 AS bigint) AS healthy"
                ),
                &[]
            )
            .await
            .expect("clean connection")[0]
                .integer("healthy")
                .expect("healthy"),
            1
        );
    }
    auth_flows::recovery(&db, &base, mail).await;
    server.abort();
    if std::env::var("BQATLAS_TEST_OIDC_ISSUER").is_ok() {
        auth_flows::oidc(db).await;
    }
}
#[tokio::test]
#[ignore = "requires disposable real database; set BQATLAS_TEST_DATABASE_URL and run --ignored"]
async fn real_database_and_http_contracts() {
    let provider = std::env::var("BQATLAS_TEST_PROVIDER").unwrap_or_else(|_| "postgresql".into());
    let connection =
        std::env::var("BQATLAS_TEST_DATABASE_URL").expect("BQATLAS_TEST_DATABASE_URL is required");
    let admin = Database::connect(&provider, &connection, 2)
        .await
        .expect("test database server");
    let name = format!("bqatlas_rust_test_{}", Uuid::new_v4().simple());
    assert!(
        name.starts_with("bqatlas_rust_test_")
            && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
    );
    admin
        .execute(
            Sql::new(
                &format!("CREATE DATABASE {name}"),
                &format!("CREATE DATABASE [{name}]"),
            ),
            &[],
        )
        .await
        .expect("create isolated database");
    let target = if provider == "postgresql" {
        let mut url = reqwest::Url::parse(&connection).expect("database URL");
        url.set_path(&name);
        url.to_string()
    } else {
        let parts: Vec<_> = connection
            .split(';')
            .filter(|p| {
                !p.split('=').next().is_some_and(|k| {
                    matches!(
                        k.trim().to_lowercase().as_str(),
                        "database" | "initial catalog"
                    )
                })
            })
            .collect();
        format!("{};Database={name}", parts.join(";"))
    };
    let database = Database::connect(&provider, &target, 10)
        .await
        .expect("isolated database");
    let result = std::panic::AssertUnwindSafe(qualification(database))
        .catch_unwind()
        .await;
    let drop_sql = if provider == "postgresql" {
        format!("DROP DATABASE {name} WITH (FORCE)")
    } else {
        format!(
            "ALTER DATABASE [{name}] SET SINGLE_USER WITH ROLLBACK IMMEDIATE; DROP DATABASE [{name}]"
        )
    };
    admin
        .execute(Sql::new(&drop_sql, &drop_sql), &[])
        .await
        .expect("cleanup isolated database");
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

async fn crm_workflow(db: &Database, admin: &Browser) {
    bqatlas_server::seed::seed(db.clone(), "Development")
        .await
        .expect("seed CRM");
    bqatlas_server::seed::seed(db.clone(), "Development")
        .await
        .expect("repeat CRM seed");
    assert!(
        bqatlas_server::seed::seed(db.clone(), "Production")
            .await
            .is_err()
    );
    for (resource, total) in [("products", 5), ("opportunities", 6), ("activities", 5)] {
        let path = format!("/api/v1/engagement/{resource}/query");
        let response = admin
            .call(Method::POST, &path, Some(json!({"pageSize":100})), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.json::<JsonValue>().await.expect("CRM page")["total"],
            total
        );
    }
    let customer_page: JsonValue = admin
        .call(
            Method::POST,
            "/api/v1/crm/customers/query",
            Some(json!({"search":"DEMO-NORTH"})),
            None,
        )
        .await
        .json()
        .await
        .expect("seeded customer");
    let customer_id = customer_page["items"][0]["id"]
        .as_str()
        .expect("customer ID");
    let product=admin.call(Method::POST,"/api/v1/engagement/products",Some(json!({"name":"Test product","code":"TEST-SKU","category":"Service","unitPrice":"12.1255","currency":"USD","active":true})),None).await;
    assert_eq!(product.status(), StatusCode::CREATED);
    let product: JsonValue = product.json().await.expect("product");
    assert_eq!(product["data"]["unitPrice"], "12.1255");
    let product_id = product["data"]["id"].as_str().expect("product id");
    let product_path = format!("/api/v1/engagement/products/{product_id}");
    assert_eq!(admin.call(Method::PUT,&product_path,Some(json!({"name":"Product edited","code":"TEST-SKU","category":"Service","unitPrice":"12.1255","currency":"USD","active":true})),Some("00000000000000000000000000000000")).await.status(),StatusCode::PRECONDITION_FAILED);
    let product_lookup: JsonValue = admin
        .call(
            Method::GET,
            &format!("/api/v1/engagement/products/lookup/{product_id}"),
            None,
            None,
        )
        .await
        .json()
        .await
        .expect("product lookup");
    assert_eq!(product_lookup["code"], "TEST-SKU");
    let activity=admin.call(Method::POST,"/api/v1/engagement/activities",Some(json!({"name":"HTTP follow-up","customerId":customer_id,"dueDate":"2026-10-01","kind":"Call","status":"Planned","owner":"Tester","notes":"Notes","customerName":"Client injected"})),None).await;
    assert_eq!(activity.status(), StatusCode::CREATED);
    let activity: JsonValue = activity.json().await.expect("activity");
    assert_eq!(activity["data"]["customerName"], "Northwind Services");
    let invalid=admin.call(Method::POST,"/api/v1/engagement/opportunities",Some(json!({"name":"Bad stage","customerId":customer_id,"stage":"Hacked","amount":"100","currency":"USD","expectedClose":"2026-10-01","owner":"Tester","notes":""})),None).await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let input = json!({"customerId":customer_id,"date":"2026-09-30","currency":"USD","lines":[{"id":Uuid::new_v4(),"description":"First","quantity":"1","unitPrice":"0.005"},{"id":Uuid::new_v4(),"description":"Second","quantity":"1","unitPrice":"0.005"}]});
    let quote = admin
        .call(
            Method::POST,
            "/api/v1/sales/quotes",
            Some(input.clone()),
            None,
        )
        .await;
    assert_eq!(quote.status(), StatusCode::CREATED);
    let quote: JsonValue = quote.json().await.expect("quote");
    assert_eq!(quote["data"]["total"], "0.02");
    let id = quote["data"]["id"].as_str().expect("quote id");
    let version = quote["version"].as_str().expect("version");
    let key = Uuid::new_v4().to_string();
    let submit_path = format!("{}/api/v1/sales/quotes/{id}/submit", admin.base);
    let request = || {
        admin
            .client
            .post(&submit_path)
            .header("X-BQATLAS-CSRF", &admin.csrf)
            .header("If-Match", format!("\"{version}\""))
            .header("Idempotency-Key", &key)
            .send()
    };
    let (one, two) = tokio::join!(request(), request());
    let one = one.expect("submit one");
    let two = two.expect("submit two");
    assert_eq!(one.status(), StatusCode::OK);
    assert_eq!(two.status(), StatusCode::OK);
    let submitted: JsonValue = one.json().await.expect("submitted");
    let replay: JsonValue = two.json().await.expect("replay");
    assert_eq!(submitted["version"], replay["version"]);
    assert_eq!(submitted["data"]["status"], "submitted");
    let quote_path = format!("/api/v1/sales/quotes/{id}");
    assert_eq!(
        admin
            .call(
                Method::PUT,
                &quote_path,
                Some(input),
                submitted["version"].as_str()
            )
            .await
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        admin
            .call(
                Method::DELETE,
                &quote_path,
                None,
                submitted["version"].as_str()
            )
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let receipts = db
        .query(
            Sql::new(
                "SELECT COUNT(*)::bigint AS count FROM sales.submissions WHERE quote_id=$1",
                "SELECT COUNT_BIG(*) AS count FROM sales.submissions WHERE quote_id=@P1",
            ),
            &[Value::Uuid(Uuid::parse_str(id).expect("id"))],
        )
        .await
        .expect("receipts");
    assert_eq!(receipts[0].integer("count").expect("count"), 1);
    odata_qualification(admin).await;
}

async fn odata_qualification(admin: &Browser) {
    for (resource, filter, key) in [
        (
            "Customers",
            "(contains(Code,'DEMO-NORTH') or contains(Name,'DEMO-NORTH')) and Active eq true",
            "Code",
        ),
        (
            "Products",
            "Code eq 'TEST-SKU' and Active eq true",
            "UnitPrice",
        ),
        ("Quotes", "Total eq 0.02 and Status eq 'submitted'", "Total"),
        ("Opportunities", "Stage eq 'Lead'", "Stage"),
        ("Activities", "Kind eq 'Call'", "Kind"),
    ] {
        let query = reqwest::Url::parse_with_params(
            &format!("{}/odata/{resource}", admin.base),
            &[
                ("$count", "true"),
                ("$top", "25"),
                ("$skip", "0"),
                ("$orderby", "Id asc"),
                ("$filter", filter),
            ],
        )
        .expect("query URL");
        let response = admin
            .client
            .get(query)
            .header("Accept", "application/json;IEEE754Compatible=true")
            .send()
            .await
            .expect("OData read");
        assert_eq!(response.status(), StatusCode::OK, "OData {resource}");
        let value: JsonValue = response.json().await.expect("OData JSON");
        assert!(
            value["@odata.count"]
                .as_str()
                .expect("exact count")
                .parse::<u64>()
                .expect("count")
                > 0
        );
        assert!(!value["value"][0][key].is_null());
        assert!(value["value"][0]["ModifiedBy"].is_null());
        assert!(value["value"][0]["Version"].is_null());
    }
    let url = reqwest::Url::parse_with_params(
        &format!("{}/odata/Customers", admin.base),
        &[("$filter", "Name eq 'O''Brien%'"), ("$count", "true")],
    )
    .expect("literal URL");
    assert_eq!(
        admin
            .client
            .get(url)
            .send()
            .await
            .expect("literal query")
            .status(),
        StatusCode::OK
    );
    for query in [
        "$expand=Customer",
        "$filter=PasswordHash%20eq%20'secret'",
        "$orderby=Name;DROP",
        "$top=101",
        "$count=true&$top=1&$top=2",
    ] {
        assert_eq!(
            admin
                .client
                .get(format!("{}/odata/Customers?{query}", admin.base))
                .send()
                .await
                .expect("rejected query")
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
}

async fn generated_directory(admin: &Browser) {
    let resource = std::env::var("BQATLAS_TEST_EXTRA_RESOURCE").expect("extra resource");
    let (module, set) = resource.split_once('.').expect("module.resource");
    assert!(module.bytes().all(|c| c.is_ascii_lowercase() || c == b'_'));
    assert!(set.bytes().all(|c| c.is_ascii_lowercase() || c == b'_'));
    let endpoint = format!("/api/v1/{module}/{set}");
    let input = json!({"code":"GEN-ONE","name":"Generated warehouse","email":null,"active":true});
    let created = admin
        .call(Method::POST, &endpoint, Some(input.clone()), None)
        .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: JsonValue = created.json().await.expect("created");
    let path = format!("{endpoint}/{}", created["data"]["id"].as_str().expect("ID"));
    let old = created["version"].as_str().expect("version");
    assert_eq!(
        admin
            .call(Method::PUT, &path, Some(input.clone()), None)
            .await
            .status(),
        StatusCode::PRECONDITION_REQUIRED
    );
    let updated=admin.call(Method::PUT,&path,Some(json!({"code":"GEN-ONE","name":"Updated warehouse","email":"owner@example.test","active":true})),Some(old)).await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated: JsonValue = updated.json().await.expect("updated");
    assert_eq!(
        admin
            .call(Method::DELETE, &path, None, Some(old))
            .await
            .status(),
        StatusCode::PRECONDITION_FAILED
    );
    let page = admin
        .call(
            Method::POST,
            &format!("{endpoint}/query"),
            Some(json!({"search":"Updated"})),
            None,
        )
        .await;
    assert_eq!(page.status(), StatusCode::OK);
    assert_eq!(page.json::<JsonValue>().await.expect("page")["total"], 1);
    assert_eq!(
        admin
            .call(Method::DELETE, &path, None, updated["version"].as_str())
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
}
