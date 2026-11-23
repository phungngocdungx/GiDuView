# GiDuView 🖥️

**GiDuView** là ứng dụng điều khiển máy tính từ xa (remote desktop) mã nguồn mở, hiệu năng cao dành riêng cho macOS, tương thích hoàn toàn với giao thức **UltraViewer**.

Ứng dụng được port và tối ưu từ dự án `LinuxViewer` (Rust) sang kiến trúc hiện đại chạy native trên macOS (hỗ trợ cả vi xử lý **Apple Silicon M1/M2/M3/M4** và **Intel x86_64**).

---

## 🚀 Tính năng nổi bật

1. **Giao diện người dùng hiện đại (Tauri 2.0)**:
   - Giao diện thiết kế theo phong cách macOS mượt mà với hiệu ứng kính mờ (acrylic vibrancy) và hệ phông chữ San Francisco.
   - Thanh công cụ nổi đa năng (Floating Toolbar) hỗ trợ:
     - **Connect / Disconnect**: Kết nối và ngắt kết nối an toàn.
     - **View Only / Full Control**: Chuyển đổi giữa chế độ chỉ xem và điều khiển toàn quyền.
     - **Fullscreen**: Bật/tắt chế độ toàn màn hình.
     - **Quality Selector**: Tùy chỉnh chất lượng hiển thị (4K Ultra, High 1080p, Balanced, Low Bandwidth).
     - **Special Keys**: Gửi nhanh tổ hợp phím `Ctrl + Alt + Del`, phím `Windows / Command`.

2. **Chụp màn hình hiệu năng cao (ScreenCaptureKit & CoreGraphics)**:
   - Loại bỏ hoàn toàn mã phụ thuộc X11 (`x11rb`, `xcb`).
   - Tận dụng framework native **ScreenCaptureKit** (macOS 12.3+) và **CoreGraphics** để chụp màn hình với tần số quét lên tới 60 FPS.
   - Hỗ trợ màn hình độ phân giải cao **Retina 4K/5K** và cấu hình đa màn hình (multi-monitor arrangement).
   - Tự động chuyển đổi định dạng pixel từ 32-bit BGRA sang **RGB555 bottom-up** tương thích 100% với giao thức UltraViewer.
   - Tích hợp sẵn bộ kiểm tra và xin quyền "Screen Recording" của macOS.

3. **Giả lập Input thông minh (Input Injection)**:
   - Thay thế hoàn toàn mã XTEST bằng **CoreGraphics Event Tap** (`CGEvent`).
   - Xử lý thông minh sự khác biệt bàn phím giữa **Windows** và **macOS**:
     - Tự động ánh xạ phím `Ctrl` trên Windows sang `Command (⌘)` trên Mac khi điều khiển từ xa, giúp người dùng thao tác phím tắt tự nhiên (`Ctrl+C` $\leftrightarrow$ `Cmd+C`, `Ctrl+V` $\leftrightarrow$ `Cmd+V`).
     - Ánh xạ đầy đủ bảng mã quét phần cứng (hardware scan code) và mã phím ảo Windows VK sang macOS `CGKeyCode`.
     - Hỗ trợ gõ trực tiếp ký tự Unicode (tiếng Việt có dấu, emoji, v.v.).
   - Tích hợp kiểm tra và chuyển hướng xin quyền Accessibility (Trợ năng).

4. **Giao thức mạng (Networking & Core Protocol)**:
   - Giữ nguyên 100% logic giao thức: `wire.rs`, `messages.rs`, `tls.rs`, `client.rs`.
   - Kết nối TLS mã hóa an toàn qua relay server UltraViewer hoặc kết nối mạng nội bộ P2P trực tiếp (LAN-direct).
   - Giải mã luồng đồ họa zstd có từ điển huấn luyện chuyên dụng (`8bit.dat`) và JPEG turbo.
   - Giải mã âm thanh G.722 thời gian thực.

---

## 📂 Cấu trúc mã nguồn

```
GiDuView/
├── .github/
│   └── workflows/
│       └── release.yml          # CI/CD: Tự động build ra .dmg và .app cho Apple Silicon & Intel
├── assets/
│   └── 8bit.dat                 # Từ điển giải nén zstd của UltraViewer
├── src/                         # Frontend giao diện (HTML5, CSS, TypeScript/JS)
│   ├── index.html               # Cấu trúc giao diện chính và thanh điều khiển
│   ├── style.css                # Style macOS native (dark/light, blur glassmorphism)
│   └── app.js                   # Xử lý render Canvas, bắt sự kiện chuột/phím và gọi IPC
├── src-tauri/                   # Backend Rust
│   ├── Cargo.toml               # Cấu hình dependency cho Tauri và macOS
│   ├── tauri.conf.json          # Cấu hình đóng gói .app, .dmg, bundle ID, quyền macOS
│   ├── build.rs                 # Script build của Tauri
│   ├── capabilities/
│   │   └── default.json         # Phân quyền Tauri v2
│   ├── src/
│   │   ├── main.rs              # Điểm khởi chạy ứng dụng
│   │   ├── lib.rs               # Khởi tạo Tauri App & đăng ký invoke handlers
│   │   ├── tauri_commands.rs    # Cầu nối IPC giữa giao diện Web và engine Rust
│   │   ├── wire.rs              # Xử lý gói tin mạng (UltraViewer wire protocol)
│   │   ├── messages.rs          # Định nghĩa cấu trúc thông điệp (byte-compatible)
│   │   ├── tls.rs               # TLS encryption & bắt tay relay
│   │   ├── client.rs            # Client session engine
│   │   ├── audio.rs             # Giải mã âm thanh G.722
│   │   ├── clipboard.rs         # Đồng bộ clipboard 2 chiều
│   │   ├── command.rs           # Đăng ký và kết nối máy chủ UltraViewer
│   │   ├── cursor.rs            # Đồng bộ con trỏ chuột
│   │   ├── direct.rs            # Kết nối P2P LAN direct
│   │   ├── files.rs             # Giao thức truyền tệp tin
│   │   ├── input.rs             # Mã hóa sự kiện chuột và phím Windows
│   │   ├── profile.rs           # Lưu trữ ID máy, mật khẩu, cấu hình
│   │   ├── screen/              # Bộ giải mã zstd và bộ dựng khung hình (renderer)
│   │   ├── host/                # Engine chia sẻ màn hình máy chủ
│   │   └── platform/            # Tầng trừu tượng hóa nền tảng hệ điều hành
│   │       ├── mod.rs
│   │       └── macos/
│   │           ├── mod.rs
│   │           ├── screen_macos.rs   # Chụp màn hình ScreenCaptureKit/CoreGraphics
│   │           ├── input_macos.rs    # Giả lập input chuột/phím và ánh xạ bàn phím Mac/Win
│   │           └── permissions.rs    # Kiểm tra & xin quyền Screen Recording, Accessibility
│   └── icons/                   # Bộ icon ứng dụng (.icns, .png)
├── package.json                 # Cấu hình npm và Tauri CLI
└── README.md
```

---

## 🛠️ Hướng dẫn cài đặt & Build

### Yêu cầu môi trường (Khi build trên máy macOS):
- macOS 12 Monterey trở lên.
- [Rust](https://rustup.rs/) (phiên bản 1.75 trở lên).
- [Node.js](https://nodejs.org/) (phiên bản 18 trở lên).
- Xcode Command Line Tools (`xcode-select --install`).

### Các bước build:

1. **Cài đặt dependencies**:
   ```bash
   npm install
   ```

2. **Chạy ở chế độ phát triển (Dev mode)**:
   ```bash
   npm run dev
   ```

3. **Build bản phát hành (Production Release)**:
   - Cho **Apple Silicon (M1/M2/M3/M4)**:
     ```bash
     npx @tauri-apps/cli build --target aarch64-apple-darwin
     ```
   - Cho **Intel (x86_64)**:
     ```bash
     npx @tauri-apps/cli build --target x86_64-apple-darwin
     ```
   - Cho **Universal Binary** (chạy chung cho cả 2 kiến trúc):
     ```bash
     npx @tauri-apps/cli build --target universal-apple-darwin
     ```

File cài đặt `.dmg` và `.app` sẽ nằm trong thư mục:
`src-tauri/target/<target>/release/bundle/dmg/` và `macos/`.

---

## 📦 Bản cài đặt tải về (Releases)

Bản cài đặt sẵn định dạng `.dmg` và `.app.zip` cho cả hai phiên bản **Apple Silicon** và **Intel** được tự động đóng gói và xuất bản tại mục [Releases](https://github.com/phungngocdungx/GiDuView/releases).

---

## 📜 Giấy phép

Phát hành theo giấy phép **MIT**.
Tương thích với giao thức UltraViewer phục vụ nghiên cứu và sử dụng cá nhân/doanh nghiệp.
