use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    // Check if schema already exists
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'users')",
    )
    .fetch_one(pool)
    .await?;

    if !exists {
        let sql = include_str!("../../../database/migrations/001_initial_schema.sql");
        // Split statements properly, respecting $$ blocks (PL/pgSQL)
        let statements = split_sql_statements(sql);
        for stmt in &statements {
            let s = stmt.trim();
            if !s.is_empty() && !s.starts_with("--") {
                if let Err(e) = sqlx::query(s).execute(pool).await {
                    tracing::warn!("Migration statement error (may be harmless): {}", e);
                }
            }
        }
        tracing::info!("Database migrations applied");
    }

    // Always apply incremental migration 002 (uses IF NOT EXISTS, safe to re-run)
    let sql_002 = include_str!("../../../database/migrations/002_add_media_keys_and_contacts.sql");
    let stmts_002 = split_sql_statements(sql_002);
    for stmt in &stmts_002 {
        let s = stmt.trim();
        if !s.is_empty() && !s.starts_with("--") {
            if let Err(e) = sqlx::query(s).execute(pool).await {
                tracing::warn!("Migration 002 statement (may be harmless): {}", e);
            }
        }
    }

    // Always apply incremental migration 003 (uses IF NOT EXISTS, safe to re-run)
    let sql_003 = include_str!("../../../database/migrations/003_api_tokens.sql");
    let stmts_003 = split_sql_statements(sql_003);
    for stmt in &stmts_003 {
        let s = stmt.trim();
        if !s.is_empty() && !s.starts_with("--") {
            if let Err(e) = sqlx::query(s).execute(pool).await {
                tracing::warn!("Migration 003 statement (may be harmless): {}", e);
            }
        }
    }

    Ok(())
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_dollar_quote = false;

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'$') {
            current.push(ch);
            current.push(chars.next().unwrap()); // consume second $
            in_dollar_quote = !in_dollar_quote;
        } else if ch == ';' && !in_dollar_quote {
            let stmt = current.trim().to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
        } else if ch == '-' && chars.peek() == Some(&'-') && !in_dollar_quote {
            // Skip line comments
            current.push(ch);
            while let Some(c) = chars.next() {
                current.push(c);
                if c == '\n' {
                    break;
                }
            }
        } else {
            current.push(ch);
        }
    }

    let stmt = current.trim().to_string();
    if !stmt.is_empty() {
        statements.push(stmt);
    }

    statements
}

pub async fn seed_admin(pool: &PgPool) -> anyhow::Result<()> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = 'admin')",
    )
    .fetch_one(pool)
    .await?;

    if !exists {
        let password_hash = crate::auth::jwt::hash_password("admin123")?;
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, display_name, role)
             VALUES (gen_random_uuid(), 'admin', 'admin@localhost', $1, 'Administrator', 'admin')",
        )
        .bind(&password_hash)
        .execute(pool)
        .await?;
        tracing::info!("Default admin user created (admin / admin123)");
    }

    Ok(())
}
