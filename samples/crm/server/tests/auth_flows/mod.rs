use super::*;
use async_trait::async_trait;
use bqatlas_auth::{
    OidcConfig, OidcProvider,
    recovery::{AccountEmail, AccountEmailSender},
};
use bqatlas_core::AppError;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Default)]
pub struct TestMailer(tokio::sync::Mutex<Vec<AccountEmail>>);
#[async_trait]
impl AccountEmailSender for TestMailer {
    async fn send(&self, mail: AccountEmail) -> Result<(), AppError> {
        self.0.lock().await.push(mail);
        Ok(())
    }
}
pub async fn recovery(db: &Database, base: &str, mail: Arc<TestMailer>) {
    let mut browser = Browser::new(base).await;
    browser
        .login("recovery@example.test", "Strong-Rust!123")
        .await;
    for address in ["recovery@example.test", "missing@example.test"] {
        let response = browser
            .call(
                Method::POST,
                "/auth/request-password-reset",
                Some(json!({"email":address})),
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(
            response.json::<JsonValue>().await.expect("ack")["message"],
            "If the account is eligible, an email has been sent."
        );
    }
    let messages = mail.0.lock().await;
    assert_eq!(messages.len(), 1);
    let url = reqwest::Url::parse(&messages[0].action_url).expect("recovery URL");
    let pairs: BTreeMap<_, _> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    drop(messages);
    let input =
        json!({"userId":pairs["userId"],"token":pairs["token"],"newPassword":"Recovered-Rust!123"});
    let token_rows = db
        .query(
            Sql::new(
                "SELECT digest FROM identity.recovery_tokens",
                "SELECT digest FROM [identity].recovery_tokens",
            ),
            &[],
        )
        .await
        .expect("hash");
    assert_ne!(
        token_rows[0].text("digest").expect("digest"),
        pairs["token"]
    );
    assert_eq!(
        browser
            .call(
                Method::POST,
                "/auth/confirm-email",
                Some(input.clone()),
                None
            )
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        browser
            .call(
                Method::POST,
                "/auth/reset-password",
                Some(input.clone()),
                None
            )
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    browser.refresh_csrf().await;
    assert_eq!(
        browser
            .call(Method::POST, "/auth/reset-password", Some(input), None)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    browser
        .login("recovery@example.test", "Recovered-Rust!123")
        .await;
    // Confirmation cannot be used as password reset, and changes the security stamp.
    let id = Uuid::parse_str(&pairs["userId"]).expect("id");
    db.execute(
        Sql::new(
            "UPDATE identity.users SET confirmed=false WHERE id=$1",
            "UPDATE [identity].users SET confirmed=0 WHERE id=@P1",
        ),
        &[Value::Uuid(id)],
    )
    .await
    .expect("unconfirmed test account");
    browser.refresh_csrf().await;
    assert_eq!(
        browser
            .call(
                Method::POST,
                "/auth/request-confirmation",
                Some(json!({"email":"recovery@example.test"})),
                None
            )
            .await
            .status(),
        StatusCode::ACCEPTED
    );
    let messages = mail.0.lock().await;
    assert_eq!(messages.len(), 2);
    let url = reqwest::Url::parse(&messages[1].action_url).expect("confirmation URL");
    let pairs: BTreeMap<_, _> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    drop(messages);
    let input = json!({"userId":pairs["userId"],"token":pairs["token"]});
    assert_eq!(
        browser
            .call(
                Method::POST,
                "/auth/confirm-email",
                Some(input.clone()),
                None
            )
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    browser.refresh_csrf().await;
    assert_eq!(
        browser
            .call(Method::POST, "/auth/confirm-email", Some(input), None)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    browser
        .login("recovery@example.test", "Recovered-Rust!123")
        .await;
    assert_eq!(
        browser
            .call(
                Method::POST,
                "/auth/request-password-reset",
                Some(json!({"email":"recovery@example.test"})),
                None
            )
            .await
            .status(),
        StatusCode::ACCEPTED
    );
    let messages = mail.0.lock().await;
    let url = reqwest::Url::parse(&messages[2].action_url).expect("expired URL");
    let pairs: BTreeMap<_, _> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    drop(messages);
    db.execute(
        Sql::new(
            "UPDATE identity.recovery_tokens SET expires=0",
            "UPDATE [identity].recovery_tokens SET expires=0",
        ),
        &[],
    )
    .await
    .expect("expire test token");
    assert_eq!(browser.call(Method::POST,"/auth/reset-password",Some(json!({"userId":pairs["userId"],"token":pairs["token"],"newPassword":"Another-Rust!123"})),None).await.status(),StatusCode::BAD_REQUEST);
}
fn location(response: &reqwest::Response) -> String {
    response
        .headers()
        .get("location")
        .expect("redirect")
        .to_str()
        .expect("location")
        .into()
}
async fn provider_login(client: &Client, base: &str, name: &str) -> String {
    let start = client
        .get(format!("{base}/auth/login"))
        .send()
        .await
        .expect("start");
    assert_eq!(start.status(), StatusCode::SEE_OTHER);
    let url = location(&start);
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("nonce="));
    let page = client.get(url).send().await.expect("login form");
    assert_eq!(page.status(), StatusCode::OK);
    let html = page.text().await.expect("html");
    let action = html
        .split("action=\"")
        .nth(1)
        .expect("form action")
        .split('"')
        .next()
        .expect("action")
        .replace("&amp;", "&");
    let response = client
        .post(action)
        .form(&[
            ("username", name),
            ("password", "RustOidc-Dev!2026"),
            ("credentialId", ""),
        ])
        .send()
        .await
        .expect("provider sign in");
    assert_eq!(
        response.status(),
        StatusCode::FOUND,
        "provider rejected {name}"
    );
    location(&response)
}
pub async fn oidc(db: Database) {
    let issuer = std::env::var("BQATLAS_TEST_OIDC_ISSUER").expect("issuer");
    let mapping: BTreeMap<String, Vec<String>> =
        serde_json::from_str(include_str!("../../../../../dev/oidc-roles.json"))
            .expect("role mappings");
    let mut auth = AuthState::new(db.clone()).await.expect("OIDC auth");
    auth.oidc = Some(
        OidcProvider::connect(
            OidcConfig {
                issuer,
                client_id: "bqatlas-rust".into(),
                client_secret: Some("rust-oidc-dev-only".into()),
                public_origin: "http://127.0.0.1:5202".into(),
                role_permissions: mapping,
                development: true,
            },
            &bqatlas_server::permissions(),
        )
        .await
        .expect("discovery"),
    );
    auth.mode = "oidc".into();
    let app = bqatlas_server::application_with_auth(db, false, auth)
        .await
        .expect("OIDC application");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5202")
        .await
        .expect("OIDC test listener");
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .expect("OIDC server")
    });
    let base = "http://127.0.0.1:5202";
    for (name, allowed) in [("admin", 17), ("reader", 6), ("unassigned", 0)] {
        let client = Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("browser");
        let callback = provider_login(&client, base, name).await;
        let response = client.get(&callback).send().await.expect("callback");
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "OIDC callback");
        let session: JsonValue = client
            .get(format!("{base}/api/v1/session"))
            .send()
            .await
            .expect("session")
            .json()
            .await
            .expect("session JSON");
        assert_eq!(session["authMode"], "oidc");
        assert_eq!(session["authenticated"], true);
        assert_eq!(
            session["permissions"]
                .as_array()
                .expect("permissions")
                .len(),
            allowed
        );
        assert!(
            session["id"]
                .as_str()
                .expect("subject")
                .starts_with("oidc:")
        );
        let csrf: JsonValue = client
            .get(format!("{base}/api/v1/session/csrf"))
            .send()
            .await
            .expect("csrf")
            .json()
            .await
            .expect("csrf JSON");
        let query = client
            .post(format!("{base}/api/v1/crm/customers/query"))
            .header("X-BQATLAS-CSRF", csrf["token"].as_str().expect("token"))
            .json(&json!({}))
            .send()
            .await
            .expect("query");
        assert_eq!(
            query.status(),
            if allowed == 0 {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::OK
            }
        );
        let write = client
            .post(format!("{base}/api/v1/crm/customers"))
            .header("X-BQATLAS-CSRF", csrf["token"].as_str().expect("token"))
            .json(&json!({"code":format!("OIDC-{name}"),"name":"OIDC test","active":true}))
            .send()
            .await
            .expect("create");
        assert_eq!(
            write.status(),
            if name == "admin" {
                StatusCode::CREATED
            } else {
                StatusCode::FORBIDDEN
            }
        );
        let manifest: JsonValue = client
            .get(format!("{base}/api/v1/manifest"))
            .send()
            .await
            .expect("manifest")
            .json()
            .await
            .expect("manifest JSON");
        assert_eq!(
            manifest["resources"].as_array().expect("resources").len(),
            if allowed == 0 { 0 } else { 5 }
        );
        assert_eq!(
            client.get(&callback).send().await.expect("replay").status(),
            StatusCode::UNAUTHORIZED
        );
        let logout = client
            .post(format!("{base}/auth/logout"))
            .form(&[(
                "__RequestVerificationToken",
                csrf["token"].as_str().expect("csrf"),
            )])
            .send()
            .await
            .expect("logout");
        assert_eq!(logout.status(), StatusCode::SEE_OTHER);
        let provider = client
            .get(location(&logout))
            .send()
            .await
            .expect("provider logout");
        assert_eq!(provider.status(), StatusCode::FOUND);
        assert_eq!(
            client
                .get(location(&provider))
                .send()
                .await
                .expect("logout callback")
                .status(),
            StatusCode::SEE_OTHER
        );
        let after: JsonValue = client
            .get(format!("{base}/api/v1/session"))
            .send()
            .await
            .expect("after logout")
            .json()
            .await
            .expect("JSON");
        assert_eq!(after["authenticated"], false);
    }
    let client = Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("negative browser");
    let callback = provider_login(&client, base, "admin").await;
    let mut url = reqwest::Url::parse(&callback).expect("callback URL");
    let pairs: Vec<_> = url
        .query_pairs()
        .map(|(k, v)| {
            (
                k.clone().into_owned(),
                if k == "state" {
                    "tampered".into()
                } else {
                    v.into_owned()
                },
            )
        })
        .collect();
    url.set_query(None);
    url.query_pairs_mut().extend_pairs(pairs);
    assert_eq!(
        client.get(url).send().await.expect("bad state").status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .get(callback)
            .send()
            .await
            .expect("consumed state")
            .status(),
        StatusCode::UNAUTHORIZED
    );
    server.abort();
}
