use std::time::Duration;

use crate::models::{Account, Entity, Project, User};
use sqlx::{postgres::PgPoolOptions, PgPool, Result};

/// Connects to a PostgreSQL database with the given `db_url`, returning a connection pool for accessing it
pub async fn connect_sqlx(db_url: &str) -> sqlx::PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .idle_timeout(Duration::from_secs(30))
        .max_connections(32)
        .min_connections(4)
        .connect(db_url)
        .await
        .expect("Could not connect to the database")
}

pub struct PostgreDatabase {
    sqlx_db: PgPool,
}

impl PostgreDatabase {
    pub fn new(sqlx_db: PgPool) -> Self {
        PostgreDatabase { sqlx_db }
    }
    /// Create a new user using a reference to a `User` struct
    pub async fn create_user(&self, user: &User) -> Result<User> {
        let result = sqlx::query!(
            r#"
            INSERT INTO app_user (name, email, hashed_password, role)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, email, hashed_password, role, created_at, updated_at
            "#,
            user.name,
            user.email,
            user.hashed_password,
            user.role
        )
        .fetch_one(&self.sqlx_db)
        .await;

        match result {
            Ok(row) => Ok(User {
                id: row.id,
                name: row.name,
                email: row.email,
                hashed_password: row.hashed_password,
                role: row.role,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }),
            Err(e) => Err(e),
        }
    }

    /// Get a user by ID
    pub async fn get_user_by_id(&self, user_id: i32) -> Result<Option<User>> {
        let row = sqlx::query_as!(
            User,
            r#"
            SELECT id, name, email, hashed_password, role, created_at, updated_at
            FROM app_user
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.sqlx_db)
        .await?;
        Ok(row)
    }

    /// Get a user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query_as!(
            User,
            r#"
            SELECT id, name, email, hashed_password, role, created_at, updated_at
            FROM app_user
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.sqlx_db)
        .await?;
        Ok(row)
    }

    // Create a new entity using a reference to a `Entity` struct
    pub async fn create_entity(&self, new_entity: &Entity) -> Result<Entity> {
        let result = sqlx::query!(
            r#"
            INSERT INTO entity (name)
            VALUES ($1)
            RETURNING id, name, created_at, updated_at
            "#,
            new_entity.name,
        )
        .fetch_one(&self.sqlx_db)
        .await;

        match result {
            Ok(row) => Ok(Entity {
                id: row.id,
                name: row.name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }),
            Err(e) => Err(e),
        }
    }
    /// Get an entity by ID
    pub async fn get_entity_by_id(&self, id: i32) -> Result<Option<Entity>> {
        let row = sqlx::query_as!(
            Entity,
            r#"
            SELECT id, name, created_at, updated_at
            FROM entity
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.sqlx_db)
        .await?;
        Ok(row)
    }
    /// Create a new account
    pub async fn create_account(&self, new_account: &Account) -> Result<Account> {
        let result = sqlx::query!(
            r#"
            INSERT INTO account (address, entity_id)
            VALUES ($1, $2)
            RETURNING id, address, entity_id, created_at, updated_at
            "#,
            new_account.address,
            new_account.entity_id
        )
        .fetch_one(&self.sqlx_db)
        .await;

        match result {
            Ok(row) => Ok(Account {
                id: row.id,
                address: row.address,
                entity_id: row.entity_id,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }),
            Err(e) => Err(e),
        }
    }
    /// Get an account by ID
    pub async fn get_account_by_id(&self, id: i32) -> Result<Option<Account>> {
        let row = sqlx::query_as!(
            Account,
            r#"
            SELECT id, address, entity_id, created_at, updated_at
            FROM account
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.sqlx_db)
        .await?;
        Ok(row)
    }
    pub async fn get_account_by_address(&self, address: &str) -> Result<Option<Account>> {
        let row = sqlx::query_as!(
            Account,
            r#"
            SELECT id, address, entity_id, created_at, updated_at
            FROM account
            WHERE address = $1
            "#,
            address
        )
        .fetch_optional(&self.sqlx_db)
        .await?;
        Ok(row)
    }
    pub async fn update_account(&self, account: &Account) -> Result<Account, sqlx::Error> {
        let query = sqlx::query_as!(
            Account,
            "UPDATE account SET entity_id = $1, updated_at = now() WHERE id = $2 RETURNING *",
            account.entity_id,
            account.id
        )
        .fetch_one(&self.sqlx_db)
        .await?;

        Ok(query)
    }

    /// Fetch a project by its ID
    pub async fn get_project_by_id(&self, id: i32) -> Result<Option<Project>, sqlx::Error> {
        let result = sqlx::query_as!(
            Project,
            r#"
            SELECT * FROM project
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.sqlx_db)
        .await?;

        Ok(result)
    }
    /// Create a new project
    pub async fn create_project(&self, project: &Project) -> Result<Project, sqlx::Error> {
        let result = sqlx::query_as!(
            Project,
            r#"
            INSERT INTO project (token, category, contract_address)
            VALUES ($1, $2, $3)
            RETURNING id, token, category, contract_address, created_at, updated_at
            "#,
            project.token,
            project.category,
            project.contract_address,
        )
        .fetch_one(&self.sqlx_db)
        .await?;

        Ok(result)
    }
    /// Update an existing project
    pub async fn update_project(&self, project: &Project) -> Result<Project, sqlx::Error> {
        let result = sqlx::query_as!(
            Project,
            r#"
            UPDATE project
            SET token = $1,
                category = $2,
                contract_address = $3,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $4
            RETURNING id, token, category, contract_address, created_at, updated_at
            "#,
            project.token,
            project.category,
            project.contract_address,
            project.id
        )
        .fetch_one(&self.sqlx_db)
        .await?;

        Ok(result)
    }
}
