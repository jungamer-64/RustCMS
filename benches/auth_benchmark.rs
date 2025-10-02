//! Authentication Performance Benchmarks
//!
//! Benchmarks for:
//! - Token generation
//! - Token verification
//! - Password hashing
//! - Biscuit operations

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cms_backend::auth::biscuit::{BiscuitAuthService, BiscuitConfig};
use cms_backend::models::user::UserRole;
use uuid::Uuid;

fn setup_auth_service() -> BiscuitAuthService {
    let config = BiscuitConfig {
        root_key_path: "./biscuit_keys/root.key".to_string(),
        public_key_path: "./biscuit_keys/public.key".to_string(),
        token_expiry_seconds: 3600,
        refresh_token_expiry_seconds: 86400,
    };
    BiscuitAuthService::new(config).expect("Failed to create auth service")
}

fn bench_token_generation(c: &mut Criterion) {
    let auth_service = setup_auth_service();
    let user_id = Uuid::new_v4();
    let username = "benchmark_user";
    let permissions = vec!["read:posts".to_string(), "write:posts".to_string()];

    c.bench_function("token_generation", |b| {
        b.iter(|| {
            auth_service.generate_token(
                black_box(user_id),
                black_box(username),
                black_box(UserRole::User),
                black_box(permissions.clone()),
            )
        })
    });
}

fn bench_token_verification(c: &mut Criterion) {
    let auth_service = setup_auth_service();
    let user_id = Uuid::new_v4();
    let token = auth_service
        .generate_token(
            user_id,
            "benchmark_user",
            UserRole::User,
            vec!["read:posts".to_string()],
        )
        .expect("Failed to generate token");

    c.bench_function("token_verification", |b| {
        b.iter(|| auth_service.verify_token(black_box(&token)))
    });
}

fn bench_token_generation_by_role(c: &mut Criterion) {
    let auth_service = setup_auth_service();
    let mut group = c.benchmark_group("token_generation_by_role");
    
    let roles = vec![
        ("User", UserRole::User),
        ("Editor", UserRole::Editor),
        ("Admin", UserRole::Admin),
    ];

    for (role_name, role) in roles {
        group.bench_with_input(BenchmarkId::from_parameter(role_name), &role, |b, role| {
            let user_id = Uuid::new_v4();
            let permissions = vec!["read:posts".to_string(), "write:posts".to_string()];
            
            b.iter(|| {
                auth_service.generate_token(
                    black_box(user_id),
                    black_box("benchmark_user"),
                    black_box(*role),
                    black_box(permissions.clone()),
                )
            });
        });
    }
    
    group.finish();
}

fn bench_password_hashing(c: &mut Criterion) {
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::SaltString;
    use rand_core::OsRng;

    let argon2 = Argon2::default();
    let password = b"secure_password_123";

    c.bench_function("password_hashing", |b| {
        b.iter(|| {
            let salt = SaltString::generate(&mut OsRng);
            argon2.hash_password(black_box(password), black_box(&salt))
        })
    });
}

fn bench_concurrent_token_generation(c: &mut Criterion) {
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    let rt = Runtime::new().unwrap();
    let auth_service = Arc::new(setup_auth_service());

    c.bench_function("concurrent_token_generation_10", |b| {
        b.to_async(&rt).iter(|| {
            let auth_service = auth_service.clone();
            async move {
                let handles: Vec<_> = (0..10)
                    .map(|_| {
                        let auth_service = auth_service.clone();
                        tokio::spawn(async move {
                            let user_id = Uuid::new_v4();
                            auth_service.generate_token(
                                user_id,
                                "benchmark_user",
                                UserRole::User,
                                vec!["read:posts".to_string()],
                            )
                        })
                    })
                    .collect();

                for handle in handles {
                    let _ = handle.await;
                }
            }
        });
    });
}

criterion_group!(
    benches,
    bench_token_generation,
    bench_token_verification,
    bench_token_generation_by_role,
    bench_password_hashing,
    bench_concurrent_token_generation
);
criterion_main!(benches);
