use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeArticle {
    pub id: String,
    pub keywords: Vec<String>,
    pub title_en: String,
    pub title_vi: String,
    pub content_en: String,
    pub content_vi: String,
}

pub struct KnowledgeBase {
    articles: Vec<KnowledgeArticle>,
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeBase {
    pub fn new() -> Self {
        let articles = vec![
            KnowledgeArticle {
                id: "cpu_troubleshooting".to_string(),
                keywords: vec![
                    "cpu".to_string(), "processor".to_string(), "quá tải".to_string(),
                    "chíp".to_string(), "nóng máy".to_string(), "100% cpu".to_string(),
                    "lag".to_string(), "giật".to_string(), "đơ".to_string(),
                ],
                title_en: "CPU Troubleshooting & High Usage Analysis".to_string(),
                title_vi: "Khắc phục CPU hoạt động tải cao & Đơ giật".to_string(),
                content_en: "When CPU usage is high:\n1. Check the Processes tab to identify which process consumes the most cycles.\n2. Heavy operations like code compilation, Windows Update, or anti-virus background scans often cause temporary spikes.\n3. If an unrecognized process stays above 80%, consider ending its task.\n4. Ensure proper cooling and clean dust from heatsinks.".to_string(),
                content_vi: "Khi CPU hoạt động ở mức tải cao:\n1. Kiểm tra tab Tiến trình (Processes) để xem ứng dụng nào đang chiếm tài nguyên nhiều nhất.\n2. Các tác vụ như biên dịch mã, cập nhật Windows hoặc quét virus chạy ngầm có thể gây tăng đột ngột trong thời gian ngắn.\n3. Nếu một ứng dụng lạ duy trì trên 80% CPU liên tục, bạn có thể cân nhắc đóng ứng dụng đó.\n4. Đảm bảo hệ thống tản nhiệt thông thoáng và quạt làm mát hoạt động tốt.".to_string(),
            },
            KnowledgeArticle {
                id: "ram_troubleshooting".to_string(),
                keywords: vec![
                    "ram".to_string(), "memory".to_string(), "bộ nhớ".to_string(),
                    "tràn ram".to_string(), "hết ram".to_string(), "out of memory".to_string(),
                ],
                title_en: "Memory & RAM Optimization".to_string(),
                title_vi: "Tối ưu và giải phóng bộ nhớ RAM".to_string(),
                content_en: "When available memory drops below safe levels:\n1. Web browsers with numerous active tabs often consume multiple gigabytes. Close unused tabs or use tab-discarding extensions.\n2. Windows uses available memory for caching, which is automatically freed when needed.\n3. If physical RAM is consistently > 90%, check if your Windows Paging File (pagefile.sys) is system-managed.\n4. Consider upgrading physical RAM if multitasking regularly exceeds current capacity.".to_string(),
                content_vi: "Khi bộ nhớ RAM khả dụng xuống thấp:\n1. Trình duyệt web mở nhiều tab thường ngốn nhiều GB bộ nhớ. Hãy đóng bớt các tab không sử dụng.\n2. Hệ điều hành Windows tự động đệm dữ liệu vào RAM và sẽ giải phóng khi có ứng dụng cần.\n3. Nếu RAM thực tế thường xuyên > 90%, kiểm tra kích thước bộ nhớ ảo (Paging File) trên ổ đĩa SSD/HDD.\n4. Cân nhắc nâng cấp thêm thanh RAM vật lý nếu nhu cầu đa nhiệm vượt quá dung lượng hiện có.".to_string(),
            },
            KnowledgeArticle {
                id: "disk_troubleshooting".to_string(),
                keywords: vec![
                    "disk".to_string(), "ổ đĩa".to_string(), "ổ cứng".to_string(),
                    "dung lượng".to_string(), "storage".to_string(), "đầy ổ".to_string(), "c:".to_string(),
                ],
                title_en: "Disk Free Space & Storage Health".to_string(),
                title_vi: "Giải phóng dung lượng & Sức khỏe ổ đĩa".to_string(),
                content_en: "Maintaining at least 15% free space on drive C: is essential for Windows performance:\n1. Run Windows Disk Cleanup (`cleanmgr.exe`) to purge Windows Update cache and temporary files.\n2. Clear temporary user files by pressing Win+R, typing `%TEMP%`, and deleting old logs.\n3. Use 'Installed Apps' in Windows Settings to remove unused games or bulky programs.\n4. Empty your Recycle Bin regularly.".to_string(),
                content_vi: "Duy trì ít nhất 15% dung lượng trống trên ổ C: là rất quan trọng:\n1. Chạy tiện ích Windows Disk Cleanup (`cleanmgr.exe`) để xóa file tạm và bản cập nhật cũ.\n2. Nhấn Win+R, gõ `%TEMP%` và dọn dẹp các tệp tạm thời không còn dùng đến.\n3. Truy cập Cài đặt Windows > Installed Apps để gỡ cài đặt các phần mềm hoặc game không dùng.\n4. Thường xuyên dọn sạch Thùng rác (Recycle Bin).".to_string(),
            },
            KnowledgeArticle {
                id: "temperature_info".to_string(),
                keywords: vec![
                    "temperature".to_string(), "nhiệt độ".to_string(), "nóng".to_string(),
                    "sensor".to_string(), "cảm biến".to_string(), "quạt".to_string(), "fan".to_string(),
                ],
                title_en: "Hardware Temperature & Thermal Monitoring".to_string(),
                title_vi: "Nhiệt độ phần cứng & Cảm biến nhiệt".to_string(),
                content_en: "Modern Windows restrictions:\n1. Reading CPU/GPU die temperatures directly requires specialized ring-0 kernel driver permissions (like WinRing0 or LibreHardwareMonitor).\n2. Unprivileged user-mode desktop applications cannot directly query motherboard ACPI thermal zones on many modern PCs.\n3. Safe operational ranges: Idle: 35-50°C; Heavy load: 65-85°C; Throttling threshold: > 95°C.".to_string(),
                content_vi: "Cơ chế theo dõi nhiệt độ trên Windows:\n1. Việc đọc trực tiếp cảm biến nhiệt của chip CPU/GPU đòi hỏi driver cấp hạt nhân (ring-0 kernel driver) có chữ ký số.\n2. Các ứng dụng chạy quyền người dùng thông thường trên Windows bị giới hạn quyền truy cập cảm biến ACPI.\n3. Mức nhiệt độ an toàn tham khảo: Khi nghỉ (35-50°C), khi tải nặng (65-85°C), nhiệt độ cảnh báo quá nhiệt (> 95°C).".to_string(),
            },
            KnowledgeArticle {
                id: "general_health".to_string(),
                keywords: vec![
                    "sức khỏe".to_string(), "health".to_string(), "bảo trì".to_string(),
                    "tối ưu".to_string(), "optimize".to_string(), "nhanh hơn".to_string(),
                    "doctor".to_string(), "bác sĩ".to_string(), "giúp".to_string(), "help".to_string(),
                    "tbk".to_string(), "core".to_string(),
                ],
                title_en: "TBK Core Routine Maintenance Guide".to_string(),
                title_vi: "Cẩm nang bảo trì định kỳ cùng TBK Core".to_string(),
                content_en: "Key guidelines for a fast and healthy PC:\n1. Keep Startup items minimal (Task Manager -> Startup tab).\n2. Restart your computer at least once a week to clear system handles and leaked memory.\n3. Keep Windows and drivers up to date.\n4. Monitor background processes using TBK Core dashboard regularly.".to_string(),
                content_vi: "Nguyên tắc giúp máy tính luôn mượt mà và bền bỉ:\n1. Giảm thiểu các ứng dụng khởi động cùng Windows (Task Manager > Startup tab).\n2. Khởi động lại máy ít nhất một lần mỗi tuần để giải phóng bộ nhớ và tiến trình tồn đọng.\n3. Cập nhật Windows và driver phần cứng định kỳ.\n4. Thường xuyên theo dõi bảng điều khiển TBK Core để phát hiện sớm các dấu hiệu quá tải.".to_string(),
            },
            KnowledgeArticle {
                id: "storage_cleanup_safety".to_string(),
                keywords: vec![
                    "clean".to_string(), "cleanup".to_string(), "temp".to_string(), "temporary".to_string(),
                    "safe".to_string(), "dry-run".to_string(), "dry run".to_string(), "storage".to_string(),
                    "dọn rác".to_string(), "file tạm".to_string(), "an toàn".to_string(), "dọn dẹp".to_string(),
                    "xóa rác".to_string(), "thư mục tạm".to_string(), "reclaim".to_string(), "xóa file".to_string(),
                    "crash dump".to_string(), "wer".to_string(), "thumbnail".to_string(),
                ],
                title_en: "Safe Storage & Temporary Cache Cleanup Guide".to_string(),
                title_vi: "Hướng dẫn Dọn dẹp Bộ nhớ tạm & An toàn Lưu trữ".to_string(),
                content_en: "Best practices for safe storage cleanup in TBK Core:\n1. Designated Safe Targets: Safe cleanup targets include User Temporary files (%TEMP%), Windows Error Reporting crash dumps (WER), application log files, and Windows thumbnail caches.\n2. Protected Boundary Guard: System-critical directories (C:\\Windows, System32, Program Files) and user personal folders (Desktop, Documents, Downloads) are strictly whitelisted/blacklisted and never deleted.\n3. Dry-Run Simulation: Always perform a dry-run first to preview exactly which files will be removed and calculate total reclaimable bytes without altering disk contents.\n4. Locked File Protection: Files currently opened or locked by active background applications are safely skipped without aborting or causing system instability.".to_string(),
                content_vi: "Nguyên tắc dọn dẹp dung lượng và bộ nhớ đệm an toàn trong TBK Core:\n1. Vùng dữ liệu an toàn: Các khu vực được phép dọn dẹp bao gồm Thư mục tạm người dùng (%TEMP%), Báo cáo sự cố Windows (WER Crash Dumps), Nhật ký ứng dụng (App Logs) và Bộ nhớ đệm hình thu nhỏ (Thumbnail Cache).\n2. Ranh giới bảo vệ nghiêm ngặt: Tuyệt đối không xóa các thư mục hệ thống (C:\\Windows, System32, Program Files) cũng như dữ liệu cá nhân (Desktop, Documents, Downloads).\n3. Chế độ Quét mô phỏng (Dry-Run): Luôn bật chế độ Dry-Run để tính toán trước dung lượng có thể giải phóng mà không làm thay đổi hay xóa bất kỳ tệp tin nào.\n4. Cơ chế bỏ qua tệp đang sử dụng: Những tệp đang bị ứng dụng khác khóa hoặc ghi dữ liệu sẽ được tự động bỏ qua an toàn, không làm gián đoạn hay treo phần mềm.".to_string(),
            },
            KnowledgeArticle {
                id: "process_management_safety".to_string(),
                keywords: vec![
                    "process".to_string(), "terminate".to_string(), "kill".to_string(), "blacklist".to_string(),
                    "protected".to_string(), "runaway".to_string(), "csrss".to_string(), "lsass".to_string(),
                    "services".to_string(), "smss".to_string(), "explorer".to_string(), "tiến trình".to_string(),
                    "tắt tiến trình".to_string(), "đóng ứng dụng".to_string(), "bảo vệ".to_string(),
                    "danh sách đen".to_string(), "an toàn tiến trình".to_string(), "task".to_string(),
                ],
                title_en: "Process Management Safety & Critical Windows Guardrails".to_string(),
                title_vi: "Quy tắc Quản lý Tiến trình An toàn & Bảo vệ Hệ thống".to_string(),
                content_en: "Guidelines for safe process termination and resource management:\n1. Hardcoded Safety Blacklist: Essential Windows OS services—such as csrss.exe, lsass.exe, services.exe, smss.exe, wininit.exe, and explorer.exe—are strictly protected. Terminating them is blocked by TBK Core to prevent system crash or Blue Screen of Death (BSOD).\n2. Process Categorization: Processes are classified into System, Background Service, Application, and Browser Engine. Inspect categories before deciding to terminate.\n3. Runaway Process Handling: Processes consuming >80% CPU or excessive RAM over sustained periods should be analyzed. Try closing the application window gracefully before forcing termination.\n4. Multi-Process Applications: Modern browsers and electron apps spawn multiple helper processes. Terminating individual tabs or renderer workers can free memory without closing the entire application.".to_string(),
                content_vi: "Hướng dẫn quản lý và kết thúc tiến trình an toàn:\n1. Danh sách bảo vệ bất khả xâm phạm (Safety Blacklist): Các tiến trình lõi của hệ điều hành Windows—như csrss.exe, lsass.exe, services.exe, smss.exe, wininit.exe, và explorer.exe—được khóa an toàn cứng. TBK Core từ chối lệnh đóng các tiến trình này để ngăn chặn lỗi màn hình xanh (BSOD) hoặc sập hệ thống.\n2. Phân loại tiến trình: Hệ thống phân chia thành Tiến trình hệ thống (System), Dịch vụ nền (Background Service), Ứng dụng người dùng (Application) và Nhân trình duyệt (Browser Engine). Hãy kiểm tra danh mục trước khi thực hiện đóng.\n3. Xử lý tiến trình chạy bất thường (Runaway): Những tiến trình ngốn >80% CPU hoặc rò rỉ RAM kéo dài cần được kiểm tra. Ưu tiên tắt qua giao diện ứng dụng trước khi buộc dừng bằng lệnh.\n4. Ứng dụng đa tiến trình: Trình duyệt web và ứng dụng nền tảng Chromium/Electron thường mở nhiều tiến trình phụ. Bạn có thể giải phóng tài nguyên bằng cách đóng từng tab thay vì tắt toàn bộ ứng dụng.".to_string(),
            },
        ];

        Self { articles }
    }

    pub fn get_by_id(&self, id: &str) -> Option<&KnowledgeArticle> {
        self.articles.iter().find(|art| art.id == id)
    }

    pub fn search(&self, query: &str) -> Vec<&KnowledgeArticle> {
        let q = query.to_lowercase();
        let words: Vec<&str> = q.split_whitespace().collect();

        self.articles
            .iter()
            .filter(|art| {
                art.keywords.iter().any(|k| q.contains(k))
                    || words.iter().any(|&w| {
                        art.title_en.to_lowercase().contains(w)
                            || art.title_vi.to_lowercase().contains(w)
                    })
            })
            .collect()
    }

    pub fn all_articles(&self) -> &[KnowledgeArticle] {
        &self.articles
    }
}
