use bqatlas_auth::AuthState;
use bqatlas_db::Database;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
#[derive(Parser)]
#[command(version, about = "bqAtlas Rust CRM modular application")]
struct Options {
    #[arg(long, env = "BQATLAS_DATABASE_PROVIDER", default_value = "postgresql")]
    database_provider: String,
    #[arg(long, env = "BQATLAS_DATABASE_URL", hide_env_values = true)]
    database_url: String,
    #[arg(long, env = "BQATLAS_ENVIRONMENT", default_value = "Production")]
    environment: String,
    #[arg(long, env = "BQATLAS_AUTH_MODE", default_value = "local")]
    auth_mode: String,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Serve {
        #[arg(long, env = "BQATLAS_LISTEN", default_value = "127.0.0.1:5201")]
        listen: SocketAddr,
    },
    Migrate,
    Bootstrap,
    Seed,
    /// Explicitly provision a local account; password/email come from environment variables.
    CreateUser {
        #[arg(long)]
        permissions: std::path::PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        confirmed: bool,
    },
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "bqatlas_server=info,bqatlas_auth=info,bqatlas_db=info,tower_http=info".into()
            }),
        )
        .init();
    let options = Options::parse();
    if !matches!(options.auth_mode.as_str(), "local" | "oidc") {
        return Err("Unsupported authentication mode".into());
    }
    let db = Database::connect(&options.database_provider, &options.database_url, 10)
        .await
        .map_err(|_| "Unable to connect to configured database")?;
    match options.command {
        Command::Migrate => {
            db.migrate(&bqatlas_server::migrations())
                .await
                .map_err(|e| e.storage_message())?;
            println!("Migrations applied for {}", db.provider());
        }
        Command::Seed => {
            db.check_migrations(&bqatlas_server::migrations())
                .await
                .map_err(|e| e.storage_message())?;
            bqatlas_server::seed::seed(db, &options.environment).await?;
            println!("CRM demo data ready; existing records preserved");
        }
        Command::CreateUser {
            permissions,
            name,
            confirmed,
        } => {
            if options.auth_mode != "local" {
                return Err("Local user provisioning requires local authentication mode".into());
            }
            db.check_migrations(&bqatlas_server::migrations())
                .await
                .map_err(|e| e.storage_message())?;
            let permissions: Vec<String> =
                serde_json::from_str(&std::fs::read_to_string(permissions)?)?;
            let declared = bqatlas_server::permissions();
            if permissions.iter().any(|p| !declared.contains(p)) {
                return Err("User permissions must name declared application permissions".into());
            }
            let email = std::env::var("BQATLAS_ACCOUNT_EMAIL")?;
            let password = std::env::var("BQATLAS_ACCOUNT_PASSWORD")?;
            let created = AuthState::new(db)
                .await?
                .create_local_account(
                    &email,
                    &password,
                    name.as_deref().unwrap_or(&email),
                    confirmed,
                    &permissions,
                )
                .await?;
            println!(
                "{}",
                if created {
                    "Local account created"
                } else {
                    "Existing account preserved"
                }
            );
        }
        Command::Bootstrap => {
            db.check_migrations(&bqatlas_server::migrations())
                .await
                .map_err(|e| e.storage_message())?;
            let email = std::env::var("BQATLAS_BOOTSTRAP_EMAIL")
                .map_err(|_| "Set BQATLAS_BOOTSTRAP_EMAIL")?;
            let password = std::env::var("BQATLAS_BOOTSTRAP_PASSWORD")
                .map_err(|_| "Set BQATLAS_BOOTSTRAP_PASSWORD")?;
            let created = AuthState::new(db)
                .await?
                .bootstrap(
                    &options.environment,
                    &email,
                    &password,
                    &bqatlas_server::permissions(),
                )
                .await?;
            println!(
                "{}",
                if created {
                    "Development account created"
                } else {
                    "Existing account preserved"
                }
            );
        }
        Command::Serve { listen } => {
            db.check_migrations(&bqatlas_server::migrations())
                .await
                .map_err(|_| "Database schema is not ready; run migrate")?;
            let mut auth = AuthState::new(db.clone()).await?;
            if let Ok(directory) = std::env::var("BQATLAS_DEV_MAIL_DIR") {
                let sender = bqatlas_auth::recovery::DevelopmentEmailSender::new(
                    &options.environment,
                    directory.into(),
                )?;
                auth.recovery = Some(bqatlas_auth::recovery::RecoveryConfig::new(
                    sender,
                    &std::env::var("BQATLAS_PUBLIC_ORIGIN")?,
                    options.environment == "Development",
                )?);
            }
            if options.auth_mode == "oidc" {
                let role_file = std::env::var("BQATLAS_OIDC_ROLE_FILE").map_err(
                    |_| "Set BQATLAS_OIDC_ROLE_FILE to an explicit role-to-permission JSON mapping",
                )?;
                let mapping = serde_json::from_str(&std::fs::read_to_string(role_file)?)?;
                auth.oidc = Some(
                    bqatlas_auth::OidcProvider::connect(
                        bqatlas_auth::OidcConfig {
                            issuer: std::env::var("BQATLAS_OIDC_ISSUER")?,
                            client_id: std::env::var("BQATLAS_OIDC_CLIENT_ID")?,
                            client_secret: std::env::var("BQATLAS_OIDC_CLIENT_SECRET").ok(),
                            public_origin: std::env::var("BQATLAS_PUBLIC_ORIGIN")?,
                            role_permissions: mapping,
                            development: options.environment == "Development",
                        },
                        &bqatlas_server::permissions(),
                    )
                    .await?,
                );
                auth.mode = "oidc".into();
            }
            let mut app = bqatlas_server::application_with_auth(
                db,
                options.environment != "Development",
                auth,
            )
            .await?;
            if let Ok(root) = std::env::var("BQATLAS_WEB_ROOT") {
                if !std::path::Path::new(&root).join("index.html").is_file() {
                    return Err("Configured frontend root has no index.html".into());
                }
                let files = tower_http::services::ServeDir::new(&root).not_found_service(
                    tower_http::services::ServeFile::new(
                        std::path::Path::new(&root).join("index.html"),
                    ),
                );
                app = app.fallback_service(tower::service_fn(
                    move |request: axum::extract::Request| {
                        use axum::response::IntoResponse;
                        use tower::ServiceExt;
                        let files = files.clone();
                        async move {
                            let path = request.uri().path();
                            let response = if path.starts_with("/api/")
                                || path.starts_with("/auth/")
                                || path.starts_with("/odata")
                                || path.starts_with("/health/")
                                || path.starts_with("/signin-")
                                || path.starts_with("/signout-")
                            {
                                bqatlas_http::ApiError(bqatlas_core::AppError::NotFound)
                                    .into_response()
                            } else {
                                files
                                    .oneshot(request)
                                    .await
                                    .expect("ServeDir is infallible")
                                    .into_response()
                            };
                            Ok::<_, std::convert::Infallible>(response)
                        }
                    },
                ));
            }
            let listener = tokio::net::TcpListener::bind(listen).await?;
            tracing::info!(%listen,"bqAtlas Rust backend listening");
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        }
    }
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {},
                    _ = terminate.recv() => {},
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
