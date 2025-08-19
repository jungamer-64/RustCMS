// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Nullable<Varchar>,
        first_name -> Nullable<Varchar>,
        last_name -> Nullable<Varchar>,
        display_name -> Nullable<Varchar>,
        avatar_url -> Nullable<Varchar>,
        bio -> Nullable<Text>,
        role -> Varchar,
        is_active -> Bool,
        email_verified -> Bool,
        dark_mode -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        last_login -> Nullable<Timestamptz>,
        failed_login_attempts -> Int4,
        locked_until -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    webauthn_credentials (id) {
        id -> Uuid,
        user_id -> Uuid,
        credential_id -> Bytea,
        public_key -> Bytea,
        counter -> Bigint,
        name -> Varchar,
        backup_eligible -> Bool,
        backup_state -> Bool,
        created_at -> Timestamptz,
        last_used -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    posts (id) {
        id -> Uuid,
        title -> Varchar,
        slug -> Varchar,
        content -> Text,
        excerpt -> Nullable<Text>,
        featured_image -> Nullable<Varchar>,
        status -> Varchar,
        author_id -> Uuid,
        published_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        tags -> Nullable<Array<Text>>,
        category -> Nullable<Varchar>,
        meta_title -> Nullable<Varchar>,
        meta_description -> Nullable<Text>,
        view_count -> Int8,
    }
}

diesel::table! {
    pages (id) {
        id -> Uuid,
        title -> Varchar,
        slug -> Varchar,
        content -> Text,
        template -> Nullable<Varchar>,
        is_published -> Bool,
        author_id -> Uuid,
        parent_id -> Nullable<Uuid>,
        sort_order -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        meta_title -> Nullable<Varchar>,
        meta_description -> Nullable<Text>,
    }
}

diesel::table! {
    media (id) {
        id -> Uuid,
        filename -> Varchar,
        original_name -> Varchar,
        mime_type -> Varchar,
        file_size -> Int8,
        path -> Varchar,
        url -> Varchar,
        alt_text -> Nullable<Varchar>,
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
        is_public -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    api_keys (id) {
        id -> Uuid,
        name -> Varchar,
        key_hash -> Varchar,
        user_id -> Uuid,
        permissions -> Array<Text>,
        last_used -> Nullable<Timestamptz>,
        expires_at -> Nullable<Timestamptz>,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    user_sessions (id) {
        id -> Uuid,
        user_id -> Uuid,
        session_token -> Varchar,
        expires_at -> Timestamptz,
        user_agent -> Nullable<Text>,
        ip_address -> Nullable<Inet>,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    post_revisions (id) {
        id -> Uuid,
        post_id -> Uuid,
        title -> Varchar,
        content -> Text,
        excerpt -> Nullable<Text>,
        author_id -> Uuid,
        revision_number -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    webhooks (id) {
        id -> Uuid,
        name -> Varchar,
        url -> Varchar,
        events -> Array<Text>,
        secret -> Nullable<Varchar>,
        is_active -> Bool,
        created_by -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        last_triggered -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(webauthn_credentials -> users (user_id));
diesel::joinable!(posts -> users (author_id));
diesel::joinable!(pages -> users (author_id));
diesel::joinable!(media -> users (uploaded_by));
diesel::joinable!(api_keys -> users (user_id));
diesel::joinable!(user_sessions -> users (user_id));
diesel::joinable!(post_revisions -> posts (post_id));
diesel::joinable!(post_revisions -> users (author_id));
diesel::joinable!(webhooks -> users (created_by));

diesel::allow_tables_to_appear_in_same_query!(
    users,
    webauthn_credentials,
    posts,
    pages,
    media,
    settings,
    api_keys,
    user_sessions,
    post_revisions,
    webhooks,
);
