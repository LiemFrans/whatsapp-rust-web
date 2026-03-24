use anyhow::Result;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub session_db_dir: String,
    pub sync_days_limit: i32,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://whatsapp:whatsapp@localhost:5432/whatsapp_web".into()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production-please".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            session_db_dir: std::env::var("SESSION_DB_DIR")
                .unwrap_or_else(|_| "./wa_sessions".into()),
            sync_days_limit: std::env::var("SYNC_DAYS_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        })
    }
}
