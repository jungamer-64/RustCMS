//! Authentication Performance Benchmarks
//!
//! Comprehensive benchmarks for authentication operations:
//! - Token generation (access, refresh, Biscuit)
//! - Token verification and validation
//! - Password hashing with Argon2
//! - Concurrent authentication scenarios
//! - Role-based token generation
//!
//! # Performance Targets
//! - Token generation: < 10ms per token
//! - Token verification: < 5ms per verification
//! - Password hashing: 100-500ms (security vs performance trade-off)
//! - Concurrent ops: Linear scaling up to CPU cores

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cms_backend::auth::biscuit::{BiscuitAuthService, BiscuitConfig};
use cms_backend::models::user::UserRole;
use uuid::Uuid;
use std::sync::Arc;
use tokio::runtime::Runtime;

// ============================================================================
// Setup and Configuration
// ============================================================================

/// Create authentication service for benchmarking
fn create_auth_service() -> BiscuitAuthService {
    let config = BiscuitConfig {
        root_key_path: "./biscuit_keys/root.key".to_string(),
        public_key_path: "./biscuit_keys/public.key".to_string(),
        token_expiry_seconds: 3600,
        refresh_token_expiry_seconds: 86400,
    };
    
    BiscuitAuthService::new(config)
        .expect("Failed to initialize authentication service")
}

/// Generate test permissions set
fn create_test_permissions() -> Vec<String> {
    vec![
        "read:posts".to_string(),
        "write:posts".to_string(),
        "read:users".to_string(),
    ]
}

/// Generate extended permissions for admin role
fn create_admin_permissions() -> Vec<String> {
    vec![
        "read:posts".to_string(),
        "write:posts".to_string(),
        "delete:posts".to_string(),
        "read:users".to_string(),
        "write:users".to_string(),
        "delete:users".to_string(),
        "admin:system".to_string(),
    ]
}

// ============================================================================
// Token Generation Benchmarks
// ============================================================================

/// Benchmark basic token generation
fn bench_token_generation(c: &mut Criterion) {
    let auth_service = create_auth_service();
    let user_id = Uuid::new_v4();
    let username = "benchmark_user";
    let permissions = create_test_permissions();

    c.bench_function("auth/token_generation", |b| {
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

/// Benchmark token generation by role
fn bench_token_generation_by_role(c: &mut Criterion) {
    let auth_service = create_auth_service();
    let mut group = c.benchmark_group("auth/token_by_role");
    
    let roles = vec![
        ("User", UserRole::User, create_test_permissions()),
        ("Editor", UserRole::Editor, create_test_permissions()),
        ("Admin", UserRole::Admin, create_admin_permissions()),
    ];

    for (role_name, role, permissions) in roles {
        group.bench_with_input(
            BenchmarkId::from_parameter(role_name),
            &(role, permissions),
            |b, (role, perms)| {
                let user_id = Uuid::new_v4();
                b.iter(|| {
                    auth_service.generate_token(
                        black_box(user_id),
                        black_box("benchmark_user"),
                        black_box(*role),
                        black_box(perms.clone()),
                    )
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark token generation with varying permission counts
fn bench_token_generation_by_permission_count(c: &mut Criterion) {
    let auth_service = create_auth_service();
    let mut group = c.benchmark_group("auth/token_by_permission_count");
    
    for count in [1, 5, 10, 20, 50].iter() {
        let permissions: Vec<String> = (0..*count)
            .map(|i| format!("permission:{}", i))
            .collect();
        
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &permissions,
            |b, perms| {
                let user_id = Uuid::new_v4();
                b.iter(|| {
                    auth_service.generate_token(
                        black_box(user_id),
                        black_box("benchmark_user"),
                        black_box(UserRole::User),
                        black_box(perms.clone()),
                    )
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Token Verification Benchmarks
// ============================================================================

/// Benchmark token verification
fn bench_token_verification(c: &mut Criterion) {
    let auth_service = create_auth_service();
    let user_id = Uuid::new_v4();
    let token = auth_service
        .generate_token(
            user_id,
            "benchmark_user",
            UserRole::User,
            create_test_permissions(),
        )
        .expect("Failed to generate test token");

    c.bench_function("auth/token_verification", |b| {
        b.iter(|| auth_service.verify_token(black_box(&token)))
    });
}

/// Benchmark verification of tokens with different roles
fn bench_token_verification_by_role(c: &mut Criterion) {
    let auth_service = create_auth_service();
    let mut group = c.benchmark_group("auth/verification_by_role");
    
    let roles = vec![
        ("User", UserRole::User),
        ("Editor", UserRole::Editor),
        ("Admin", UserRole::Admin),
    ];

    for (role_name, role) in roles {
        let token = auth_service
            .generate_token(
                Uuid::new_v4(),
                "benchmark_user",
                role,
                create_test_permissions(),
            )
            .expect("Failed to generate test token");
        
        group.bench_with_input(
            BenchmarkId::from_parameter(role_name),
            &token,
            |b, token| {
                b.iter(|| auth_service.verify_token(black_box(token)));
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Password Hashing Benchmarks
// ============================================================================

/// Benchmark password hashing with Argon2
fn bench_password_hashing(c: &mut Criterion) {
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::SaltString;
    use rand_core::OsRng;

    let argon2 = Argon2::default();
    let password = b"secure_password_with_sufficient_length";

    c.bench_function("auth/password_hashing", |b| {
        b.iter(|| {
            let salt = SaltString::generate(&mut OsRng);
            argon2.hash_password(black_box(password), black_box(&salt))
        })
    });
}

/// Benchmark password hashing with different configurations
fn bench_password_hashing_configs(c: &mut Criterion) {
    use argon2::{Argon2, PasswordHasher, Algorithm, Version, Params};
    use argon2::password_hash::SaltString;
    use rand_core::OsRng;

    let mut group = c.benchmark_group("auth/password_hashing_configs");
    let password = b"secure_password_with_sufficient_length";
    
    let configs = vec![
        ("default", Argon2::default()),
        ("low_memory", {
            let params = Params::new(8192, 2, 1, None).unwrap();
            Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        }),
        ("high_security", {
            let params = Params::new(65536, 4, 2, None).unwrap();
            Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        }),
    ];

    for (name, argon2) in configs {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &argon2,
            |b, hasher| {
                b.iter(|| {
                    let salt = SaltString::generate(&mut OsRng);
                    hasher.hash_password(black_box(password), black_box(&salt))
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Concurrent Operations Benchmarks
// ============================================================================

/// Benchmark concurrent token generation
fn bench_concurrent_token_generation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let auth_service = Arc::new(create_auth_service());
    let mut group = c.benchmark_group("auth/concurrent_token_generation");

    for concurrency in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &count| {
                b.to_async(&rt).iter(|| {
                    let auth_service = auth_service.clone();
                    async move {
                        let handles: Vec<_> = (0..count)
                            .map(|_| {
                                let auth_service = auth_service.clone();
                                tokio::spawn(async move {
                                    let user_id = Uuid::new_v4();
                                    auth_service.generate_token(
                                        user_id,
                                        "benchmark_user",
                                        UserRole::User,
                                        create_test_permissions(),
                                    )
                                })
                            })
                            .collect();

                        for handle in handles {
                            let _ = handle.await;
                        }
                    }
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark concurrent token verification
fn bench_concurrent_token_verification(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let auth_service = Arc::new(create_auth_service());
    
    // Generate tokens for verification
    let tokens: Vec<_> = (0..100)
        .map(|_| {
            auth_service
                .generate_token(
                    Uuid::new_v4(),
                    "benchmark_user",
                    UserRole::User,
                    create_test_permissions(),
                )
                .expect("Failed to generate token")
        })
        .collect();
    
    let tokens = Arc::new(tokens);
    let mut group = c.benchmark_group("auth/concurrent_token_verification");

    for concurrency in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &count| {
                let tokens = tokens.clone();
                b.to_async(&rt).iter(|| {
                    let auth_service = auth_service.clone();
                    let tokens = tokens.clone();
                    async move {
                        let handles: Vec<_> = (0..count)
                            .map(|i| {
                                let auth_service = auth_service.clone();
                                let token = tokens[i % tokens.len()].clone();
                                tokio::spawn(async move {
                                    auth_service.verify_token(&token)
                                })
                            })
                            .collect();

                        for handle in handles {
                            let _ = handle.await;
                        }
                    }
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Mixed Workload Benchmarks
// ============================================================================

/// Benchmark mixed authentication workload (generation + verification)
fn bench_mixed_auth_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let auth_service = Arc::new(create_auth_service());
    
    // Pre-generate some tokens for verification
    let tokens: Vec<_> = (0..50)
        .map(|_| {
            auth_service
                .generate_token(
                    Uuid::new_v4(),
                    "benchmark_user",
                    UserRole::User,
                    create_test_permissions(),
                )
                .expect("Failed to generate token")
        })
        .collect();
    
    let tokens = Arc::new(tokens);

    c.bench_function("auth/mixed_workload", |b| {
        b.to_async(&rt).iter(|| {
            let auth_service = auth_service.clone();
            let tokens = tokens.clone();
            async move {
                let handles: Vec<_> = (0..20)
                    .map(|i| {
                        let auth_service = auth_service.clone();
                        let tokens = tokens.clone();
                        tokio::spawn(async move {
                            if i % 3 == 0 {
                                // 33% token generation
                                auth_service.generate_token(
                                    Uuid::new_v4(),
                                    "benchmark_user",
                                    UserRole::User,
                                    create_test_permissions(),
                                )
                            } else {
                                // 67% token verification
                                let token = &tokens[i % tokens.len()];
                                auth_service.verify_token(token).map(|_| ())
                            }
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

// ============================================================================
// Benchmark Group Configuration
// ============================================================================

criterion_group!(
    name = token_benches;
    config = Criterion::default();
    targets = 
        bench_token_generation,
        bench_token_generation_by_role,
        bench_token_generation_by_permission_count,
        bench_token_verification,
        bench_token_verification_by_role,
);

criterion_group!(
    name = password_benches;
    config = Criterion::default();
    targets = 
        bench_password_hashing,
        bench_password_hashing_configs,
);

criterion_group!(
    name = concurrent_benches;
    config = Criterion::default();
    targets = 
        bench_concurrent_token_generation,
        bench_concurrent_token_verification,
        bench_mixed_auth_workload,
);

criterion_main!(token_benches, password_benches, concurrent_benches);