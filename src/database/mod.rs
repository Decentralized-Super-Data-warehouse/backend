use crate::models::{project::ProjectAttribute, Account, Entity, Project, User};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Result};

/// Connects to a PostgreSQL database with the given `db_url`, returning a connection pool for accessing it
pub async fn connect_sqlx(db_url: &str) -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(32)
        .min_connections(4)
        .connect(db_url)
        .await
        .expect("Could not connect to the database")
}

#[derive(Clone)]
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
            RETURNING id, name, address, entity_id, created_at, updated_at
            "#,
            new_account.address,
            new_account.entity_id
        )
        .fetch_one(&self.sqlx_db)
        .await;

        match result {
            Ok(row) => Ok(Account {
                id: row.id,
                name: row.name,
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
            SELECT *
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
            SELECT *
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
            "UPDATE account SET name = $1, entity_id = $2, updated_at = now() WHERE id = $3 RETURNING *",
            account.name,
            account.entity_id,
            account.id
        )
        .fetch_one(&self.sqlx_db)
        .await?;

        Ok(query)
    }

    /// Fetch a project by its ID
    pub async fn get_project_by_id(&self, id: i32) -> Result<Option<Project>, sqlx::Error> {
        let project = sqlx::query!(
            r#"
            SELECT *
            FROM project
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.sqlx_db)
        .await?;

        if let Some(p) = project {
            let attributes = self.get_project_attributes(id).await?;
            Ok(Some(Project {
                id: p.id,
                name: p.name,
                token: p.token,
                category: p.category,
                contract_address: p.contract_address,
                avatar_url: p.avatar_url,
                created_at: p.created_at,
                updated_at: p.updated_at,
                attributes,
            }))
        } else {
            Ok(None)
        }
    }

    /// Fetch a project by its name
    pub async fn get_project_by_name(&self, name: &str) -> Result<Option<Project>, sqlx::Error> {
        let project = sqlx::query!(
            r#"
            SELECT *
            FROM project
            WHERE name = $1
            "#,
            name
        )
        .fetch_optional(&self.sqlx_db)
        .await?;

        if let Some(p) = project {
            let attributes = self.get_project_attributes(p.id).await?;
            Ok(Some(Project {
                id: p.id,
                name: p.name,
                token: p.token,
                category: p.category,
                contract_address: p.contract_address,
                avatar_url: p.avatar_url,
                created_at: p.created_at,
                updated_at: p.updated_at,
                attributes,
            }))
        } else {
            Ok(None)
        }
    }

    /// Fetch a project by its contract address
    pub async fn get_project_by_address(
        &self,
        address: &str,
    ) -> Result<Option<Project>, sqlx::Error> {
        let project = sqlx::query!(
            r#"
            SELECT *
            FROM project
            WHERE contract_address = $1
            "#,
            address
        )
        .fetch_optional(&self.sqlx_db)
        .await?;

        if let Some(p) = project {
            let attributes = self.get_project_attributes(p.id).await?;
            Ok(Some(Project {
                id: p.id,
                name: p.name,
                token: p.token,
                category: p.category,
                contract_address: p.contract_address,
                avatar_url: p.avatar_url,
                created_at: p.created_at,
                updated_at: p.updated_at,
                attributes,
            }))
        } else {
            Ok(None)
        }
    }

    /// Create a new project
    pub async fn create_project(&self, project: &Project) -> Result<Project, sqlx::Error> {
        // Start a new transaction
        let mut transaction = self.sqlx_db.begin().await?;

        let new_project = sqlx::query!(
            r#"
            INSERT INTO project (name, token, category, contract_address)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
            project.name,
            project.token,
            project.category,
            project.contract_address,
        )
        .fetch_one(&mut *transaction)
        .await?;

        for attr in &project.attributes {
            sqlx::query!(
                r#"
                INSERT INTO project_attribute (project_id, key, value, value_type)
                VALUES ($1, $2, $3, $4)
                "#,
                new_project.id,
                attr.key,
                attr.value.to_string(),
                get_value_type(&attr.value)
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(Project {
            id: new_project.id,
            name: new_project.name,
            token: new_project.token,
            category: new_project.category,
            contract_address: new_project.contract_address,
            avatar_url: new_project.avatar_url,
            created_at: new_project.created_at,
            updated_at: new_project.updated_at,
            attributes: project.attributes.clone(),
        })
    }

    /// Update an existing project
    pub async fn update_project(&self, project: &Project) -> Result<Project, sqlx::Error> {
        // Start a new transaction
        let mut transaction = self.sqlx_db.begin().await?;

        let updated_project = sqlx::query!(
        r#"
        UPDATE project
        SET name = $1, token = $2, category = $3, contract_address = $4, updated_at = CURRENT_TIMESTAMP
        WHERE id = $5
        RETURNING *
        "#,
        project.name, project.token, project.category, project.contract_address, project.id
    )
    .fetch_one(&mut *transaction)
    .await?;

        // Delete existing attributes
        sqlx::query!(
            r#"
        DELETE FROM project_attribute
        WHERE project_id = $1
        "#,
            project.id
        )
        .execute(&mut *transaction)
        .await?;

        // Insert new attributes
        for attr in &project.attributes {
            sqlx::query!(
                r#"
            INSERT INTO project_attribute (project_id, key, value, value_type)
            VALUES ($1, $2, $3, $4)
            "#,
                project.id,
                attr.key,
                attr.value.to_string(),
                get_value_type(&attr.value)
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(Project {
            id: updated_project.id,
            name: updated_project.name,
            token: updated_project.token,
            category: updated_project.category,
            contract_address: updated_project.contract_address,
            avatar_url: updated_project.avatar_url,
            created_at: updated_project.created_at,
            updated_at: updated_project.updated_at,
            attributes: project.attributes.clone(),
        })
    }

    /// Get project attributes
    async fn get_project_attributes(
        &self,
        project_id: i32,
    ) -> Result<Vec<ProjectAttribute>, sqlx::Error> {
        let attributes = sqlx::query!(
            r#"
            SELECT key, value, value_type
            FROM project_attribute
            WHERE project_id = $1
            "#,
            project_id
        )
        .fetch_all(&self.sqlx_db)
        .await?;

        Ok(attributes
            .into_iter()
            .map(|attr| ProjectAttribute {
                key: attr.key,
                // If `attr.value` is `None`, provide a default value (e.g., an empty string).
                value: parse_value(attr.value.as_deref().unwrap_or(""), &attr.value_type),
            })
            .collect())
    }
    pub async fn update_project_attribute(
        &self,
        project_id: i32,
        key: &str,
        value: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO project_attribute (project_id, key, value, value_type)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (project_id, key) 
            DO UPDATE SET value = $3, value_type = $4
            "#,
            project_id,
            key,
            value.to_string(),
            get_type(&value)
        )
        .execute(&self.sqlx_db)
        .await?;

        Ok(())
    }
}

fn get_value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) => {
            if n.is_i64() {
                "integer"
            } else {
                "float"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn get_type(value: &str) -> &'static str {
    if value == "null" {
        "null"
    } else if value.parse::<bool>().is_ok() {
        "boolean"
    } else if value.parse::<i64>().is_ok() {
        "integer"
    } else if value.parse::<f64>().is_ok() {
        "float"
    } else {
        "string"
    }
}

fn parse_value(value: &str, value_type: &str) -> Value {
    match value_type {
        "null" => Value::Null,
        "boolean" => value.parse().map(Value::Bool).unwrap_or(Value::Bool(false)),
        "integer" => value
            .parse()
            .map(Value::Number)
            .unwrap_or(Value::Number(0.into())),
        "float" => serde_json::Number::from_f64(value.parse().unwrap_or(0.0))
            .map(Value::Number)
            .unwrap_or(Value::Null),
        "string" => Value::String(value.to_string()),
        "array" | "object" => serde_json::from_str(value).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}
