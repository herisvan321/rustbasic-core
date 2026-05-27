# 🚀 RustBasic Core

## 📝 Kata Pengantar

Selamat datang di **RustBasic Core**. Crate ini adalah mesin inti (core engine) berkinerja tinggi yang menggerakkan seluruh ekosistem **RustBasic Framework**. Dirancang dengan arsitektur modular yang tangguh, `rustbasic-core` menyatukan kekuatan web server **RustBasic**, ORM asinkron **Sea-ORM**, mesin template **MiniJinja**, serta layanan keamanan terintegrasi seperti proteksi CSRF otomatis, sesi terenkripsi, tracing logger harian, dan pengiriman email SMTP menggunakan Lettre. Core engine ini memberikan fondasi yang sangat stabil, efisien, dan aman untuk membangun aplikasi web modern berskala besar.

---

## 🛠️ Script Contoh

### A. Penambahan Dependensi ke Proyek Rust (`Cargo.toml`)
```toml
[dependencies]
rustbasic-core = "0.1"
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
    // Catatan: session_store dan static_files disuplai sesuai konfigurasi proyek
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

---

## 🔄 Perbandingan Pemakaian (Standard Rust vs RustBasic Core)

Berikut adalah perbandingan pemakaian dan kepraktisan antara menulis aplikasi web menggunakan library standar Rust secara langsung dan menggunakan pembungkus (wrapper) terintegrasi dari RustBasic Core:

| Fitur / Karakteristik | Menulis Manual (Standard Rust / Sea-ORM) | Menggunakan RustBasic Core Wrapper |
| :--- | :--- | :--- |
| **Inisiasi Koneksi DB** | Harus menulis puluhan baris kode pool connection manual. | Cukup panggil `database::connect(&cfg).await` secara instan. |
| **Sistem Proteksi CSRF** | Harus mengonfigurasi cookie & validator header manual. | Terintegrasi langsung dan diaktifkan otomatis pada layer HTTP. |
| **Manajemen Sesi** | Harus mengintegrasikan session store & enkripsi kunci sendiri. | Menyediakan session terenkripsi kuat berbasis `APP_KEY`. |
| **Penyajian Aset Web** | Menggunakan static folder konvensional yang lambat dibaca. | Mendukung single-binary embedding (RAM memory cache). |

---

## 📊 Tabel Ringkasan Fitur Inti RustBasic Core

Berikut adalah fitur utama yang disediakan secara modular oleh library `rustbasic-core`:

| Nama Modul Inti | Tanggung Jawab Utama | Deskripsi Fitur & Fungsionalitas |
| :--- | :--- | :--- |
| **`server`** | HTTP Server Engine | Pembungkus web server RustBasic yang menangani request/response dengan efisien. |
| **`database`** | Driver & Connection Pool | Integrasi database Sea-ORM asinkron dengan fitur auto-migration tersemat. |
| **`security`** | Proteksi CSRF & Encrypted Session | Mengamankan data sesi pengguna menggunakan enkripsi Application Key (`APP_KEY`). |
| **`logger`** | Tracing & Daily Rolling Logs | Logger yang mengarsipkan riwayat log secara harian di folder `storage/logs/`. |
| **`mailer`** | Layanan SMTP Mailer | Mengirim email HTML/teks menggunakan mail transport terintegrasi (Lettre). |

---

## 🏁 Penutup

Dengan menyediakan pustaka utilitas yang terintegrasi erat, **RustBasic Core** menyederhanakan kompleksitas pengembangan aplikasi web di Rust tanpa mengorbankan performa kecepatan dan keamanan. Pengembang dapat dengan mudah membangun aplikasi berskala produksi yang kokoh dengan baris kode yang jauh lebih ringkas.
