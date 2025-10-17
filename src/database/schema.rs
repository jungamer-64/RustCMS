// src/database/schema.rs
// @generated automatically by Diesel CLI.

diesel::table! {
    api_keys (id) {
        id -> Uuid,
        name -> Varchar,
        key_hash -> Varchar,
    api_key_lookup_hash -> Varchar,
        user_id -> Uuid,
        permissions -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        expires_at -> Nullable<Timestamptz>,
        last_used_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    posts (id) {
        id -> Uuid,
        title -> Varchar,
        slug -> Varchar,
        content -> Text,
        excerpt -> Nullable<Text>,
        author_id -> Uuid,
        status -> Varchar,
        featured_image_id -> Nullable<Uuid>,
        tags -> Array<Text>,
        categories -> Array<Text>,
        meta_title -> Nullable<Varchar>,
        meta_description -> Nullable<Text>,
        published_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Nullable<Varchar>,
        first_name -> Nullable<Varchar>,
        last_name -> Nullable<Varchar>,
        role -> Varchar,
        is_active -> Bool,
        email_verified -> Bool,
        last_login -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    pages (id) {
        id -> Uuid,
        title -> Varchar,
        slug -> Varchar,
        content -> Text,
        author_id -> Uuid,
        status -> Varchar,
        meta_title -> Nullable<Varchar>,
        meta_description -> Nullable<Text>,
        published_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    media_files (id) {
        id -> Uuid,
        filename -> Varchar,
        original_filename -> Varchar,
        mime_type -> Varchar,
        file_size -> Int8,
        file_path -> Varchar,
        alt_text -> Nullable<Text>,
        caption -> Nullable<Text>,
        uploaded_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    settings (id) {
        id -> Uuid,
        key -> Varchar,
        value -> Jsonb,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    comments (id) {
        id -> Uuid,
        post_id -> Uuid,
        author_id -> Nullable<Uuid>,
        author_name -> Nullable<Varchar>,
        author_email -> Nullable<Varchar>,
        content -> Text,
        status -> Varchar,
        ip_address -> Nullable<Inet>,
        user_agent -> Nullable<Text>,
        parent_id -> Nullable<Uuid>,
        like_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(api_keys -> users (user_id));
diesel::joinable!(posts -> users (author_id));
diesel::joinable!(pages -> users (author_id));
diesel::joinable!(media_files -> users (uploaded_by));
diesel::joinable!(posts -> media_files (featured_image_id));
diesel::joinable!(comments -> posts (post_id));
diesel::joinable!(comments -> users (author_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_keys,
    posts,
    users,
    pages,
    media_files,
    settings,
    comments,
);
