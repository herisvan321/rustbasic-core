# 🚀 RustBasic Core

## 📝 Kata Pengantar

Selamat datang di **RustBasic Core**. Crate ini adalah mesin inti (core engine) berkinerja tinggi yang menggerakkan seluruh ekosistem **RustBasic Framework**. Dirancang dengan arsitektur modular yang tangguh, `rustbasic-core` menyatukan kekuatan web server **Hyper**, SQL asinkron kustom, mesin template terintegrasi, serta layanan keamanan terintegrasi seperti proteksi CSRF otomatis, sesi terenkripsi, tracing logger harian, dan pengiriman email SMTP. Core engine ini memberikan fondasi yang sangat stabil, efisien, dan aman untuk membangun aplikasi web modern berskala besar.

---

## ⚡ Sistem Fitur Opsional (Compile-Time Features)

`rustbasic-core` menggunakan sistem **Cargo features** untuk menjaga jumlah dependensi seminimal mungkin. Hanya fitur yang benar-benar dibutuhkan yang akan dikompilasi.

### Tabel Fitur

| Feature | Default | Crates Tambahan | Deskripsi |
| :--- | :---: | :---: | :--- |
| `sqlite` | ✅ Ya | ~14 | Driver SQLite via `rusqlite`. Untuk database file lokal. |
| `sqlite-bundled` | ❌ Tidak | ~14 | SQLite bundled (tidak butuh `libsqlite3` di sistem). |
| `mysql` | ❌ Tidak | ~174 | Driver MySQL/MariaDB via `mysql_async` + TLS stack penuh. |
| `mail` | ❌ Tidak | beberapa | Pengiriman email SMTP via `lettre`. |
| `http-client` | ❌ Tidak | beberapa | HTTP client via `reqwest` dengan dukungan TLS. |

> **Catatan:** Secara default, hanya `sqlite` yang aktif (~63 crates total). Ini membuat waktu kompilasi jauh lebih singkat dibanding mengaktifkan semua fitur sekaligus (>297 crates).

### Cara Mengaktifkan Fitur

```toml
# Cargo.toml project Anda

# Hanya SQLite (default - paling ringan)
rustbasic-core = { version = "0.0" }

# SQLite + MySQL
rustbasic-core = { version = "0.0", features = ["mysql"] }

# SQLite + MySQL + Email
rustbasic-core = { version = "0.0", features = ["mysql", "mail"] }

# SQLite + MySQL + Email + HTTP Client
rustbasic-core = { version = "0.0", features = ["mysql", "mail", "http-client"] }

# SQLite bundled (tanpa instalasi libsqlite3 di sistem)
rustbasic-core = { version = "0.0", features = ["sqlite-bundled"] }
```

> ⚠️ Jika `DB_CONNECTION=mysql` di file `.env`, **wajib** aktifkan feature `mysql`, atau server akan panic saat startup.

---

## 🛠️ Contoh Penggunaan

### A. Penambahan Dependensi ke Proyek Rust (`Cargo.toml`)

```toml
[dependencies]
# Pilih sesuai kebutuhan database Anda:

# Untuk SQLite:
rustbasic-core = "0.0"

# Untuk MySQL:
rustbasic-core = { version = "0.0", features = ["mysql"] }
```

### B. Memuat Konfigurasi & Menjalankan Server Utama (`src/main.rs`)

```rust
use rustbasic_core::{Config, server, database, Router};

#[tokio::main]
async fn main() {
    // 1. Memuat konfigurasi environment (.env)
    let cfg = Config::load();

    // 2. Membuka koneksi database relasional asinkron
    let db = database::connect(&cfg).await;

    // 3. Mendefinisikan router web aplikasi
    let app_router = Router::new();

    // 4. Menjalankan server web RustBasic (mendengarkan port konfigurasi)
    server::start_server(cfg, session_store, static_files, db, app_router).await;
}
```

### C. Proteksi CSRF Otomatis & Sesi dalam Handler

```rust
use rustbasic_core::{Request, Response, IntoResponse, serde_json::json};

pub async fn handler_transaksi(req: Request) -> impl IntoResponse {
    // Mengambil user ID yang tersimpan dengan aman di session terenkripsi backend
    let user_id: Option<i32> = req.session.get("user_id");
    
    match user_id {
        Some(id) => format!("Memproses transaksi untuk user ID: {}", id),
        None => "Akses Ditolak: Sesi Tidak Valid".to_string()
    }
}
```

### D. Menggunakan HTTP Client (fitur opsional)

```toml
# Aktifkan dulu di Cargo.toml:
rustbasic-core = { version = "0.0", features = ["http-client"] }
```

```rust
use rustbasic_core::Http;

// Hanya tersedia jika feature "http-client" diaktifkan
let response = Http::get("https://api.example.com/data")
    .send()
    .await?;
```

### E. Mengirim Email (fitur opsional)

```toml
# Aktifkan dulu di Cargo.toml:
rustbasic-core = { version = "0.0", features = ["mail"] }
```

```rust
use rustbasic_core::MailService;

// Hanya tersedia jika feature "mail" diaktifkan
MailService::send()
    .to("user@example.com")
    .subject("Selamat Datang!")
    .body("<h1>Terima kasih telah mendaftar.</h1>")
    .send()
    .await?;
```

---

## 🔄 Perbandingan Pemakaian (Standard Rust vs RustBasic Core)

| Fitur / Karakteristik | Menulis Manual (Standard Rust) | Menggunakan RustBasic Core |
| :--- | :--- | :--- |
| **Inisiasi Koneksi DB** | Harus menulis puluhan baris kode pool connection manual. | Cukup panggil `database::connect(&cfg).await` secara instan. |
| **Sistem Proteksi CSRF** | Harus mengonfigurasi cookie & validator header manual. | Terintegrasi langsung dan diaktifkan otomatis pada layer HTTP. |
| **Manajemen Sesi** | Harus mengintegrasikan session store & enkripsi kunci sendiri. | Menyediakan session terenkripsi kuat berbasis `APP_KEY`. |
| **Penyajian Aset Web** | Menggunakan static folder konvensional yang lambat dibaca. | Mendukung single-binary embedding (RAM memory cache). |
| **Jumlah Dependensi** | Bergantung pada semua library yang dipilih secara manual. | Minimal by default (~63 crates), fitur berat bersifat opt-in. |

---

## 📊 Tabel Ringkasan Fitur Inti RustBasic Core

| Nama Modul Inti | Tanggung Jawab Utama | Deskripsi Fitur & Fungsionalitas |
| :--- | :--- | :--- |
| **`server`** | HTTP Server Engine | Pembungkus web server RustBasic yang menangani request/response dengan efisien. |
| **`database`** | Driver & Connection Pool | Koneksi database asinkron dengan dukungan SQLite (default) dan MySQL (opsional). |
| **`security`** | Proteksi CSRF & Encrypted Session | Mengamankan data sesi pengguna menggunakan enkripsi Application Key (`APP_KEY`). |
| **`logger`** | Tracing & Daily Rolling Logs | Logger yang mengarsipkan riwayat log secara harian di folder `storage/logs/`. |
| **`mailer`** | Layanan SMTP Mailer (opsional) | Mengirim email HTML/teks menggunakan mail transport terintegrasi. Aktifkan dengan feature `mail`. |
| **`http_client`** | HTTP Client (opsional) | Mengirim request HTTP ke service eksternal. Aktifkan dengan feature `http-client`. |

---

## 🏁 Penutup

Dengan menyediakan pustaka utilitas yang terintegrasi erat dan sistem **compile-time features** yang fleksibel, **RustBasic Core** menyederhanakan kompleksitas pengembangan aplikasi web di Rust tanpa mengorbankan performa kecepatan dan keamanan. Pengembang dapat dengan mudah membangun aplikasi berskala produksi yang kokoh dengan waktu kompilasi yang sangat singkat.
