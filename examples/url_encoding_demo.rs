use cms_backend::utils::url_encoding::*;

fn main() {
    println!("🔗 URL エンコード機能デモンストレーション\n");

    // 基本的なURLエンコード
    println!("=== 基本URLエンコード ===");
    let test_string = "こんにちは 世界!";
    let encoded = encode_url_param(test_string);
    println!("原文: {}", test_string);
    println!("エンコード: {}", encoded);
    println!("デコード: {}\n", decode_url_param(&encoded).unwrap());

    // スラッグ生成
    println!("=== 安全なスラッグ生成 ===");
    let titles = vec![
        "Hello World!",
        "こんにちは世界",
        "Rust is awesome 🦀",
        "C++からRustへの移行",
        "API設計のベストプラクティス",
    ];

    for title in titles {
        let slug = generate_safe_slug(title);
        println!("タイトル: {} → スラッグ: {}", title, slug);
    }
    println!();

    // URLパラメータ検証
    println!("=== URLパラメータ検証 ===");
    let long_param = "x".repeat(3000);
    let test_params = vec![
        "valid-param",
        "こんにちは",
        &long_param, // 長すぎるパラメータ
        "param with spaces",
    ];

    for param in test_params {
        match validate_url_param(param) {
            Ok(_) => println!(
                "✅ '{}' は有効です",
                if param.len() > 50 {
                    &param[..50]
                } else {
                    param
                }
            ),
            Err(e) => println!(
                "❌ '{}' は無効です: {}",
                if param.len() > 50 {
                    &param[..50]
                } else {
                    param
                },
                e
            ),
        }
    }
    println!();

    // 日本語スラッグのテスト
    println!("=== 日本語スラッグのエンコード/デコード ===");
    let japanese_slug = "プログラミング-言語-比較";
    let encoded_slug = encode_slug(japanese_slug);
    let decoded_slug = decode_slug(&encoded_slug).unwrap();

    println!("原文スラッグ: {}", japanese_slug);
    println!("エンコード: {}", encoded_slug);
    println!("デコード: {}", decoded_slug);
    println!("一致: {}", japanese_slug == decoded_slug);
    println!();

    // 二重エンコード防止のテスト
    println!("=== 二重エンコード防止 ===");
    let already_encoded = "hello%20world";
    let encoded_again = encode_url_param(already_encoded);
    println!("既にエンコード済み: {}", already_encoded);
    println!("再エンコード結果: {}", encoded_again);
    println!("変化なし: {}", already_encoded == encoded_again);

    println!("\n🎉 URLエンコード機能のデモが完了しました！");
}
