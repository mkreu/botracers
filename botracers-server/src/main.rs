use botracers_server::{AuthMode, ServerConfig, run_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "botracers_server=info,tower_http=info".into()),
        )
        .init();

    let mut config = ServerConfig::default();
    if let Ok(bind) = std::env::var("BOTRACERS_BIND") {
        config.bind = bind;
    }
    if let Ok(db_path) = std::env::var("BOTRACERS_DB_PATH") {
        config.db_path = db_path.into();
    }
    if let Ok(artifacts_dir) = std::env::var("BOTRACERS_ARTIFACTS_DIR") {
        config.artifacts_dir = artifacts_dir.into();
    }
    if let Ok(mode) = std::env::var("BOTRACERS_AUTH_MODE") {
        config.auth_mode = AuthMode::from_env(&mode);
    }
    if let Ok(cookie_secure) = std::env::var("BOTRACERS_COOKIE_SECURE") {
        config.cookie_secure = matches!(cookie_secure.as_str(), "1" | "true" | "TRUE" | "True");
    }
    if let Ok(registration_enabled) = std::env::var("BOTRACERS_REGISTRATION_ENABLED") {
        config.registration_enabled = matches!(
            registration_enabled.as_str(),
            "1" | "true" | "TRUE" | "True"
        );
    }
    if let Ok(static_dir) = std::env::var("BOTRACERS_STATIC_DIR") {
        if static_dir.trim().is_empty() {
            config.static_dir = None;
        } else {
            config.static_dir = Some(static_dir.into());
        }
    }

    run_server(config).await
}
