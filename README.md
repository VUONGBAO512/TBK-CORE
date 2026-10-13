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

## 🛠️ Hướng dẫn Khởi chạy và Kiểm thử

```powershell
# Chạy toàn bộ test suite Rust
cargo test --workspace

# Biên dịch frontend
cd apps/desktop
npm run build

# Khởi chạy ứng dụng Desktop
npm run tauri dev
```
