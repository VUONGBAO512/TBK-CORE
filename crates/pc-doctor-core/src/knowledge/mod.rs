use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        ];

        Self { articles }
    }

    pub fn search(&self, query: &str) -> Vec<&KnowledgeArticle> {
        let q = query.to_lowercase();
        let words: Vec<&str> = q.split_whitespace().collect();

        self.articles
            .iter()
            .filter(|art| {
                art.keywords.iter().any(|k| q.contains(k))
                    || words.iter().any(|&w| art.title_en.to_lowercase().contains(w) || art.title_vi.to_lowercase().contains(w))
            })
            .collect()
    }

    pub fn all_articles(&self) -> &[KnowledgeArticle] {
        &self.articles
    }
}
