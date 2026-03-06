# Modul 3 : Order & War Engine (json-order-service)

Microservice untuk mengelola **Modul Order & War Engine** pada platform JaStip Online Nasional (JSON).

## Tanggung Jawab Modul

Modul ini bertindak sebagai orkestrator transaksi — menangani seluruh siklus hidup pesanan mulai dari checkout hingga selesai, sekaligus mengelola mekanisme war (flash sale) untuk barang limited edition.

---


## Tech Stack

- **Rust** + **Axum** — web framework
- **SQLx** — database driver + auto migration
- **PostgreSQL** (Neon DB) — penyimpanan data


---
## ERD
![img.png](img.png)

## STATE DIAGRAM
![img_1.png](img_1.png)