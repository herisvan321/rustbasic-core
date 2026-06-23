use crate::app::Config;
use crate::schema::MigratorTrait;
use crate::seeder::SeederTrait;
use crate::colored::Colorize;

/// Entry point CLI utama — dipanggil oleh project's cli.rs
///
/// M = MigratorTrait  (dari project/database/migrations)
/// S = SeederTrait    (dari project/database/seeders)
pub async fn handle<
    M: MigratorTrait + Send + Sync + 'static,
    S: SeederTrait + Send + Sync + 'static,
>(
    args: &[String],
    cfg: &Config,
    seeder: Option<S>,
) -> bool {
    if args.len() < 2 {
        return false;
    }

    let command = args[1].as_str();

    let is_migration_cmd = command.starts_with("migrate") || command == "db:seed";
    let is_storage_cmd   = command == "storage:link";
    let is_server_cmd    = command == "server" || command == "serve";
    let is_build_cmd     = command == "build";
    let is_make_cmd      = command.starts_with("make:");
    let is_route_cmd     = command == "route:list";
    let is_key_cmd       = command == "key:generate";
    let is_deploy_cmd    = command == "deploy";
    let is_publish_cmd   = command == "publish";

    // server --android / --desktop → jalankan native runner
    if is_server_cmd {
        let run_android = args.iter().any(|arg| arg == "--android");
        let run_desktop = args.iter().any(|arg| arg == "--desktop");
        if run_android || run_desktop {
            run_native(run_android, run_desktop);
            return true;
        }
        return false; // fall through ke standard web server
    }

    if !is_migration_cmd && !is_storage_cmd && !is_build_cmd
        && !is_make_cmd && !is_route_cmd && !is_key_cmd && !is_deploy_cmd
        && !is_publish_cmd {
        return false;
    }

    println!("🛠️  RustBasic CLI - Command: {}", command);

    if is_build_cmd {
        handle_build(args).await;
        return true;
    }

    if is_deploy_cmd {
        handle_deploy().await;
        return true;
    }

    if is_storage_cmd {
        handle_storage_link(cfg);
        return true;
    }

    if is_make_cmd {
        handle_make(args);
        return true;
    }

    if is_route_cmd {
        handle_route_list();
        return true;
    }

    if is_key_cmd {
        handle_key_generate();
        return true;
    }

    if is_publish_cmd {
        let target = args.get(2).map(|s| s.as_str()).unwrap_or("");
        handle_publish(target);
        return true;
    }

    // Perintah database — perlu koneksi pool
    let pool = crate::database::connect(cfg).await;

    match command {
        "migrate" => {
            println!("🚀 Menjalankan migrasi database...");
            if let Err(e) = M::up(&pool, None).await {
                println!("❌ Gagal menjalankan migrasi: {}", e);
            } else {
                println!("✅ Migrasi selesai!");
            }
        }
        "migrate:refresh" => {
            println!("🔄 Mereset dan menjalankan ulang migrasi...");
            if let Err(e) = M::fresh(&pool).await {
                println!("❌ Gagal refresh migrasi: {}", e);
            } else {
                println!("✅ Database berhasil di-refresh!");
            }
        }
        "migrate:back" | "migrate:rollback" => {
            println!("⬅️  Rollback migrasi terakhir...");
            if let Err(e) = M::down(&pool, None).await {
                println!("❌ Gagal rollback: {}", e);
            } else {
                println!("✅ Rollback berhasil!");
            }
        }
        "db:seed" => {
            println!("🌱 Menjalankan database seeder...");
            match seeder {
                Some(s) => {
                    if let Err(e) = s.run(&pool).await {
                        println!("❌ Seeder gagal: {}", e);
                    } else {
                        println!("✅ Seeder selesai!");
                    }
                }
                None => {
                    println!("⚠️  Tidak ada seeder yang terdaftar.");
                }
            }
        }
        _ => return false,
    }

    true
}

// ============================================================
// KEY:GENERATE
// ============================================================
fn handle_key_generate() {
    let key = crate::rand::random_alphanumeric(32);
    let encoded = crate::base64::encode(key.as_bytes());

    // Baca .env, update APP_KEY, tulis balik
    let env_path = std::path::Path::new(".env");
    if env_path.exists() {
        let content = std::fs::read_to_string(env_path).unwrap_or_default();
        let new_content = if content.contains("APP_KEY=") {
            let mut lines: Vec<String> = content
                .lines()
                .map(|line| {
                    if line.starts_with("APP_KEY=") {
                        format!("APP_KEY={}", encoded)
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            // pastikan ada newline di akhir
            if !lines.last().map(|l| l.is_empty()).unwrap_or(true) {
                lines.push(String::new());
            }
            lines.join("\n")
        } else {
            format!("{}\nAPP_KEY={}\n", content.trim_end(), encoded)
        };
        if let Err(e) = std::fs::write(env_path, new_content) {
            println!("❌ Gagal menulis ke .env: {}", e);
            return;
        }
        println!("✅ APP_KEY berhasil dibuat dan disimpan ke .env");
        println!("   Key: {}", encoded);
    } else {
        println!("⚠️  File .env tidak ditemukan. Key yang dibuat:");
        println!("   APP_KEY={}", encoded);
    }
}

// ============================================================
// ROUTE:LIST
// ============================================================
fn handle_route_list() {
    // Baca route definitions dari src/routes/ secara statis (parse file)
    let route_files = ["src/routes/web.rs", "src/routes/api.rs"];
    let mut routes: Vec<(String, String, String)> = Vec::new(); // (method, path, handler)

    for file_path in &route_files {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            for line in content.lines() {
                let line = line.trim();
                // Match pola: .route("/path", get(handler)).name("x")
                // atau: .route("/path", post(handler))
                if line.starts_with(".route(") || line.contains(".route(\"") {
                    parse_route_line(line, &mut routes);
                }
            }
        }
    }

    if routes.is_empty() {
        println!("ℹ️  Tidak ada rute ditemukan atau format rute tidak dikenal.");
        println!("   Cek file src/routes/web.rs dan src/routes/api.rs");
        return;
    }

    // Header tabel
    println!("\n{}", "=".repeat(72));
    println!("  {:<8} {:<35} {}", "METHOD", "PATH", "HANDLER");
    println!("{}", "=".repeat(72));
    for (method, path, handler) in &routes {
        println!("  {:<8} {:<35} {}", method, path, handler);
    }
    println!("{}\n", "=".repeat(72));
    println!("  Total: {} rute terdaftar", routes.len());
}

fn parse_route_line(line: &str, routes: &mut Vec<(String, String, String)>) {
    // Cari path string (content antara tanda kutip pertama)
    let methods = ["get", "post", "put", "patch", "delete"];
    
    // Ekstrak path dari .route("path", ...)
    let path = if let Some(start) = line.find(".route(\"") {
        let after = &line[start + 8..];
        if let Some(end) = after.find('"') {
            after[..end].to_string()
        } else { return; }
    } else { return; };

    // Ekstrak method dan handler
    for method in &methods {
        let pattern = format!("{}(", method);
        if let Some(pos) = line.find(&pattern) {
            let after = &line[pos + method.len() + 1..];
            let handler = if let Some(end) = after.find(')') {
                after[..end].to_string()
            } else {
                after.to_string()
            };
            routes.push((method.to_uppercase(), path.clone(), handler));
            break;
        }
    }
}

// ============================================================
// MAKE:* GENERATOR
// ============================================================
fn handle_make(args: &[String]) {
    let subcommand = args[1].as_str(); // e.g. "make:controller"
    let name = args.get(2).map(|s| s.as_str()).unwrap_or("");

    if name.is_empty() {
        println!("❌ Nama diperlukan. Contoh: rustbasic {} MyName", subcommand);
        return;
    }

    match subcommand {
        "make:controller" => make_controller(name),
        "make:model" => {
            let with_migration = args.iter().any(|a| a == "-m" || a == "--migration");
            make_model(name, with_migration);
        }
        "make:middleware" => make_middleware(name),
        "make:observer" => {
            let model = args.iter().find(|a| a.starts_with("--model="))
                .and_then(|a| a.strip_prefix("--model="))
                .unwrap_or("Model");
            make_observer(name, model);
        }
        "make:service" => make_service(name),
        "make:seeder"  => make_seeder(name),
        "make:migration" => make_migration(name),
        _ => {
            println!("❌ Subcommand tidak dikenal: {}", subcommand);
        }
    }
}

/// Konversi PascalCase → snake_case
fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// Tulis file — buat folder parent jika belum ada
fn write_file(path: &str, content: &str) {
    let p = std::path::Path::new(path);
    if let Some(parent) = p.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            println!("❌ Gagal membuat direktori {}: {}", parent.display(), e);
            return;
        }
    }
    if p.exists() {
        println!("⚠️  File sudah ada, dilewati: {}", path);
        return;
    }
    if let Err(e) = std::fs::write(p, content) {
        println!("❌ Gagal membuat file {}: {}", path, e);
    } else {
        println!("✅ Dibuat: {}", path);
    }
}

/// Tambahkan baris `pub mod <name>;` ke mod.rs jika belum ada
fn register_mod(mod_file: &str, module_name: &str) {
    let line = format!("pub mod {};\n", module_name);
    let content = std::fs::read_to_string(mod_file).unwrap_or_default();
    if content.contains(&line) { return; }

    let label = "// 📑 LABEL: MODULE";
    let new_content = if let Some(pos) = content.find(label) {
        // Sisipkan setelah baris label
        let nl = content[pos..].find('\n').map(|n| pos + n + 1).unwrap_or(content.len());
        format!("{}{}{}", &content[..nl], &line, &content[nl..])
    } else {
        format!("{}{}", content, &line)
    };

    if let Err(e) = std::fs::write(mod_file, new_content) {
        println!("⚠️  Gagal mendaftarkan modul di {}: {}", mod_file, e);
    } else {
        println!("   📝 Daftar di: {}", mod_file);
    }
}

fn make_controller(name: &str) {
    let snake = to_snake_case(name);
    let file_name = if snake.ends_with("_controller") {
        snake.clone()
    } else {
        format!("{}_controller", snake)
    };
    let struct_name = name.to_string();
    let path = format!("src/app/http/controllers/{}.rs", file_name);

    let content = format!(r#"use rustbasic_core::requests::Request;
use rustbasic_core::IntoResponse;

pub async fn index(_req: Request) -> impl IntoResponse {{
    "Hello from {struct_name}Controller!"
}}

pub async fn show(_req: Request) -> impl IntoResponse {{
    "Show action"
}}

pub async fn store(_req: Request) -> impl IntoResponse {{
    "Store action"
}}

pub async fn update(_req: Request) -> impl IntoResponse {{
    "Update action"
}}

pub async fn destroy(_req: Request) -> impl IntoResponse {{
    "Destroy action"
}}
"#);

    write_file(&path, &content);
    register_mod("src/app/http/controllers/mod.rs", &file_name);
}

fn make_model(name: &str, with_migration: bool) {
    let snake = to_snake_case(name);
    let path = format!("src/app/models/{}.rs", snake);

    let content = format!(r#"use rustbasic_core::model;

model! {{
    table: "{snake}s",
    fillable: [],
    Model {{
        pub id: i32,
        pub created_at: Option<String>,
        pub updated_at: Option<String>,
    }}
}}

impl Model {{
    pub fn to_resource(&self) -> rustbasic_core::serde_json::Value {{
        rustbasic_core::serde_json::json!({{
            "id": self.id,
        }})
    }}
}}
"#);

    write_file(&path, &content);
    register_mod("src/app/models/mod.rs", &snake);

    if with_migration {
        make_migration(&format!("create_{}_table", snake));
    }
}

fn make_middleware(name: &str) {
    let snake = to_snake_case(name);
    let path = format!("src/app/http/middleware/{}.rs", snake);

    let content = format!(r#"use rustbasic_core::requests::Request;
use rustbasic_core::router::{{Response, Next}};

pub async fn {snake}_middleware(req: Request, next: Next) -> Response {{
    // Logika middleware sebelum handler
    let res = next.run(req).await;
    // Logika middleware setelah handler
    res
}}
"#);

    write_file(&path, &content);
    register_mod("src/app/http/middleware/mod.rs", &snake);
}

fn make_observer(name: &str, model: &str) {
    let snake = to_snake_case(name);
    let obs_name = if snake.ends_with("_observer") {
        snake.clone()
    } else {
        format!("{}_observer", snake)
    };
    let path = format!("src/app/observers/{}.rs", obs_name);

    let content = format!(r#"use crate::app::models::{snake}::Model as {model};

pub struct {name}Observer;

impl {name}Observer {{
    pub async fn created(&self, model: &{model}) {{
        // Dipanggil setelah model dibuat
        let _ = model;
    }}

    pub async fn updated(&self, model: &{model}) {{
        // Dipanggil setelah model diupdate
        let _ = model;
    }}

    pub async fn deleted(&self, model: &{model}) {{
        // Dipanggil setelah model dihapus
        let _ = model;
    }}
}}
"#);

    write_file(&path, &content);
}

fn make_service(name: &str) {
    let snake = to_snake_case(name);
    let svc_name = if snake.ends_with("_service") {
        snake.clone()
    } else {
        format!("{}_service", snake)
    };
    let path = format!("src/app/services/{}.rs", svc_name);

    let content = format!(r#"use rustbasic_core::sql::AnyPool;

pub struct {name}Service<'a> {{
    pub db: &'a AnyPool,
}}

impl<'a> {name}Service<'a> {{
    pub fn new(db: &'a AnyPool) -> Self {{
        Self {{ db }}
    }}

    pub async fn execute(&self) -> Result<(), String> {{
        // Implementasi logika bisnis di sini
        Ok(())
    }}
}}
"#);

    write_file(&path, &content);
}

fn make_seeder(name: &str) {
    let snake = to_snake_case(name);
    let seed_name = if snake.ends_with("_seeder") {
        snake.clone()
    } else {
        format!("{}_seeder", snake)
    };
    let path = format!("database/seeders/{}.rs", seed_name);

    let content = format!(r#"use rustbasic_core::sql::AnyPool;

pub async fn run(db: &AnyPool) {{
    println!("🌱 Menjalankan {}...");
    // Contoh:
    // rustbasic_core::database::execute(db, "INSERT INTO ...", ()).await;
}}
"#, seed_name);

    write_file(&path, &content);
}

fn make_migration(name: &str) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Format timestamp: m{ts}_<name>
    let migration_name = format!("m{}_{}", ts, to_snake_case(name));
    let path = format!("database/migrations/{}.rs", migration_name);

    let content = format!(r#"use rustbasic_core::schema::{{Schema, MigrationTrait, DbErr}};
use rustbasic_core::sql::AnyPool;
use rustbasic_core::async_trait;

pub struct {migration_name};

#[async_trait]
impl MigrationTrait for {migration_name} {{
    async fn up(&self, db: &AnyPool) -> Result<(), DbErr> {{
        Schema::create("{}", db, |t| {{
            t.id();
            t.timestamps();
        }}).await
    }}

    async fn down(&self, db: &AnyPool) -> Result<(), DbErr> {{
        Schema::drop_if_exists("{}", db).await
    }}
}}
"#, to_snake_case(name), to_snake_case(name));

    write_file(&path, &content);
}

// ============================================================
// STORAGE:LINK
// ============================================================
fn handle_storage_link(cfg: &Config) {
    let target = "public/storage";
    let source = "storage/app/public";

    if let Err(e) = std::fs::create_dir_all(source) {
        println!("❌ Gagal membuat direktori storage: {}", e);
        return;
    }

    let path = std::path::Path::new(target);
    if path.exists() || path.is_symlink() {
        println!("ℹ️  Link 'public/storage' sudah ada.");
        return;
    }

    println!("🔗 Membuat symbolic link...");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        if let Err(e) = symlink("../storage/app/public", target) {
            println!("❌ Gagal membuat symlink: {}", e);
        } else {
            println!("✅ Link storage berhasil! [public/storage -> storage/app/public]");
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_dir;
        if let Err(e) = symlink_dir("../storage/app/public", target) {
            println!("❌ Gagal membuat symlink: {}", e);
        } else {
            println!("✅ Link storage berhasil! [public/storage -> storage/app/public]");
        }
    }

    let _ = cfg;
}

// ============================================================
// RUN NATIVE (server --android/--desktop)
// ============================================================
fn setup_java_home() {
    if std::env::var("JAVA_HOME").is_err() {
        let mut custom_java_home: Option<String> = None;
        let os = std::env::consts::OS;
        if os == "macos" {
            let paths = vec![
                "/Applications/Android Studio.app/Contents/jbr/Contents/Home",
                "/Applications/Android Studio.app/Contents/jre/Contents/Home",
                "/Library/Java/JavaVirtualMachines/zulu-17.jdk/Contents/Home",
            ];
            for path in &paths {
                if std::path::Path::new(path).exists() {
                    custom_java_home = Some(path.to_string());
                    break;
                }
            }
        } else if os == "windows" {
            let win_paths = [
                "C:\\Program Files\\Android\\Android Studio\\jbr",
                "C:\\Program Files\\Android\\Android Studio\\jre",
            ];
            for path in &win_paths {
                if std::path::Path::new(path).exists() {
                    custom_java_home = Some(path.to_string());
                    break;
                }
            }
        } else {
            // Linux & other Unix-like OS
            let unix_paths = [
                "/opt/android-studio/jbr",
                "/opt/android-studio/jre",
                "/snap/android-studio/current/jbr",
                "/snap/android-studio/current/jre",
                "/usr/local/android-studio/jbr",
                "/usr/local/android-studio/jre",
                "/usr/lib/jvm/default-java",
            ];
            for path in &unix_paths {
                if std::path::Path::new(path).exists() {
                    custom_java_home = Some(path.to_string());
                    break;
                }
            }
        }
        if let Some(jh) = custom_java_home {
            unsafe {
                std::env::set_var("JAVA_HOME", &jh);
            }
        }
    }
}

fn run_native(run_android: bool, run_desktop: bool) {
    if run_android {
        println!("🚀 Memulai RustBasic Android Wrapper (Native implementation)...");

        // 1. Setup environment
        let home = std::env::var("HOME").unwrap_or_default();
        let android_home = std::env::var("ANDROID_HOME")
            .unwrap_or_else(|_| format!("{}/Library/Android/sdk", home));
        unsafe { std::env::set_var("ANDROID_HOME", &android_home); }

        setup_java_home();

        let mut devices = get_adb_devices();
        if devices.is_empty() {
            println!("📱 Perangkat Android atau emulator tidak terdeteksi aktif.");
            let emulator_bin = format!("{}/emulator/emulator", android_home);
            if std::path::Path::new(&emulator_bin).exists() {
                let avd_output = std::process::Command::new(&emulator_bin).arg("-list-avds").output();
                if let Ok(avd_out) = avd_output {
                    let avds_str = String::from_utf8_lossy(&avd_out.stdout);
                    if let Some(avd_name) = avds_str.lines().next() {
                        println!("🚀 Menyalakan emulator AVD: {}...", avd_name);
                        let _ = std::process::Command::new(&emulator_bin)
                            .args(["-avd", avd_name])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn();
                        
                        println!("⏳ Menunggu emulator menyala dan terdeteksi adb...");
                        let _ = std::process::Command::new("adb").arg("wait-for-device").status();
                        println!("✅ Emulator berhasil aktif!");
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        devices = get_adb_devices();
                    }
                }
            }
        }

        let (device_id, device_name) = if devices.len() == 1 {
            let d = devices[0].clone();
            println!("📱 Menggunakan perangkat tunggal: {} ({})", d.1, d.0);
            d
        } else if devices.len() > 1 {
            println!("📱 Terdeteksi beberapa perangkat Android. Silakan pilih target:");
            for (idx, d) in devices.iter().enumerate() {
                println!("  [{}] {} ({})", idx + 1, d.1, d.0);
            }
            let choice = prompt_choice("👉 Pilih nomor perangkat: ", 1, devices.len());
            devices[choice - 1].clone()
        } else {
            println!("❌ Error: Tidak ada perangkat Android terhubung.");
            return;
        };

        // 3. build JNI
        if !compile_jni_libraries() {
            return;
        }

        // 4. local.properties
        let local_props = std::path::Path::new("native/android/local.properties");
        if !local_props.exists() {
            if let Ok(mut file) = std::fs::File::create(local_props) {
                use std::io::Write;
                let _ = writeln!(file, "sdk.dir={}", android_home);
            }
        }

        // 5. gradlew assembleDebug
        println!("🔨 Membangun debug APK menggunakan Gradle...");
        let gradlew_bin = if cfg!(target_os = "windows") { "gradlew.bat" } else { "./gradlew" };
        let mut gradle_cmd = std::process::Command::new(gradlew_bin);
        gradle_cmd.arg("assembleDebug");
        gradle_cmd.current_dir("native/android");

        if let Ok(jh) = std::env::var("JAVA_HOME") {
            gradle_cmd.env("JAVA_HOME", jh);
        }

        let gradle_status = gradle_cmd.status();
        if gradle_status.is_err() || !gradle_status.unwrap().success() {
            println!("❌ Gradle build assembleDebug gagal.");
            return;
        }

        // 6. adb install
        println!("🔨 Memasang APK ke perangkat {} ({})...", device_name, device_id);
        let install_status = std::process::Command::new("adb")
            .args(["-s", &device_id, "install", "-r", "native/android/app/build/outputs/apk/debug/app-debug.apk"])
            .status();

        if install_status.is_err() || !install_status.unwrap().success() {
            println!("❌ Gagal memasang APK ke device.");
            return;
        }

        // 7. adb reverse
        let vite_port = "5173"; // default
        let reverse_status = std::process::Command::new("adb")
            .args(["-s", &device_id, "reverse", &format!("tcp:{}", vite_port), &format!("tcp:{}", vite_port)])
            .status();
        if reverse_status.is_err() {
            println!("⚠️ Warning: Gagal melakukan adb reverse port {}", vite_port);
        }

        // 8. adb shell am start
        println!("🚀 Membuka aplikasi di perangkat {}...", device_name);
        let _ = std::process::Command::new("adb")
            .args(["-s", &device_id, "logcat", "-c"])
            .status();
        
        let _ = std::process::Command::new("adb")
            .args(["-s", &device_id, "shell", "am", "start", "-n", "com.rustbasic.mobile/com.rustbasic.mobile.MainActivity"])
            .status();

        println!("📋 Menampilkan log realtime dari perangkat {} (Tekan Ctrl+C untuk keluar)...", device_name);
        let mut logcat_cmd = std::process::Command::new("adb");
        logcat_cmd.args(["-s", &device_id, "logcat", "-s", "RustBasicServer"]);
        let mut child = logcat_cmd.spawn().expect("Gagal menjalankan adb logcat");
        let _ = child.wait();
    } else if run_desktop {
        println!("🚀 Memulai RustBasic Desktop Wrapper...");
        let mut cmd = std::process::Command::new("cargo");
        cmd.args(["run", "--bin", "rustbasic-desktop", "--features", "desktop"]);
        let status = cmd.status();
        match status {
            Ok(s) if s.success() => {}
            _ => {
                println!("❌ Gagal menjalankan Desktop Wrapper.");
            }
        }
    }
}

fn get_adb_devices() -> Vec<(String, String)> {
    let output = std::process::Command::new("adb").arg("devices").output();
    let mut devices = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if line.contains("device") && !line.contains("List of devices") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() { continue; }
                let device_id = parts[0].to_string();
                let model_out = std::process::Command::new("adb")
                    .args(["-s", &device_id, "shell", "getprop", "ro.product.model"])
                    .output();
                let model = if let Ok(m_out) = model_out {
                    String::from_utf8_lossy(&m_out.stdout).trim().to_string()
                } else {
                    "Unknown Device".to_string()
                };
                devices.push((device_id, model));
            }
        }
    }
    devices
}

fn compile_jni_libraries() -> bool {
    println!("🚀 Building Rust library for Android (Native Rust implementation)...");

    let _ = std::process::Command::new("rustup")
        .args(["target", "add", "aarch64-linux-android", "armv7-linux-androideabi", "x86_64-linux-android"])
        .status();

    let home = std::env::var("HOME").unwrap_or_default();
    let android_ndk_home = if let Ok(val) = std::env::var("ANDROID_NDK_HOME") {
        val
    } else {
        let mac_ndk = format!("{}/Library/Android/sdk/ndk", home);
        if std::path::Path::new(&mac_ndk).exists() {
            if let Ok(entries) = std::fs::read_dir(&mac_ndk) {
                let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
                paths.sort();
                if let Some(highest) = paths.last() {
                    highest.display().to_string()
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        }
    };

    if android_ndk_home.is_empty() {
        println!("❌ Error: ANDROID_NDK_HOME is not set. Please set ANDROID_NDK_HOME.");
        return false;
    }

    println!("Using NDK: {}", android_ndk_home);

    let os = std::env::consts::OS;
    let toolchain_sub = if os == "macos" { "darwin-x86_64" } else { "linux-x86_64" };
    let toolchain_bin_path = std::path::Path::new(&android_ndk_home)
        .join("toolchains/llvm/prebuilt")
        .join(toolchain_sub)
        .join("bin");

    if !toolchain_bin_path.exists() {
        println!("❌ Error: Toolchain bin path not found: {}", toolchain_bin_path.display());
        return false;
    }

    let sqlite_version = "3450100";
    let sqlite_dir = format!("target/sqlite-amalgamation-{}", sqlite_version);
    if !std::path::Path::new(&sqlite_dir).exists() {
        println!("📥 Downloading SQLite source amalgamation...");
        std::fs::create_dir_all("target").ok();
        
        let zip_path = "target/sqlite.zip";
        let sqlite_url = format!("https://www.sqlite.org/2024/sqlite-amalgamation-{}.zip", sqlite_version);
        
        let curl_status = std::process::Command::new("curl")
            .args(["-sSLo", zip_path, &sqlite_url])
            .status();
        
        if curl_status.is_err() || !curl_status.unwrap().success() {
            println!("❌ Gagal men-download SQLite source.");
            return false;
        }

        let unzip_status = std::process::Command::new("unzip")
            .args(["-q", zip_path, "-d", "target/"])
            .status();

        let _ = std::fs::remove_file(zip_path);

        if unzip_status.is_err() || !unzip_status.unwrap().success() {
            println!("❌ Gagal mengekstrak SQLite source.");
            return false;
        }
    }

    let targets = vec![
        ("aarch64-linux-android", "arm64-v8a", "aarch64-linux-android21-clang"),
        ("armv7-linux-androideabi", "armeabi-v7a", "armv7a-linux-androideabi21-clang"),
        ("x86_64-linux-android", "x86_64", "x86_64-linux-android21-clang"),
    ];

    let jnilibs_dir = "native/android/app/src/main/jniLibs";

    for (target, arch, clang_name) in targets {
        println!("🔨 Preparing SQLite static library for {}...", target);
        
        let clang_path = toolchain_bin_path.join(clang_name);
        let ar_path = toolchain_bin_path.join("llvm-ar");

        if !clang_path.exists() {
            println!("❌ Error: Compiler not found: {}", clang_path.display());
            return false;
        }

        let sqlite_out = format!("target/{}/sqlite", target);
        std::fs::create_dir_all(&sqlite_out).ok();

        let libsqlite3_a = format!("{}/libsqlite3.a", sqlite_out);
        if !std::path::Path::new(&libsqlite3_a).exists() {
            println!("   Compiling SQLite static lib for {}...", target);
            let sqlite3_o = format!("{}/sqlite3.o", sqlite_out);
            let sqlite3_c = format!("{}/sqlite3.c", sqlite_dir);
            
            let compile_status = std::process::Command::new(&clang_path)
                .args(["-O2", "-c", &sqlite3_c, "-o", &sqlite3_o])
                .status();

            if compile_status.is_err() || !compile_status.unwrap().success() {
                println!("❌ Gagal mengompilasi sqlite3.o");
                return false;
            }

            let archive_status = std::process::Command::new(&ar_path)
                .args(["rcs", &libsqlite3_a, &sqlite3_o])
                .status();

            if archive_status.is_err() || !archive_status.unwrap().success() {
                println!("❌ Gagal mengarsip libsqlite3.a");
                return false;
            }
        }

        println!("🔨 Compiling Rust library for {}...", target);
        let mut cargo_cmd = std::process::Command::new("cargo");
        cargo_cmd.args(["build", "--target", target, "--release"]);

        let clang_path_str = clang_path.display().to_string();
        let ar_path_str = ar_path.display().to_string();

        let target_upper = target.replace("-", "_").to_uppercase();
        let linker_env = format!("CARGO_TARGET_{}_LINKER", target_upper);
        let cc_env = format!("CC_{}", target.replace("-", "_"));
        let ar_env = format!("AR_{}", target.replace("-", "_"));

        cargo_cmd.env(&linker_env, &clang_path_str);
        cargo_cmd.env(&cc_env, &clang_path_str);
        cargo_cmd.env(&ar_env, &ar_path_str);

        let cargo_status = cargo_cmd.status();
        if cargo_status.is_err() || !cargo_status.unwrap().success() {
            println!("❌ Gagal mengompilasi library Rust untuk target {}", target);
            return false;
        }

        let dest_dir = format!("{}/{}", jnilibs_dir, arch);
        std::fs::create_dir_all(&dest_dir).ok();

        let src_so = format!("target/{}/release/librustbasic.so", target);
        let dest_so = format!("{}/librustbasic_mobile.so", dest_dir);

        if let Err(e) = std::fs::copy(&src_so, &dest_so) {
            println!("❌ Gagal menyalin {}: {}", src_so, e);
            return false;
        }
    }

    println!("✅ Android JNI libraries built successfully!");
    true
}

// ============================================================
// BUILD
// ============================================================
fn prompt_choice(prompt: &str, min: usize, max: usize) -> usize {
    use std::io::{self, Write};
    loop {
        print!("{}", prompt);
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            if let Ok(choice) = input.trim().parse::<usize>() {
                if choice >= min && choice <= max {
                    return choice;
                }
            }
        }
        println!("⚠️ Pilihan tidak valid, silakan coba lagi.");
    }
}

pub async fn handle_build(args: &[String]) {
    let mut build_docker  = args.iter().any(|a| a == "--docker");
    let mut build_desktop = args.iter().any(|a| a == "--desktop");
    let mut build_android = args.iter().any(|a| a == "--android");
    let release_mode  = args.iter().any(|a| a == "--release" || a == "-r");
    let mut target_type   = String::new();
    let mut docker_tag    = String::new();
    let mut docker_platform = String::new();

    for i in 0..args.len() {
        if args[i] == "--type" && i + 1 < args.len() { target_type = args[i+1].to_lowercase(); }
        if args[i] == "--tag"  && i + 1 < args.len() { docker_tag  = args[i+1].clone(); }
        if args[i] == "--platform" && i + 1 < args.len() { docker_platform = args[i+1].clone(); }
    }

    if !build_docker && !build_desktop && !build_android {
        let is_native_installed = if let Ok(content) = std::fs::read_to_string(".rustbasic_packages.json") {
            content.contains("\"rustbasic-native\"")
        } else {
            false
        };

        println!("🛠️  RustBasic Build CLI");
        println!("Pilih platform target untuk di-build:");
        println!("  [1] Docker (Container Image)");

        let max_choice = if is_native_installed {
            println!("  [2] Desktop Wrapper (Windows, macOS, Linux)");
            println!("  [3] Android Wrapper (APK, AAB)");
            3
        } else {
            1
        };

        let prompt_str = format!("👉 Pilih nomor platform (1-{}): ", max_choice);
        match prompt_choice(&prompt_str, 1, max_choice) {
            1 => build_docker  = true,
            2 => build_desktop = true,
            3 => build_android = true,
            _ => {}
        }
    }

    if build_docker {
        let mut extract = args.iter().any(|a| a == "--extract" || a == "-e");
        if docker_platform.is_empty() {
            println!("\nPilih Platform Target CPU Docker:");
            println!("  [1] Current Host Platform (Sesuai OS komputer Anda)");
            println!("  [2] Linux AMD64 / x86_64 (Standard VPS Intel/AMD - Umum/Rekomendasi)");
            println!("  [3] Linux ARM64 / aarch64 (Server berbasis ARM / AWS Graviton)");
            match prompt_choice("👉 Pilih (1-3): ", 1, 3) {
                2 => docker_platform = "linux/amd64".to_string(),
                3 => docker_platform = "linux/arm64".to_string(),
                _ => {}
            }
        }

        // Peringatan Arsitektur CPU Mismatch
        let host_arch = std::env::consts::ARCH;
        let is_mismatch = (host_arch == "aarch64" && docker_platform == "linux/amd64")
            || (host_arch == "x86_64" && docker_platform == "linux/arm64");

        if is_mismatch {
            println!("\n⚠️  {}", "PERINGATAN: Arsitektur CPU Mismatch (Sangat Lambat)".yellow().bold());
            println!("   Anda berada di host dengan CPU '{}' tetapi memilih target Docker '{}'.", host_arch, docker_platform);
            println!("   Docker akan menggunakan emulasi CPU (QEMU) yang membuat proses kompilasi");
            println!("   Rust berjalan {} (bisa memakan waktu 10-30 menit).", "10x-20x LEBIH LAMBAT".red().bold());
            println!("   ");
            println!("   💡 {} Kami telah mengaktifkan target caching untuk mempercepat build.", "TIPS:".cyan().bold());
            println!("      Build pertama tetap lambat, namun build berikutnya akan sangat cepat (1-2 menit)");
            println!("      karena target directory dan dependency cache disimpan oleh Docker.");
            println!("   ");
            let proceed = prompt_string("👉 Apakah Anda ingin melanjutkan proses build? (y/n) [default: y]: ", "y");
            if !proceed.to_lowercase().starts_with('y') {
                println!("❌ Build dibatalkan oleh pengguna.");
                return;
            }
        }

        if !extract && !args.iter().any(|a| a == "--docker") {
            let extract_choice = prompt_string("\n👉 Apakah Anda ingin mengekstrak biner Linux hasil build ke folder './build-output'? (y/n) [default: n]: ", "n");
            extract = extract_choice.to_lowercase().starts_with('y');
        }

        build_docker_image(&docker_tag, &docker_platform, extract).await;
    } else if build_desktop {
        build_desktop_binary(args, release_mode).await;
    } else if build_android {
        build_android_apk(args, &target_type, release_mode).await;
    }
}

async fn build_docker_image(custom_tag: &str, platform: &str, extract_binary: bool) {
    // Cek Docker tersedia
    if !std::process::Command::new("docker").arg("version")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false)
    {
        println!("❌ Docker tidak ditemukan. Install: https://docs.docker.com/get-docker/");
        return;
    }

    let dockerfile_path = std::path::Path::new("Dockerfile");
    if !dockerfile_path.exists() {
        println!("📝 Membuat Dockerfile...");
        let is_monorepo = std::path::Path::new("../rustbasic-core").exists() || std::path::Path::new("rustbasic-core").exists();
        let binary_name = get_cargo_package_name();

        let dockerfile_content = if is_monorepo {
            r#"# ============================================================
# RustBasic Docker Build — Standalone (Cached)
# ============================================================

# Stage 1: Builder
FROM rust:1-slim-bookworm AS builder

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y \
    pkg-config libssl-dev

# Copy rustbasic-core (dari konteks workspace root)
WORKDIR /rustbasic-core
COPY --from=core . .

# Copy proyek utama rustbasic
WORKDIR /build
COPY . .

# Build release binary using Cargo registry, git cache, and target cache
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --bin rustbasic && \
    cp target/release/rustbasic /build/rustbasic-bin

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary dari builder stage
COPY --from=builder /build/rustbasic-bin ./rustbasic

# Copy assets yang diperlukan dari builder stage (lebih aman dan bersih)
COPY --from=builder /build/src/resources/views/ src/resources/views/
COPY --from=builder /build/src/dist/ src/dist/
COPY --from=builder /build/public/ public/
COPY --from=builder /build/database/ database/
COPY --from=builder /build/.env.example .env

# Expose port aplikasi
EXPOSE 4000

CMD ["./rustbasic"]
"#.to_string()
        } else {
            format!(r#"# ============================================================
# RustBasic Docker Build — Standalone (Cached)
# ============================================================

# Stage 1: Builder
FROM rust:1-slim-bookworm AS builder

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y \
    pkg-config libssl-dev

WORKDIR /build

COPY . .

# Build release binary using Cargo registry, git cache, and target cache
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --bin {bin_name} && \
    cp target/release/{bin_name} /build/{bin_name}-bin

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary dari builder stage
COPY --from=builder /build/{bin_name}-bin ./{bin_name}

# Copy assets yang diperlukan dari builder stage
COPY --from=builder /build/src/resources/views/ src/resources/views/
COPY --from=builder /build/src/dist/ src/dist/
COPY --from=builder /build/public/ public/
COPY --from=builder /build/database/ database/
COPY --from=builder /build/.env.example .env

# Expose port aplikasi
EXPOSE 4000

CMD ["./{bin_name}"]
"#, bin_name = binary_name)
        };

        if let Err(e) = std::fs::write(dockerfile_path, dockerfile_content) {
            println!("❌ Gagal membuat Dockerfile: {}", e);
            return;
        }
        println!("✅ Dockerfile berhasil dibuat.");
    }

    let app_name = std::env::var("APP_NAME").unwrap_or_else(|_| get_cargo_package_name()).to_lowercase();
    let image_tag = if custom_tag.is_empty() { format!("{}:latest", app_name) } else { custom_tag.to_string() };

    if std::path::Path::new("package.json").exists() {
        println!("📦 Mengompilasi frontend assets (npm run build)...");
        let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };
        let success = match std::process::Command::new(npm_cmd)
            .args(&["run", "build"])
            .status()
        {
            Ok(status) => status.success(),
            Err(_) => {
                println!("❌ Gagal menjalankan npm. Pastikan Node.js & npm terinstal di sistem Anda.");
                return;
            }
        };

        if !success {
            println!("❌ Gagal mengompilasi frontend assets. Silakan periksa error di atas.");
            return;
        }
    }

    println!("\n🐳 Docker build dimulai...");
    println!("   Image: {}", image_tag);
    
    let core_context = if std::path::Path::new("../rustbasic-core").exists() {
        "core=../rustbasic-core".to_string()
    } else {
        "core=.".to_string()
    };

    let mut build_args = vec![
        "build".to_string(),
        "--build-context".to_string(),
        core_context,
    ];
    
    if !platform.is_empty() {
        build_args.push("--platform".to_string());
        build_args.push(platform.to_string());
        println!("   Platform: {}", platform);
    }
    
    build_args.push("-t".to_string());
    build_args.push(image_tag.clone());
    build_args.push(".".to_string());

    println!("   Running: docker {}", build_args.join(" "));

    let mut cmd = std::process::Command::new("docker")
        .args(&build_args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn().expect("Gagal menjalankan docker build");

    let success = cmd.wait().map(|s| s.success()).unwrap_or(false);

    if success {
        println!("\n✅ Docker build selesai! Image: {}", image_tag);

        if extract_binary {
            println!("📦 Mengekstrak biner Linux dari image Docker...");
            let container_name = format!("temp-extract-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs());
            
            // docker create
            let create_res = std::process::Command::new("docker")
                .args(["create", "--name", &container_name, &image_tag])
                .status();

            if create_res.is_ok() && create_res.unwrap().success() {
                // create build-output directory
                let _ = std::fs::create_dir_all("build-output");
                
                // Get cargo package name (which is the binary name inside container)
                let binary_name = get_cargo_package_name();

                // docker cp
                let cp_status = std::process::Command::new("docker")
                    .args(["cp", &format!("{}:/app/{}", container_name, binary_name), &format!("build-output/{}", binary_name)])
                    .status();
                    
                // docker rm
                let _ = std::process::Command::new("docker")
                    .args(["rm", &container_name])
                    .status();
                    
                if cp_status.is_ok() && cp_status.unwrap().success() {
                    println!("✅ Biner berhasil diekstrak ke: {}", format!("build-output/{}", binary_name).cyan().bold());
                } else {
                    println!("❌ Gagal mengekstrak biner dari container.");
                }
            } else {
                println!("❌ Gagal membuat temporary container untuk ekstraksi.");
            }
        }

        println!("   Jalankan container (Lokal/Development):");
        if !platform.is_empty() {
            println!("   docker run --platform {} -p 4000:4000 --env-file .env {}", platform, image_tag);
        } else {
            println!("   docker run -p 4000:4000 --env-file .env {}", image_tag);
        }
        println!("   Jalankan container (Produksi/Server - Auto Restart):");
        if !platform.is_empty() {
            println!("   docker run --platform {} -d -p 80:4000 --restart unless-stopped --env-file .env {}", platform, image_tag);
        } else {
            println!("   docker run -d -p 80:4000 --restart unless-stopped --env-file .env {}", image_tag);
        }
    } else {
        println!("\n❌ Docker build gagal.");
    }
}

async fn build_desktop_binary(args: &[String], mut release_mode: bool) {
    let mut target_triple = "";

    for i in 0..args.len() {
        if args[i] == "--os" && i + 1 < args.len() {
            target_triple = match args[i+1].as_str() {
                "macos-intel"   => "x86_64-apple-darwin",
                "macos-silicon" => "aarch64-apple-darwin",
                "windows"       => "x86_64-pc-windows-msvc",
                "windows-gnu"   => "x86_64-pc-windows-gnu",
                "linux"         => "x86_64-unknown-linux-gnu",
                _               => "",
            };
        }
    }

    if target_triple.is_empty() && !args.iter().any(|a| a.starts_with("--os")) {
        println!("\nPilih OS Target Desktop:");
        println!("  [1] Current OS");
        println!("  [2] macOS Intel (x86_64-apple-darwin)");
        println!("  [3] macOS Apple Silicon (aarch64-apple-darwin)");
        println!("  [4] Windows MSVC (x86_64-pc-windows-msvc)");
        println!("  [5] Windows GNU (x86_64-pc-windows-gnu - Rekomendasi Cross-compile dari macOS/Linux)");
        println!("  [6] Linux (x86_64-unknown-linux-gnu)");
        match prompt_choice("👉 Pilih (1-6): ", 1, 6) {
            2 => target_triple = "x86_64-apple-darwin",
            3 => target_triple = "aarch64-apple-darwin",
            4 => target_triple = "x86_64-pc-windows-msvc",
            5 => target_triple = "x86_64-pc-windows-gnu",
            6 => target_triple = "x86_64-unknown-linux-gnu",
            _ => {}
        }
    }

    if !args.iter().any(|a| a == "--release" || a == "-r" || a == "--debug" || a == "-d") {
        println!("\n  [1] Debug\n  [2] Release");
        if prompt_choice("👉 Mode (1-2): ", 1, 2) == 2 { release_mode = true; }
    }

    let mut build_args = vec!["build", "--bin", "rustbasic-desktop", "--features", "desktop"];
    if release_mode   { build_args.push("--release"); }
    if !target_triple.is_empty() {
        build_args.push("--target");
        build_args.push(target_triple);
        let _ = std::process::Command::new("rustup").args(["target", "add", target_triple]).status();
    }

    println!("\n🖥️  Desktop build: cargo {}", build_args.join(" "));
    let mut cmd = std::process::Command::new("cargo")
        .args(&build_args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn().expect("Gagal menjalankan cargo build");

    if cmd.wait().map(|s| s.success()).unwrap_or(false) {
        println!("\n✅ Desktop build selesai!");
    } else {
        println!("\n❌ Desktop build gagal.");
    }
}

async fn build_android_apk(args: &[String], target_type: &str, mut release_mode: bool) {
    let is_aab = if target_type.is_empty() {
        println!("\n  [1] APK\n  [2] AAB (Google Play)");
        prompt_choice("👉 Format (1-2): ", 1, 2) == 2
    } else {
        target_type == "aab"
    };

    if !args.iter().any(|a| a == "--release" || a == "-r" || a == "--debug" || a == "-d") {
        println!("\n  [1] Debug\n  [2] Release");
        if prompt_choice("👉 Mode (1-2): ", 1, 2) == 2 { release_mode = true; }
    }

    // Build JNI
    println!("\n🔨 Membangun JNI library (Native implementation)...");
    if !compile_jni_libraries() {
        println!("❌ JNI build gagal.");
        return;
    }

    // Setup environment
    let home = std::env::var("HOME").unwrap_or_default();
    let android_home = std::env::var("ANDROID_HOME")
        .unwrap_or_else(|_| format!("{}/Library/Android/sdk", home));
    unsafe { std::env::set_var("ANDROID_HOME", &android_home); }

    setup_java_home();

    let gradle_task = match (is_aab, release_mode) {
        (false, false) => "assembleDebug",
        (false, true)  => "assembleRelease",
        (true,  false) => "bundleDebug",
        (true,  true)  => "bundleRelease",
    };

    println!("\n🔨 Gradle task: {}", gradle_task);
    let gradlew = if std::path::Path::new("native/android/gradlew").exists() { "./gradlew" } else { "gradle" };
    let mut cmd = std::process::Command::new(gradlew);
    cmd.arg(gradle_task)
        .current_dir("native/android");

    if let Ok(jh) = std::env::var("JAVA_HOME") {
        cmd.env("JAVA_HOME", jh);
    }

    cmd.stdin(std::process::Stdio::inherit()).stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit());
    let mut child = cmd.spawn().expect("Gagal menjalankan Gradle");

    if child.wait().map(|s| s.success()).unwrap_or(false) {
        println!("\n✅ Android build selesai!");
    } else {
        println!("\n❌ Android build gagal.");
    }
}

fn prompt_string(prompt: &str, default: &str) -> String {
    use std::io::Write;
    print!("{}", prompt);
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            default.to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        default.to_string()
    }
}

pub async fn handle_deploy() {
    println!("\n{}", "🚀 RustBasic Docker Deploy CLI".magenta().bold());
    println!("{}", "------------------------------".magenta());

    // 1. Konfigurasi Image & Pengiriman
    let image_name = prompt_string("👉 Masukkan Nama/Tag Docker Image (default: rustbasic:latest): ", "rustbasic:latest");

    // Cek apakah Docker image sudah ada
    let inspect = std::process::Command::new("docker")
        .args(["image", "inspect", &image_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match inspect {
        Ok(status) if status.success() => {}
        _ => {
            println!("{}", format!("⚠️  Peringatan: Image '{}' tidak ditemukan lokal.", image_name).yellow());
            let proceed = prompt_string("👉 Tetap lanjutkan proses ekspor? (y/n) [default: n]: ", "n");
            if proceed.to_lowercase() != "y" {
                println!("❌ Proses dihentikan.");
                return;
            }
        }
    }

    // 2. Ekspor ke tar
    println!("📦 Mengekspor Docker image '{}' ke 'rustbasic.tar'...", image_name);
    let save_status = std::process::Command::new("docker")
        .args(["save", "-o", "rustbasic.tar", &image_name])
        .status();

    match save_status {
        Ok(status) if status.success() => {
            println!("{}", "✅ Image berhasil diekspor ke rustbasic.tar.".green());
        }
        _ => {
            println!("{}", "❌ Gagal mengekspor Docker image.".red().bold());
            return;
        }
    }

    // 3. Konfigurasi Pengiriman
    println!("\n{}", "🌐 Konfigurasi Pengiriman ke Server".cyan().bold());
    println!("{}", "-----------------------------------".cyan());
    let ssh_user = prompt_string("👉 Masukkan SSH Username Server (contoh: root) [default: root]: ", "root");
    let ssh_ip = prompt_string("👉 Masukkan IP Address Server: ", "");
    if ssh_ip.is_empty() {
        println!("{}", "❌ IP Address server tidak boleh kosong.".red().bold());
        let _ = std::fs::remove_file("rustbasic.tar");
        return;
    }
    let ssh_port = prompt_string("👉 Masukkan SSH Port Server (default: 22): ", "22");
    let dest_dir = prompt_string("👉 Masukkan Folder Tujuan di Server (default: ~/app): ", "~/app");
    let server_port = prompt_string("👉 Masukkan Port Server Mapping (contoh: 80:4000) [default: 80:4000]: ", "80:4000");
    let env_file = prompt_string("👉 Masukkan File Env yang akan dikirim (default: .env): ", ".env");

    println!("\n🚀 Menyiapkan folder tujuan di server...");
    let mkdir_status = std::process::Command::new("ssh")
        .args([
            "-p", &ssh_port,
            &format!("{}@{}", ssh_user, ssh_ip),
            &format!("mkdir -p {}", dest_dir)
        ])
        .status();

    match mkdir_status {
        Ok(status) if status.success() => {}
        _ => {
            println!("{}", "❌ Gagal terhubung ke server menggunakan SSH.".red().bold());
            let _ = std::fs::remove_file("rustbasic.tar");
            return;
        }
    }

    println!("🚀 Mengirimkan berkas rustbasic.tar & {} ke server...", env_file);
    let scp_status = std::process::Command::new("scp")
        .args([
            "-P", &ssh_port,
            "rustbasic.tar", &env_file,
            &format!("{}@{}:{}", ssh_user, ssh_ip, dest_dir)
        ])
        .status();

    // Hapus file tar lokal
    let _ = std::fs::remove_file("rustbasic.tar");

    if let Ok(status) = scp_status {
        if !status.success() {
            println!("{}", "❌ Gagal mengirimkan berkas via SCP.".red().bold());
            return;
        }
    } else {
        println!("{}", "❌ Gagal menjalankan SCP.".red().bold());
        return;
    }

    println!("{}", "✅ Pengiriman berkas berhasil!".green());

    // 4. Eksekusi SSH otomatis di server jika disetujui
    let auto_run = prompt_string("\n👉 Apakah Anda ingin langsung menjalankan container di server secara otomatis? (y/n) [default: y]: ", "y");
    if auto_run.to_lowercase() == "y" || auto_run.is_empty() {
        println!("\n🚀 Memuat image di server (docker load)...");
        let load_status = std::process::Command::new("ssh")
            .args([
                "-p", &ssh_port,
                &format!("{}@{}", ssh_user, ssh_ip),
                &format!("docker load -i {}/rustbasic.tar", dest_dir)
            ])
            .status();

        match load_status {
            Ok(status) if status.success() => {
                println!("{}", "✅ Image berhasil dimuat di server.".green());
            }
            _ => {
                println!("{}", "❌ Gagal memuat image di server.".red().bold());
                return;
            }
        }

        println!("🚀 Menghentikan & menghapus container lama 'rustbasic-app' jika ada...");
        let stop_status = std::process::Command::new("ssh")
            .args([
                "-p", &ssh_port,
                &format!("{}@{}", ssh_user, ssh_ip),
                "docker stop rustbasic-app || true && docker rm rustbasic-app || true"
            ])
            .status();

        if let Err(e) = stop_status {
            println!("⚠️ Peringatan saat membersihkan container lama: {}", e);
        }

        println!("🚀 Menjalankan container baru 'rustbasic-app'...");
        let run_cmd = format!(
            "docker run -d --name rustbasic-app -p {} --restart unless-stopped --env-file {}/.env {}",
            server_port, dest_dir, image_name
        );
        let run_status = std::process::Command::new("ssh")
            .args([
                "-p", &ssh_port,
                &format!("{}@{}", ssh_user, ssh_ip),
                &run_cmd
            ])
            .status();

        match run_status {
            Ok(status) if status.success() => {
                println!("{}", "🎉 Container 'rustbasic-app' berhasil dijalankan di server!".green().bold());
            }
            _ => {
                println!("{}", "❌ Gagal menjalankan container di server.".red().bold());
                return;
            }
        }

        println!("🚀 Membersihkan file tar di server...");
        let rm_status = std::process::Command::new("ssh")
            .args([
                "-p", &ssh_port,
                &format!("{}@{}", ssh_user, ssh_ip),
                &format!("rm {}/rustbasic.tar", dest_dir)
            ])
            .status();

        if let Err(e) = rm_status {
            println!("⚠️ Peringatan saat membersihkan file tar di server: {}", e);
        }

        println!("\n{}", "🎉 Deployment selesai!".green().bold());
        println!("{}", "--------------------------------------------------------".green());
        println!("Untuk melihat log aplikasi di server, jalankan:");
        println!("ssh -p {} {}@{} \"docker logs -f rustbasic-app\"", ssh_port, ssh_user, ssh_ip);
        println!("{}", "--------------------------------------------------------".green());
    } else {
        println!("\n{}", "🖥️  Langkah Selanjutnya di Server Anda:".cyan().bold());
        println!("{}", "--------------------------------------------------------".green());
        println!("1. Hubungkan ke server via SSH:");
        println!("   ssh -p {} {}@{}", ssh_port, ssh_user, ssh_ip);
        println!("");
        println!("2. Masuk ke folder tujuan:");
        println!("   cd {}", dest_dir);
        println!("");
        println!("3. Muat (load) image Docker dari berkas tar:");
        println!("   docker load -i rustbasic.tar");
        println!("");
        println!("4. Jalankan container dengan fitur auto-restart:");
        println!("   docker run -d --name rustbasic-app -p {} --restart unless-stopped --env-file .env {}", server_port, image_name);
        println!("");
        println!("5. Hapus file tar di server untuk menghemat disk:");
        println!("   rm rustbasic.tar");
        println!("{}", "--------------------------------------------------------".green());
    }
}

// ============================================================
// PUBLISH
// ============================================================
fn handle_publish(target: &str) {
    let mut selected_target = target.to_string();
    if selected_target.is_empty() {
        println!("🛠️  RustBasic Configuration Publisher");
        println!("Pilih konfigurasi yang ingin dipublikasikan ke proyek Anda:");
        println!("  [1] CORS (Cross-Origin Resource Sharing)");
        println!("  [2] CSRF (Cross-Site Request Forgery)");
        println!("  [3] APP (Application settings & Storage path overrides)");
        let choice = prompt_choice("👉 Pilih nomor konfigurasi (1-3): ", 1, 3);
        match choice {
            1 => selected_target = "cors".to_string(),
            2 => selected_target = "csrf".to_string(),
            3 => selected_target = "app".to_string(),
            _ => return,
        }
    }

    match selected_target.as_str() {
        "cors" => {
            let path = std::path::Path::new("src/config/cors.rs");
            if path.exists() {
                println!("ℹ️  File cors.rs sudah ada di src/config/cors.rs");
                return;
            }
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let content = r#"/* ---------------------------------------------------------
 * 📑 LABEL: CORS CONFIGURATION (src/config/cors.rs)
 * Berkas konfigurasi tambahan untuk kustomisasi CORS.
 * --------------------------------------------------------- */

pub struct CorsConfig {
    pub allowed_origins: Vec<&'static str>,
    pub allowed_methods: Vec<&'static str>,
    pub allowed_headers: Vec<&'static str>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*"],
            allowed_methods: vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"],
            allowed_headers: vec!["*"],
        }
    }
}
"#;
            if std::fs::write(path, content).is_ok() {
                println!("{} {}", "✅ Berhasil mempublikasikan konfigurasi CORS ke:".green().bold(), path.display().to_string().cyan());
                println!("💡 File ini sekarang dapat diimpor untuk menyesuaikan aturan CORS lokal.");
            } else {
                println!("❌ Gagal menulis file CORS config.");
            }
        }
        "csrf" => {
            let path = std::path::Path::new("src/config/csrf.rs");
            if path.exists() {
                println!("ℹ️  File csrf.rs sudah ada di src/config/csrf.rs");
                return;
            }
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let content = r#"/* ---------------------------------------------------------
 * 📑 LABEL: CSRF CONFIGURATION (src/config/csrf.rs)
 * Berkas konfigurasi tambahan untuk perlindungan CSRF.
 * --------------------------------------------------------- */

pub struct CsrfConfig {
    pub except_paths: Vec<&'static str>,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self {
            except_paths: vec![], // Masukkan rute yang dikecualikan dari CSRF di sini
        }
    }
}
"#;
            if std::fs::write(path, content).is_ok() {
                println!("{} {}", "✅ Berhasil mempublikasikan konfigurasi CSRF ke:".green().bold(), path.display().to_string().cyan());
                println!("💡 File ini sekarang dapat diimpor untuk mengecualikan rute tertentu dari CSRF.");
            } else {
                println!("❌ Gagal menulis file CSRF config.");
            }
        }
        "app" => {
            let path = std::path::Path::new("src/config/app.rs");
            if path.exists() {
                println!("ℹ️  File app.rs sudah ada di src/config/app.rs");
                return;
            }
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let content = r#"/* ---------------------------------------------------------
 * 📑 LABEL: APP CONFIGURATION (src/config/app.rs)
 * Berkas konfigurasi tambahan untuk kustomisasi lokasi storage lokal.
 * --------------------------------------------------------- */

pub const STORAGE_TARGET: &str = "public/storage";
pub const STORAGE_SOURCE: &str = "storage/app/public";
"#;
            if std::fs::write(path, content).is_ok() {
                println!("{} {}", "✅ Berhasil mempublikasikan konfigurasi APP ke:".green().bold(), path.display().to_string().cyan());
            } else {
                println!("❌ Gagal menulis file APP config.");
            }
        }
        _ => {
            println!("❌ Target '{}' tidak dikenal untuk di-publish.", selected_target);
            println!("💡 Target yang didukung: cors, csrf, app");
        }
    }
}

fn get_cargo_package_name() -> String {
    if let Ok(content) = std::fs::read_to_string("Cargo.toml") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name =") || trimmed.starts_with("name=") {
                let parts: Vec<&str> = trimmed.split('=').collect();
                if parts.len() > 1 {
                    let name = parts[1].trim().trim_matches('"').trim_matches('\'');
                    return name.to_string();
                }
            }
        }
    }
    "rustbasic".to_string()
}

