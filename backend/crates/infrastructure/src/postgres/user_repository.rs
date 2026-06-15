use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use application::ports::{RepoError, UserRepository};
use domain::{Email, PasswordHash, User, UserId};

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct LoginRow {
    id: Uuid,
    email: String,
    name: String,
    password_hash: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = RepoError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        let email = Email::parse(row.email)
            .map_err(|e| RepoError::Storage(format!("invalid email in db: {e}")))?;
        Ok(User {
            id: UserId(row.id),
            email,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn map_sqlx(e: sqlx::Error) -> RepoError {
    match e {
        sqlx::Error::RowNotFound => RepoError::NotFound,
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            RepoError::Conflict(db.message().to_string())
        }
        other => RepoError::Storage(other.to_string()),
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn create(&self, user: &User, password_hash: &PasswordHash) -> Result<(), RepoError> {
        sqlx::query(
            r#"
            INSERT INTO users (id, email, name, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(user.id.0)
        .bind(user.email.as_str())
        .bind(&user.name)
        .bind(password_hash.as_str())
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn create_oauth(&self, user: &User, google_id: &str) -> Result<(), RepoError> {
        sqlx::query(
            r#"
            INSERT INTO users (id, email, name, password_hash, google_id, created_at, updated_at)
            VALUES ($1, $2, $3, NULL, $4, $5, $6)
            "#,
        )
        .bind(user.id.0)
        .bind(user.email.as_str())
        .bind(&user.name)
        .bind(google_id)
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn find_by_google_id(&self, google_id: &str) -> Result<Option<User>, RepoError> {
        let row: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, email, name, created_at, updated_at
            FROM users
            WHERE google_id = $1
            "#,
        )
        .bind(google_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        row.map(User::try_from).transpose()
    }

    async fn link_google(&self, user_id: UserId, google_id: &str) -> Result<(), RepoError> {
        // Refuse to overwrite an existing google_id on this user — if a
        // re-link is ever needed, that should be an explicit migration
        // path. The WHERE clause makes this a no-op for already-linked
        // rows, and we then verify rows_affected to distinguish.
        let res = sqlx::query(
            r#"
            UPDATE users
            SET google_id = $2, updated_at = NOW()
            WHERE id = $1 AND google_id IS NULL
            "#,
        )
        .bind(user_id.0)
        .bind(google_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        if res.rows_affected() == 0 {
            // Either no such user, or they already have a (possibly
            // different) google_id. Report the more useful error.
            let row: Option<(Option<String>,)> =
                sqlx::query_as("SELECT google_id FROM users WHERE id = $1")
                    .bind(user_id.0)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(map_sqlx)?;
            return match row {
                None => Err(RepoError::NotFound),
                Some((Some(existing),)) if existing == google_id => Ok(()),
                Some((Some(_),)) => Err(RepoError::Conflict(
                    "user already linked to a different Google account".into(),
                )),
                Some((None,)) => Err(RepoError::Storage(
                    "link_google update affected 0 rows despite NULL google_id".into(),
                )),
            };
        }
        Ok(())
    }

    async fn get(&self, id: UserId) -> Result<User, RepoError> {
        let row: UserRow = sqlx::query_as(
            r#"
            SELECT id, email, name, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        row.try_into()
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError> {
        let row: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, email, name, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        row.map(User::try_from).transpose()
    }

    async fn find_for_login(
        &self,
        email: &Email,
    ) -> Result<Option<(User, PasswordHash)>, RepoError> {
        // `IS NOT NULL` keeps OAuth-only users out of the password-login
        // path: Login then returns InvalidCredentials, which is the
        // semantically correct response (this account has no password).
        let row: Option<LoginRow> = sqlx::query_as(
            r#"
            SELECT id, email, name, password_hash, created_at, updated_at
            FROM users
            WHERE email = $1 AND password_hash IS NOT NULL
            "#,
        )
        .bind(email.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

        let Some(row) = row else { return Ok(None) };
        let email = Email::parse(row.email)
            .map_err(|e| RepoError::Storage(format!("invalid email in db: {e}")))?;
        let user = User {
            id: UserId(row.id),
            email,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        Ok(Some((user, PasswordHash::from_raw(row.password_hash))))
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, RepoError> {
        let rows: Vec<UserRow> = sqlx::query_as(
            r#"
            SELECT id, email, name, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        rows.into_iter().map(User::try_from).collect()
    }
}
