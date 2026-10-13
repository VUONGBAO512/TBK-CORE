# TBK Core

> **AI-Powered PC Health, Diagnostics, Security & Desktop Companion**  
> Visual reference & official site: [https://kernova.click/](https://kernova.click/)

TBK Core là ứng dụng máy tính nguyên bản chạy trên nền **Tauri 2**, **Rust**, và **React / TypeScript**, được thiết kế với ngôn ngữ hình ảnh và nhận diện thương hiệu chuẩn theo website [kernova.click](https://kernova.click/).

---

## 🌟 Tính năng chính

1. **Tổng quan Sức khỏe Máy tính (PC Health Overview):**
   - Đo lường và hiển thị thông số phần cứng thực tế (CPU, RAM, Ổ đĩa, Tiến trình) qua crate `sysinfo`.
   - Card Hero chuẩn theo website với biểu đồ Sparkline mini và chip trạng thái hoạt động thực.
2. **Động cơ Chẩn đoán Xác định (Deterministic Diagnostics Engine):**
   - 8 quy tắc chẩn đoán phân loại theo mức độ nghiêm trọng (`Critical`, `High`, `Moderate`, `Low`, `Informational`).
   - Phân biệt rõ giữa dữ liệu đo đạc thực tế và phỏng đoán.
3. **Trung tâm Bảo mật (Security Watch):**
   - Giám sát tín hiệu an toàn hệ thống (Windows Defender, Firewall, phát hiện tiến trình tiêu hao tài nguyên bất thường).
4. **Cố vấn Lưu trữ (Storage Advisor):**
   - Thống kê chi tiết các phân vùng ổ đĩa và đề xuất các phương án giải phóng dung lượng an toàn.
5. **AI Companion & Trợ lý Bác sĩ:**
   - Avatar Companion hình học phản ứng động theo trạng thái tài nguyên máy.
   - Trợ lý AI trả lời câu hỏi bảo dưỡng và phân tích số liệu hệ thống trực tiếp 100% offline.
6. **Hỗ trợ song ngữ & Đổi Theme:**
   - Hỗ trợ Tiếng Việt và English.
   - Chế độ Sáng (mặc định theo giao diện website) và Chế độ Tối.

---

## 🛠️ Hướng dẫn Khởi chạy và Build trên từng hệ điều hành

### 1. Dành cho Windows:
```powershell
# Chạy ở chế độ phát triển (Dev)
cd apps/desktop
npm install
npm run tauri dev

# Đóng gói bản Release chính thức
cargo build --release --bin TBK_Core --manifest-path src-tauri/Cargo.toml
```

---

### 2. Dành cho Linux (Ubuntu / Debian / Mint / Fedora / Arch):
Tauri trên Linux cần thư viện giao diện đồ họa WebKitGTK. Nếu bạn gặp lỗi khi chạy trên Linux, hãy cài đặt các thư viện hệ thống cần thiết bằng 1 lệnh duy nhất:

#### Với Ubuntu / Debian:
```bash
# 1. Cài đặt các gói phụ thuộc WebKit & đồ họa
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

# 2. Khởi chạy ứng dụng
cd apps/desktop
npm install
npm run tauri dev
```

#### Với Fedora:
```bash
sudo dnf check-update
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel
```

#### Với Arch Linux:
```bash
sudo pacman -Syu
sudo pacman -S webkit2gtk-4.1 base-devel curl wget openssl libappindicator-gtk3 librsvg
```
