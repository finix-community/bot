use eros::Context;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub struct Storage {
    pool: SqlitePool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct LinkedAccount {
    pub discord_id: i64,
    pub github_login: String,
    pub discord_refresh_token: String,
    pub last_synced_at: Option<i64>,
    pub pending_resync: bool,
}

impl Storage {
    pub async fn connect(database_url: &str) -> eros::Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .context("connecting to sqlite")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS linked_accounts (
                discord_id            INTEGER PRIMARY KEY,
                github_login          TEXT NOT NULL,
                discord_refresh_token TEXT NOT NULL,
                last_synced_at        INTEGER,
                pending_resync        INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .context("creating linked_accounts table")?;

        Ok(Self { pool })
    }

    pub async fn upsert_link(
        &self,
        discord_id: u64,
        github_login: &str,
        discord_refresh_token: &str,
    ) -> eros::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO linked_accounts (discord_id, github_login, discord_refresh_token, pending_resync)
            VALUES (?1, ?2, ?3, 1)
            ON CONFLICT(discord_id) DO UPDATE SET
                github_login = excluded.github_login,
                discord_refresh_token = excluded.discord_refresh_token,
                pending_resync = 1
            "#,
        )
        .bind(discord_id as i64)
        .bind(github_login)
        .bind(discord_refresh_token)
        .execute(&self.pool)
        .await
        .context("upserting linked_accounts")?;
        Ok(())
    }

    pub async fn remove_link(&self, discord_id: u64) -> eros::Result<()> {
        sqlx::query("DELETE FROM linked_accounts WHERE discord_id = ?1")
            .bind(discord_id as i64)
            .execute(&self.pool)
            .await
            .context("deleting linked_accounts")?;
        Ok(())
    }

    pub async fn is_linked(&self, discord_id: u64) -> eros::Result<bool> {
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT discord_id FROM linked_accounts WHERE discord_id = ?1")
                .bind(discord_id as i64)
                .fetch_optional(&self.pool)
                .await
                .context("checking is_linked")?;
        Ok(row.is_some())
    }

    pub async fn queue_resync(&self, discord_id: u64) -> eros::Result<()> {
        sqlx::query("UPDATE linked_accounts SET pending_resync = 1 WHERE discord_id = ?1")
            .bind(discord_id as i64)
            .execute(&self.pool)
            .await
            .context("queueing resync")?;
        Ok(())
    }

    pub async fn all_linked(&self) -> eros::Result<Vec<LinkedAccount>> {
        Ok(
            sqlx::query_as::<_, LinkedAccount>("SELECT * FROM linked_accounts")
                .fetch_all(&self.pool)
                .await
                .context("fetching all linked_accounts")?,
        )
    }

    pub async fn mark_synced(&self, discord_id: u64, timestamp: i64) -> eros::Result<()> {
        sqlx::query(
            "UPDATE linked_accounts SET last_synced_at = ?2, pending_resync = 0 WHERE discord_id = ?1",
        )
        .bind(discord_id as i64)
        .bind(timestamp)
        .execute(&self.pool)
        .await
        .context("marking account as synced")?;
        Ok(())
    }
}
