use sqlx::SqlitePool;

/// Delete expired paste rows, returning how many were removed.
/// Called once at start up from `init_server`.
pub async fn delete_expired(db: &SqlitePool) -> sqlx::Result<u64> {
    let result =
        sqlx::query("DELETE FROM pastes WHERE expires_at IS NOT NULL AND expires_at < unixepoch()")
            .execute(db)
            .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use crate::state::test_state;

    use super::delete_expired;

    #[tokio::test]
    async fn delete_expired_removes_only_expired_rows() {
        let now = crate::schema::time::now_secs();
        let state = test_state().await;

        for (id, created_at, expires_at) in [
            ("expired", now - 100, Some(now - 50)),
            ("fresh", now, Some(now + 3600)),
            ("eternal", now, None),
        ] {
            sqlx::query(
                "INSERT INTO pastes (id, content, created_at, expires_at) VALUES (?, ?, ?, ?)",
            )
            .bind(id)
            .bind("content")
            .bind(created_at)
            .bind(expires_at)
            .execute(&state.db)
            .await
            .unwrap();
        }

        let deleted = delete_expired(&state.db).await.unwrap();

        assert_eq!(deleted, 1, "only the expired paste should be deleted");
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pastes")
                .fetch_one(&state.db)
                .await
                .unwrap(),
            2,
            "fresh and non-expiring rows must survive"
        );
    }
}
