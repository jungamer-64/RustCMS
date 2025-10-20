// @generated automatically by Diesel CLI.diesel: コマンドが見つかりません

// Phase 9: Manually generated from migrations/003_production_schema.sql
// Note: Schema aligned with DbUser model in models.rs

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Nullable<Varchar>,
        first_name -> Nullable<Text>,
        last_name -> Nullable<Text>,
        role -> Varchar,
        is_active -> Bool,
        email_verified -> Bool,
        last_login -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    webauthn_credentials (id) {
        id -> Uuid,
        user_id -> Uuid,
        credential_id -> Bytea,
        public_key -> Bytea,
        counter -> Int8,
        device_name -> Nullable<Varchar>,
        created_at -> Timestamptz,
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
    categories (id) {
        id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Text>,
        parent_id -> Nullable<Uuid>,
        sort_order -> Int4,
        post_count -> Int4,
        // Legacy field for new structure
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    tags (id) {
        id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Text>,
        color -> Nullable<Varchar>,
        post_count -> Int4,
        // Legacy fields for new structure
        usage_count -> Int4,
        updated_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    post_categories (post_id, category_id) {
        post_id -> Uuid,
        category_id -> Uuid,
    }
}

diesel::table! {
    post_tags (post_id, tag_id) {
        post_id -> Uuid,
        tag_id -> Uuid,
    }
}

diesel::table! {
    comments (id) {
        id -> Uuid,
        post_id -> Uuid,
        author_id -> Uuid,
        content -> Text,
        parent_id -> Nullable<Uuid>,
        is_approved -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    media (id) {
        id -> Uuid,
        filename -> Varchar,
        original_filename -> Varchar,
        file_path -> Text,
        mime_type -> Varchar,
        file_size -> Int8,
        width -> Nullable<Int4>,
        height -> Nullable<Int4>,
        uploaded_by -> Uuid,
        title -> Nullable<Varchar>,
        alt_text -> Nullable<Text>,
        caption -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    api_keys (id) {
        id -> Uuid,
        user_id -> Uuid,
        key_hash -> Varchar,
        name -> Varchar,
        permissions -> Array<Text>,
        rate_limit -> Int4,
        expires_at -> Nullable<Timestamptz>,
        last_used_at -> Nullable<Timestamptz>,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    sessions (id) {
        id -> Varchar,
        user_id -> Uuid,
        data -> Jsonb,
        expires_at -> Timestamptz,
        created_at -> Timestamptz,
        last_accessed_at -> Timestamptz,
    }
}

diesel::table! {
    audit_logs (id) {
        id -> Uuid,
        user_id -> Nullable<Uuid>,
        action -> Varchar,
        entity_type -> Varchar,
        entity_id -> Nullable<Uuid>,
        old_values -> Nullable<Jsonb>,
        new_values -> Nullable<Jsonb>,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(webauthn_credentials -> users (user_id));
diesel::joinable!(posts -> users (author_id));
diesel::joinable!(post_categories -> posts (post_id));
diesel::joinable!(post_categories -> categories (category_id));
diesel::joinable!(post_tags -> posts (post_id));
diesel::joinable!(post_tags -> tags (tag_id));
diesel::joinable!(comments -> posts (post_id));
diesel::joinable!(comments -> users (author_id));
diesel::joinable!(media -> users (uploaded_by));
diesel::joinable!(api_keys -> users (user_id));
diesel::joinable!(sessions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    users,
    webauthn_credentials,
    posts,
    categories,
    tags,
    post_categories,
    post_tags,
    comments,
    media,
    api_keys,
    sessions,
    audit_logs,
);
