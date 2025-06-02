use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};


pub enum CreatedVia {
    Web,
    Mobile,
    Google,
    Spotify,
    SoundCloud
}

pub struct Report {
    pub id: Uuid,
    pub user_id: Uuid,
    pub reason: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: ReportStatus,
}

pub enum ReportStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
}

pub struct Comment {
    pub id: Uuid,
    pub referred_track_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_deleted: bool,
    pub replies: Option<Vec<Comment>>,
    pub likes: Option<Vec<Uuid>>,
    pub dislikes: Option<Vec<Uuid>>,
    pub is_pinned: bool,
    pub reports: Option<Vec<Report>>,
}


pub struct Track {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub audio_url: String,
    pub cover_image_url: Option<String>,
    pub genre: Option<String>,
    pub tags: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_public: bool,
    pub is_deleted: bool,
    pub likes: u32,
    pub dislikes: u32,
    pub comments: Option<Vec<Comment>>,
}

pub struct UserProfile {
    pub profile_name: String,
    pub pronouns: Option<String>,
    pub location: Option<String>,
    pub social_links: Option<Vec<String>>,
    pub profile_banner: Option<String>,
    pub profile_picture: Option<String>,
    pub profile_bio: Option<String>,
    pub social_links: Option<Vec<String>>,
    pub profile_views: u32,
    pub friends_list: Option<Vec<Uuid>>,
    pub blocked_users: Option<Vec<Uuid>>,
    pub is_private: bool,
    pub uploads: Option<Vec<Track>>,
    pub followers: Option<Vec<Uuid>>,
    pub following: Option<Vec<Uuid>>,
    pub last_login: Option<DateTime<Utc>>,
    pub last_activity: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub is_admin: bool,
    pub is_banned: bool,
    pub is_deleted: bool,
    pub reports: Option<Vec<Report>>,
}

pub struct Playlist {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub cover_image_url: Option<String>,
    pub is_public: bool,
    pub is_deleted: bool,
    pub is_collaborative: bool,
    pub tracks: Vec<Track>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_public: bool,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub hashed_password: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub bio: Option<String>,
    pub created_via: CreatedVia,
    pub profile: Option<UserProfile>,
    pub email_verified: bool,
    pub playlists: Option<Vec<Playlist>>,
}