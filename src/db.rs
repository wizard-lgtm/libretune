use std::sync::LazyLock;
use surrealdb;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;
use surrealdb::opt::auth::Root;
use surrealdb::engine::remote::ws::Ws;
use uuid::Uuid;
use chrono::Utc;
use crate::types::user::{User, UserProfile, CreatedVia};

mod error {
    use actix_web::{HttpResponse, ResponseError};
    use thiserror::Error;
    
    #[derive(Error, Debug)]
    pub enum Error {
        #[error("database error")]
        Db(String),
        
        #[error("user not found")]
        UserNotFound,
        
        #[error("email already exists")]
        EmailExists,
        
        #[error("username already exists")]
        UsernameExists,
    }
    
    impl ResponseError for Error {
        fn error_response(&self) -> HttpResponse {
            match self {
                Error::Db(e) => HttpResponse::InternalServerError().body(e.to_string()),
                Error::UserNotFound => HttpResponse::NotFound().body("User not found"),
                Error::EmailExists => HttpResponse::Conflict().body("Email already exists"),
                Error::UsernameExists => HttpResponse::Conflict().body("Username already exists"),
            }
        }
    }
    
    impl From<surrealdb::Error> for Error {
        fn from(error: surrealdb::Error) -> Self {
            eprintln!("{error}");
            Self::Db(error.to_string())
        }
    }
}

pub static DB: LazyLock<Surreal<Client>> = LazyLock::new(Surreal::init);

pub async fn connect_db() -> Result<(), surrealdb::Error> {
    DB.connect::<Ws>("localhost:8000").await?;
    DB.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;
    
    // Use namespace and database
    DB.use_ns("libretune").use_db("main").await?;
    
    println!("🚀 Connected to SurrealDB!");
    Ok(())
}

pub struct UserOperations;

impl UserOperations {
    /// Create a new user
    pub async fn create_user(
        username: String,
        email: String,
        hashed_password: String,
        created_via: CreatedVia,
        bio: Option<String>,
    ) -> Result<User, error::Error> {
        // Check if email already exists
        let existing_email: Option<User> = DB
            .query("SELECT * FROM users WHERE email = $email")
            .bind(("email", email.clone()))
            .await?
            .take(0)?;
            
        if existing_email.is_some() {
            return Err(error::Error::EmailExists);
        }
        
        // Check if username already exists
        let existing_username: Option<User> = DB
            .query("SELECT * FROM users WHERE username = $username")
            .bind(("username", username.clone()))
            .await?
            .take(0)?;
            
        if existing_username.is_some() {
            return Err(error::Error::UsernameExists);
        }
        
        let now = Utc::now();
        let user_id = Uuid::new_v4();
        
        let user = User {
            id: user_id,
            username: username.clone(),
            email: email.clone(),
            hashed_password,
            created_at: now,
            updated_at: now,
            bio,
            created_via,
            profile: None,
            email_verified: false,
            playlists: None,
        };
        
        let created_user: Option<User> = DB
            .create(("users", user_id.to_string()))
            .content(user)
            .await?;
            
        created_user.ok_or(error::Error::Db("Failed to create user".to_string()))
    }
    
    /// Get user by ID
    pub async fn get_user_by_id(user_id: Uuid) -> Result<User, error::Error> {
        let user: Option<User> = DB
            .select(("users", user_id.to_string()))
            .await?;
            
        user.ok_or(error::Error::UserNotFound)
    }
    
    /// Get user by email
    pub async fn get_user_by_email(email: String) -> Result<User, error::Error> {
        let user: Option<User> = DB
            .query("SELECT * FROM users WHERE email = $email")
            .bind(("email", email))
            .await?
            .take(0)?;
            
        user.ok_or(error::Error::UserNotFound)
    }
    
    /// Get user by username
    pub async fn get_user_by_username(username: String) -> Result<User, error::Error> {
        let user: Option<User> = DB
            .query("SELECT * FROM users WHERE username = $username")
            .bind(("username", username))
            .await?
            .take(0)?;
            
        user.ok_or(error::Error::UserNotFound)
    }
    
    /// Update user with modified user object (checks for changes)  
    pub async fn update_user(user_id: Uuid, mut modified_user: User) -> Result<User, error::Error> {
        // Get current user from database
        let current_user = Self::get_user_by_id(user_id).await?;
        
        // Ensure the user ID matches
        modified_user.id = user_id;
        
        // Check for conflicts if username has changed
        if modified_user.username != current_user.username {
            let existing: Option<User> = DB
                .query("SELECT * FROM users WHERE username = $username AND id != $user_id")
                .bind(("username", modified_user.username.clone()))
                .bind(("user_id", user_id.to_string()))
                .await?
                .take(0)?;
                
            if existing.is_some() {
                return Err(error::Error::UsernameExists);
            }
        }
        
        // Check for conflicts if email has changed
        if modified_user.email != current_user.email {
            let existing: Option<User> = DB
                .query("SELECT * FROM users WHERE email = $email AND id != $user_id")
                .bind(("email", modified_user.email.clone()))
                .bind(("user_id", user_id.to_string()))
                .await?
                .take(0)?;
                
            if existing.is_some() {
                return Err(error::Error::EmailExists);
            }
        }
        
        // Preserve certain fields that shouldn't be changed through this method
        modified_user.created_at = current_user.created_at;
        modified_user.hashed_password = current_user.hashed_password; // Password changes should use separate method
        modified_user.email_verified = current_user.email_verified; // Email verification should use separate method
        
        // Update the timestamp
        modified_user.updated_at = Utc::now();
        
        let updated_user: Option<User> = DB
            .update(("users", user_id.to_string()))
            .content(modified_user)
            .await?;
            
        updated_user.ok_or(error::Error::Db("Failed to update user".to_string()))
    }
    
    /// Update user basic information with individual fields
    pub async fn update_user_fields(
        user_id: Uuid,
        username: Option<String>,
        email: Option<String>,
        bio: Option<String>,
    ) -> Result<User, error::Error> {
        // Check if user exists
        let mut user = Self::get_user_by_id(user_id).await?;
        
        // Check for conflicts if updating username or email
        if let Some(ref new_username) = username {
            if new_username != &user.username {
                let existing: Option<User> = DB
                    .query("SELECT * FROM users WHERE username = $username AND id != $user_id")
                    .bind(("username", new_username.clone()))
                    .bind(("user_id", user_id.to_string()))
                    .await?
                    .take(0)?;
                    
                if existing.is_some() {
                    return Err(error::Error::UsernameExists);
                }
            }
        }
        
        if let Some(ref new_email) = email {
            if new_email != &user.email {
                let existing: Option<User> = DB
                    .query("SELECT * FROM users WHERE email = $email AND id != $user_id")
                    .bind(("email", new_email.clone()))
                    .bind(("user_id", user_id.to_string()))
                    .await?
                    .take(0)?;
                    
                if existing.is_some() {
                    return Err(error::Error::EmailExists);
                }
            }
        }
        
        // Update fields
        if let Some(new_username) = username {
            user.username = new_username;
        }
        if let Some(new_email) = email {
            user.email = new_email;
        }
        if bio.is_some() {
            user.bio = bio;
        }
        
        user.updated_at = Utc::now();
        
        let updated_user: Option<User> = DB
            .update(("users", user_id.to_string()))
            .content(user)
            .await?;
            
        updated_user.ok_or(error::Error::Db("Failed to update user".to_string()))
    }
    
    /// Update user password
    pub async fn update_password(user_id: Uuid, new_hashed_password: String) -> Result<User, error::Error> {
        let mut user = Self::get_user_by_id(user_id).await?;
        user.hashed_password = new_hashed_password;
        user.updated_at = Utc::now();
        
        let updated_user: Option<User> = DB
            .update(("users", user_id.to_string()))
            .content(user)
            .await?;
            
        updated_user.ok_or(error::Error::Db("Failed to update password".to_string()))
    }
    
    /// Verify user email
    pub async fn verify_email(user_id: Uuid) -> Result<User, error::Error> {
        let mut user = Self::get_user_by_id(user_id).await?;
        user.email_verified = true;
        user.updated_at = Utc::now();
        
        let updated_user: Option<User> = DB
            .update(("users", user_id.to_string()))
            .content(user)
            .await?;
            
        updated_user.ok_or(error::Error::Db("Failed to verify email".to_string()))
    }
    
    /// Create or update user profile
    pub async fn update_profile(user_id: Uuid, profile: UserProfile) -> Result<User, error::Error> {
        let mut user = Self::get_user_by_id(user_id).await?;
        user.profile = Some(profile);
        user.updated_at = Utc::now();
        
        let updated_user: Option<User> = DB
            .update(("users", user_id.to_string()))
            .content(user)
            .await?;
            
        updated_user.ok_or(error::Error::Db("Failed to update profile".to_string()))
    }
    
    /// Get all users with pagination
    pub async fn get_users(limit: Option<u32>, offset: Option<u32>) -> Result<Vec<User>, error::Error> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);
        
        let users: Vec<User> = DB
            .query("SELECT * FROM users ORDER BY created_at DESC LIMIT $limit START $offset")
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?
            .take(0)?;
            
        Ok(users)
    }
    
    /// Search users by username or profile name
    pub async fn search_users(
        query: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<User>, error::Error> {
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);
        
        let users: Vec<User> = DB
            .query(
                "SELECT * FROM users WHERE 
                string::lowercase(username) CONTAINS string::lowercase($query) OR 
                string::lowercase(profile.profile_name) CONTAINS string::lowercase($query)
                ORDER BY created_at DESC 
                LIMIT $limit START $offset"
            )
            .bind(("query", query))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?
            .take(0)?;
            
        Ok(users)
    }
    
    /// Delete user (soft delete)
    pub async fn delete_user(user_id: Uuid) -> Result<(), error::Error> {
        // First check if user exists
        let mut user = Self::get_user_by_id(user_id).await?;
        
        // Update profile to mark as deleted if profile exists
        if let Some(ref mut profile) = user.profile {
            profile.is_deleted = true;
        }
        
        user.updated_at = Utc::now();
        
        let _: Option<User> = DB
            .update(("users", user_id.to_string()))
            .content(user)
            .await?;
            
        Ok(())
    }
    
    /// Hard delete user (permanently remove from database)
    pub async fn hard_delete_user(user_id: Uuid) -> Result<(), error::Error> {
        // Check if user exists first
        let _user = Self::get_user_by_id(user_id).await?;
        
        let _: Option<User> = DB
            .delete(("users", user_id.to_string()))
            .await?;
            
        Ok(())
    }
    
    /// Get user statistics/counts
    pub async fn get_user_stats() -> Result<UserStats, error::Error> {
        let total_users: Option<i64> = DB
            .query("SELECT count() FROM users GROUP ALL")
            .await?
            .take((0, "count"))?;
            
        let verified_users: Option<i64> = DB
            .query("SELECT count() FROM users WHERE email_verified = true GROUP ALL")
            .await?
            .take((0, "count"))?;
            
        let active_users: Option<i64> = DB
            .query("SELECT count() FROM users WHERE profile.is_active = true GROUP ALL")
            .await?
            .take((0, "count"))?;
            
        Ok(UserStats {
            total_users: total_users.unwrap_or(0) as u64,
            verified_users: verified_users.unwrap_or(0) as u64,
            active_users: active_users.unwrap_or(0) as u64,
        })
    }
    
    /// Check if username is available
    pub async fn is_username_available(username: String) -> Result<bool, error::Error> {
        let existing: Option<User> = DB
            .query("SELECT * FROM users WHERE username = $username")
            .bind(("username", username))
            .await?
            .take(0)?;
            
        Ok(existing.is_none())
    }
    
    /// Check if email is available
    pub async fn is_email_available(email: String) -> Result<bool, error::Error> {
        let existing: Option<User> = DB
            .query("SELECT * FROM users WHERE email = $email")
            .bind(("email", email))
            .await?
            .take(0)?;
            
        Ok(existing.is_none())
    }
    
    /// Ban user
    pub async fn ban_user(user_id: Uuid) -> Result<User, error::Error> {
        let mut user = Self::get_user_by_id(user_id).await?;
        
        if let Some(ref mut profile) = user.profile {
            profile.is_banned = true;
        }
        
        user.updated_at = Utc::now();
        
        let updated_user: Option<User> = DB
            .update(("users", user_id.to_string()))
            .content(user)
            .await?;
            
        updated_user.ok_or(error::Error::Db("Failed to ban user".to_string()))
    }
    
    /// Unban user
    pub async fn unban_user(user_id: Uuid) -> Result<User, error::Error> {
        let mut user = Self::get_user_by_id(user_id).await?;
        
        if let Some(ref mut profile) = user.profile {
            profile.is_banned = false;
        }
        
        user.updated_at = Utc::now();
        
        let updated_user: Option<User> = DB
            .update(("users", user_id.to_string()))
            .content(user)
            .await?;
            
        updated_user.ok_or(error::Error::Db("Failed to unban user".to_string()))
    }
    
    /// Update user last login
    pub async fn update_last_login(user_id: Uuid) -> Result<User, error::Error> {
        let mut user = Self::get_user_by_id(user_id).await?;
        let now = Utc::now();
        
        if let Some(ref mut profile) = user.profile {
            profile.last_login = Some(now);
            profile.last_activity = Some(now);
        }
        
        user.updated_at = now;
        
        let updated_user: Option<User> = DB
            .update(("users", user_id.to_string()))
            .content(user)
            .await?;
            
        updated_user.ok_or(error::Error::Db("Failed to update last login".to_string()))
    }
}

#[derive(Debug, serde::Serialize)]
pub struct UserStats {
    pub total_users: u64,
    pub verified_users: u64,
    pub active_users: u64,
}