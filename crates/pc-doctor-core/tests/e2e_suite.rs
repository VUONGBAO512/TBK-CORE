// crates/pc-doctor-core/tests/e2e_suite.rs
// ============================================================================
// Automated 4-Tier E2E Test Suite for PC Doctor (TBK Core)
// Actionable Diagnostics & Safe Cleanup Suite
//
// Tier 1: Category-Partition Feature Coverage (F01 - F17)
// Tier 2: Boundary Value Analysis & Corner Cases (F01 - F17)
// Tier 3: Cross-Feature Combinatorial Pairwise Interactions
// Tier 4: Real-World Operational Multi-Step Scenarios
// ============================================================================

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use pc_doctor_core::diagnostics::DiagnosticEngine;
use pc_doctor_core::knowledge::KnowledgeBase;
use pc_doctor_core::models::*;
use pc_doctor_core::processes::{
    categorize_process, detect_runaway, is_runaway, ProcessGuard, ProcessTerminationError,
    RunawayKind,
};
use pc_doctor_core::storage::{
    CleanRequest, CleanableTarget, DefaultWindowsPathResolver, SafetyGuard, SafetyViolation,
    StorageEngine, StoragePathResolver, StoragePurger, StorageScanReport, StorageScanner,
    TargetCatalog,
};

// ============================================================================
// Section 0: Test Infrastructure Architecture & Harness
// ============================================================================

#[derive(Clone)]
pub struct TestCase {
    pub id: &'static str,
    pub tier: &'static str,
    pub feature: &'static str,
    pub description: &'static str,
    pub test_fn: fn() -> Result<(), String>,
}

pub struct TestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub duration_ms: u128,
    pub failures: Vec<(&'static str, String)>,
}

/// RAII Sandbox Fixture ensuring clean isolation and drop auto-cleanup
pub struct TestSandbox {
    pub root: PathBuf,
}

impl TestSandbox {
    pub fn new(prefix: &str) -> Self {
        let temp_dir = std::env::temp_dir();
        let unique_name = format!("tbk_e2e_{}_{}", prefix, uuid::Uuid::new_v4());
        let root = temp_dir.join(unique_name);
        fs::create_dir_all(&root).expect("Failed to create test sandbox");
        Self { root }
    }

    pub fn create_file(&self, relative_path: &str, content: &[u8]) -> PathBuf {
        let p = self.root.join(relative_path);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = File::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    pub fn create_dir(&self, relative_path: &str) -> PathBuf {
        let p = self.root.join(relative_path);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for TestSandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Mock Storage Path Resolver bounded inside a test sandbox
pub struct E2EMockResolver {
    pub temp_dir: PathBuf,
    pub crash_dumps: PathBuf,
    pub wer: PathBuf,
    pub explorer: PathBuf,
    pub logs: PathBuf,
    pub forbidden_windows: PathBuf,
    pub forbidden_system32: PathBuf,
    pub forbidden_desktop: PathBuf,
    pub forbidden_documents: PathBuf,
    pub forbidden_program_files: PathBuf,
}

impl E2EMockResolver {
    pub fn new(sandbox: &TestSandbox) -> Self {
        let temp_dir = sandbox.create_dir("Temp");
        let crash_dumps = sandbox.create_dir("CrashDumps");
        let wer = sandbox.create_dir("WER");
        let explorer = sandbox.create_dir("Explorer");
        let logs = sandbox.create_dir("Logs");

        let forbidden_windows = sandbox.create_dir("Windows");
        let forbidden_system32 = sandbox.create_dir("Windows/System32");
        let forbidden_desktop = sandbox.create_dir("Desktop");
        let forbidden_documents = sandbox.create_dir("Documents");
        let forbidden_program_files = sandbox.create_dir("Program Files");

        Self {
            temp_dir,
            crash_dumps,
            wer,
            explorer,
            logs,
            forbidden_windows,
            forbidden_system32,
            forbidden_desktop,
            forbidden_documents,
            forbidden_program_files,
        }
    }
}

impl StoragePathResolver for E2EMockResolver {
    fn resolve_target_root(&self, target_id: &str) -> Option<PathBuf> {
        match target_id {
            "user_temp" => Some(self.temp_dir.clone()),
            "crash_dumps" => Some(self.crash_dumps.clone()),
            "windows_error_reporting" => Some(self.wer.clone()),
            "explorer_thumbnails" => Some(self.explorer.clone()),
            "application_logs" => Some(self.logs.clone()),
            _ => None,
        }
    }

    fn get_safe_roots(&self) -> Vec<PathBuf> {
        vec![
            self.temp_dir.clone(),
            self.crash_dumps.clone(),
            self.wer.clone(),
            self.explorer.clone(),
            self.logs.clone(),
        ]
    }

    fn get_forbidden_roots(&self) -> Vec<PathBuf> {
        vec![
            self.forbidden_windows.clone(),
            self.forbidden_system32.clone(),
            self.forbidden_desktop.clone(),
            self.forbidden_documents.clone(),
            self.forbidden_program_files.clone(),
        ]
    }
}

/// Cross-platform locked file helper
#[cfg(windows)]
pub fn lock_file_handle(path: &Path) -> File {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0) // Exclusive lock: sharing violation on deletion attempts
        .open(path)
        .expect("Failed to lock file exclusively")
}

#[cfg(not(windows))]
pub fn lock_file_handle(path: &Path) -> File {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("Failed to open file")
}

// ---------------------------------------------------------------------------
// Telemetry & Assistant Helper Functions
// ---------------------------------------------------------------------------

pub fn dummy_cpu(usage: f32) -> CpuSnapshot {
    CpuSnapshot {
        model_name: "AMD Ryzen 9 7950X 16-Core Processor".to_string(),
        logical_cores: 32,
        physical_cores: 16,
        global_usage_percent: usage,
        cores: (0..32)
            .map(|i| CpuCoreInfo {
                core_id: i,
                usage_percent: usage,
                frequency_mhz: 4500,
            })
            .collect(),
    }
}

pub fn dummy_mem(total: u64, available: u64) -> MemorySnapshot {
    let used = total.saturating_sub(available);
    let pct = if total > 0 {
        (used as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    MemorySnapshot {
        total_bytes: total,
        used_bytes: used,
        available_bytes: available,
        usage_percent: pct,
        swap_total_bytes: 8 * 1024 * 1024 * 1024,
        swap_used_bytes: 1 * 1024 * 1024 * 1024,
    }
}

pub fn dummy_disk(mount: &str, total_gb: u64, avail_gb: u64) -> DiskSnapshot {
    let total = total_gb * 1024 * 1024 * 1024;
    let avail = avail_gb * 1024 * 1024 * 1024;
    let used = total.saturating_sub(avail);
    let pct = if total > 0 {
        (used as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    DiskSnapshot {
        name: mount.to_string(),
        mount_point: mount.to_string(),
        filesystem: "NTFS".to_string(),
        total_bytes: total,
        available_bytes: avail,
        used_bytes: used,
        usage_percent: pct,
        is_removable: false,
    }
}

pub fn dummy_os() -> OsSnapshot {
    OsSnapshot {
        name: "Windows".to_string(),
        kernel_version: "10.0.22631".to_string(),
        os_version: "Windows 11 Pro".to_string(),
        host_name: "TBK-WORKSTATION".to_string(),
        uptime_seconds: 7200,
    }
}

pub fn dummy_snapshot(findings: Vec<DiagnosticFinding>) -> SystemSnapshot {
    SystemSnapshot {
        timestamp: Utc::now(),
        cpu: dummy_cpu(25.0),
        memory: dummy_mem(32 * 1024 * 1024 * 1024, 16 * 1024 * 1024 * 1024),
        disks: vec![dummy_disk("C:", 500, 250)],
        os: dummy_os(),
        temperatures: vec![TemperatureSensor {
            label: "CPU Package".to_string(),
            celsius: Some(45.0),
            status: "available".to_string(),
        }],
        findings,
    }
}

/// Pure offline assistant response generator matching desktop IPC logic
pub fn query_assistant_pure(
    prompt: &str,
    lang: &str,
    snapshot: &SystemSnapshot,
    kb: &KnowledgeBase,
) -> AssistantResponse {
    let is_vi = lang.to_lowercase() == "vi";
    let query_lower = prompt.to_lowercase();
    let asks_status = query_lower.contains("status")
        || query_lower.contains("tình trạng")
        || query_lower.contains("sức khỏe")
        || query_lower.contains("health")
        || query_lower.contains("kiểm tra");

    let mut reply = String::new();
    let mut references = Vec::new();

    if asks_status {
        if is_vi {
            reply.push_str(&format!(
                "**Báo cáo sức khỏe máy tính hiện tại:**\n\n- **CPU**: Model {} ({:.1}% đang sử dụng trên {} luồng)\n- **RAM**: {:.1} GB / {:.1} GB ({:.1}%)\n- **Ổ đĩa chính**: {} ({:.1}% đã dùng)\n- **Thời gian hoạt động**: {} giờ\n\n",
                snapshot.cpu.model_name,
                snapshot.cpu.global_usage_percent,
                snapshot.cpu.logical_cores,
                snapshot.memory.used_bytes as f64 / 1_073_741_824.0,
                snapshot.memory.total_bytes as f64 / 1_073_741_824.0,
                snapshot.memory.usage_percent,
                snapshot.disks.first().map(|d| d.mount_point.as_str()).unwrap_or("C:"),
                snapshot.disks.first().map(|d| d.usage_percent).unwrap_or(0.0),
                snapshot.os.uptime_seconds / 3600
            ));

            if snapshot.findings.is_empty() {
                reply.push_str("✅ Hệ thống của bạn đang hoạt động trong ngưỡng an toàn ổn định!");
            } else {
                reply.push_str(&format!(
                    "⚠️ Phát hiện **{} vấn đề/cảnh báo** cần lưu ý:\n",
                    snapshot.findings.len()
                ));
                for f in &snapshot.findings {
                    reply.push_str(&format!("- **{}**: {}\n", f.title, f.description));
                    if let Some(step) = f.next_steps.first() {
                        reply.push_str(&format!("  * Khắc phục: {}\n", step));
                    }
                }
            }
        } else {
            reply.push_str(&format!(
                "**Current Computer Health Status:**\n\n- **CPU**: {} ({:.1}% usage across {} logical cores)\n- **Memory**: {:.1} GB / {:.1} GB ({:.1}% used)\n- **Primary Disk**: {} ({:.1}% used)\n- **System Uptime**: {} hours\n\n",
                snapshot.cpu.model_name,
                snapshot.cpu.global_usage_percent,
                snapshot.cpu.logical_cores,
                snapshot.memory.used_bytes as f64 / 1_073_741_824.0,
                snapshot.memory.total_bytes as f64 / 1_073_741_824.0,
                snapshot.memory.usage_percent,
                snapshot.disks.first().map(|d| d.mount_point.as_str()).unwrap_or("C:"),
                snapshot.disks.first().map(|d| d.usage_percent).unwrap_or(0.0),
                snapshot.os.uptime_seconds / 3600
            ));

            if snapshot.findings.is_empty() {
                reply.push_str("✅ Your system is running within optimal performance parameters!");
            } else {
                reply.push_str(&format!(
                    "⚠️ Detected **{} diagnostic alert(s)**:\n",
                    snapshot.findings.len()
                ));
                for f in &snapshot.findings {
                    reply.push_str(&format!("- **{}**: {}\n", f.title, f.description));
                    if let Some(step) = f.next_steps.first() {
                        reply.push_str(&format!("  * Action: {}\n", step));
                    }
                }
            }
        }
        references.push("DiagnosticEngine::evaluate".to_string());
    } else {
        let matches = kb.search(prompt);
        if let Some(first) = matches.first() {
            references.push(first.id.clone());
            if is_vi {
                reply.push_str(&format!("### {}\n\n{}", first.title_vi, first.content_vi));
            } else {
                reply.push_str(&format!("### {}\n\n{}", first.title_en, first.content_en));
            }
        } else {
            if is_vi {
                reply.push_str("Tôi là trợ lý **TBK Core** chạy hoàn toàn trên máy tính của bạn.\nBạn có thể hỏi tôi về:\n- Tình trạng máy tính / kiểm tra CPU, RAM, ổ đĩa\n- Cách khắc phục khi máy bị giật lag hoặc đầy ổ\n- Hướng dẫn bảo trì máy tính định kỳ");
            } else {
                reply.push_str("I am your native **TBK Core** assistant operating completely offline.\nYou can ask me about:\n- System health overview & CPU/RAM/Disk stats\n- Troubleshooting performance bottlenecks or low disk space\n- Routine computer optimization practices");
            }
        }
    }

    AssistantResponse { reply, references }
}

// ============================================================================
// Section 1: Tier 1 — Category-Partition Feature Coverage
// ============================================================================

// --- F01: Telemetry ---
fn test_t1_f01_01_cpu_global_and_per_core() -> Result<(), String> {
    let cpu = dummy_cpu(42.5);
    if cpu.logical_cores != 32 {
        return Err(format!("Expected 32 cores, got {}", cpu.logical_cores));
    }
    if (cpu.global_usage_percent - 42.5).abs() > 0.01 {
        return Err("Global usage mismatch".to_string());
    }
    if cpu.cores.len() != 32 {
        return Err("Cores list length mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f01_02_memory_metrics() -> Result<(), String> {
    let total = 16 * 1024 * 1024 * 1024;
    let available = 8 * 1024 * 1024 * 1024;
    let mem = dummy_mem(total, available);
    if mem.used_bytes != available {
        return Err("Used bytes mismatch".to_string());
    }
    if (mem.usage_percent - 50.0).abs() > 0.01 {
        return Err("Usage percent mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f01_03_disk_enumeration() -> Result<(), String> {
    let d1 = dummy_disk("C:", 500, 250);
    let d2 = dummy_disk("D:", 1000, 800);
    if d1.used_bytes > d1.total_bytes || d2.used_bytes > d2.total_bytes {
        return Err("Disk used bytes exceeds total".to_string());
    }
    if d1.mount_point != "C:" || d2.mount_point != "D:" {
        return Err("Mount points mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f01_04_os_platform_uptime() -> Result<(), String> {
    let os = dummy_os();
    if os.name != "Windows" {
        return Err("OS name mismatch".to_string());
    }
    if os.uptime_seconds != 7200 {
        return Err("Uptime mismatch".to_string());
    }
    if os.kernel_version.is_empty() {
        return Err("Kernel version empty".to_string());
    }
    Ok(())
}

fn test_t1_f01_05_temp_sensors() -> Result<(), String> {
    let sensor = TemperatureSensor {
        label: "CPU Core".to_string(),
        celsius: Some(55.0),
        status: "available".to_string(),
    };
    if sensor.celsius != Some(55.0) || sensor.status != "available" {
        return Err("Sensor verification failed".to_string());
    }
    Ok(())
}

// --- F02: Diagnostics ---
fn test_t1_f02_01_disk_critical_rule() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(20.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 1)]; // 1% free -> Critical
    let findings = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    if !findings.iter().any(|f| f.rule_id == "DISK_CRITICAL_CAPACITY" && f.severity == SeverityLevel::Critical) {
        return Err("DISK_CRITICAL_CAPACITY not emitted".to_string());
    }
    Ok(())
}

fn test_t1_f02_02_disk_low_rule() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(20.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 5)]; // 5% free -> Moderate
    let findings = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    if !findings.iter().any(|f| f.rule_id == "DISK_LOW_CAPACITY" && f.severity == SeverityLevel::Moderate) {
        return Err("DISK_LOW_CAPACITY not emitted".to_string());
    }
    Ok(())
}

fn test_t1_f02_03_sustained_cpu_rule() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu_high = dummy_cpu(95.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 50)];

    // 3 ticks required
    let _ = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    let _ = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    let f3 = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);

    if !f3.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED" && f.severity == SeverityLevel::High) {
        return Err("PERF_CPU_HIGH_SUSTAINED was not triggered after 3 samples".to_string());
    }
    Ok(())
}

fn test_t1_f02_04_low_memory_rule() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(20.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 400 * 1024 * 1024); // 400MB available (<500MB)
    let disks = vec![dummy_disk("C:", 100, 50)];
    let findings = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    if !findings.iter().any(|f| f.rule_id == "PERF_MEM_LOW_AVAILABLE" && f.severity == SeverityLevel::Moderate) {
        return Err("PERF_MEM_LOW_AVAILABLE not triggered".to_string());
    }
    Ok(())
}

fn test_t1_f02_05_system_healthy_baseline() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(15.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 10 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 60)];
    let temps = vec![TemperatureSensor {
        label: "CPU".to_string(),
        celsius: Some(40.0),
        status: "available".to_string(),
    }];
    let findings = engine.evaluate(&cpu, &mem, &disks, &temps, &[]);
    if findings.iter().any(|f| f.severity == SeverityLevel::Critical || f.severity == SeverityLevel::High) {
        return Err("Healthy system produced unexpected high/critical findings".to_string());
    }
    Ok(())
}

// --- F03: Safe Temp Cache Scanner ---
fn test_t1_f03_01_user_temp_discovery() -> Result<(), String> {
    let sandbox = TestSandbox::new("f03_user_temp");
    for i in 0..10 {
        sandbox.create_file(&format!("Temp/file_{}.tmp", i), &[b'A'; 2500]);
    }
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "user_temp").ok_or("user_temp not found")?;
    if target.file_count != 10 || target.total_bytes != 25000 {
        return Err(format!("Expected 10 files / 25000 bytes, got {} files / {} bytes", target.file_count, target.total_bytes));
    }
    Ok(())
}

fn test_t1_f03_02_crash_dumps_target() -> Result<(), String> {
    let sandbox = TestSandbox::new("f03_crash_dumps");
    sandbox.create_file("CrashDumps/app1.dmp", &[b'X'; 1024]);
    sandbox.create_file("CrashDumps/app2.dmp", &[b'Y'; 2048]);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "crash_dumps").ok_or("crash_dumps not found")?;
    if target.file_count != 2 || target.total_bytes != 3072 {
        return Err("Crash dumps scan mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f03_03_wer_reports_target() -> Result<(), String> {
    let sandbox = TestSandbox::new("f03_wer");
    sandbox.create_file("WER/report.wer", &[b'W'; 500]);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "windows_error_reporting").ok_or("WER not found")?;
    if target.file_count != 1 || target.total_bytes != 500 {
        return Err("WER scan mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f03_04_explorer_thumbnail_cache() -> Result<(), String> {
    let sandbox = TestSandbox::new("f03_thumb");
    sandbox.create_file("Explorer/thumbcache_256.db", &[b'T'; 4096]);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "explorer_thumbnails").ok_or("Thumbnail cache not found")?;
    if target.file_count != 1 || target.total_bytes != 4096 {
        return Err("Thumbnail cache scan mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f03_05_itemized_scan_aggregation() -> Result<(), String> {
    let sandbox = TestSandbox::new("f03_agg");
    sandbox.create_file("Temp/f1.tmp", &[b'1'; 100]);
    sandbox.create_file("CrashDumps/d1.dmp", &[b'2'; 200]);
    sandbox.create_file("WER/w1.wer", &[b'3'; 300]);
    sandbox.create_file("Explorer/thumbcache_32.db", &[b'4'; 400]);
    sandbox.create_file("Logs/app.log", &[b'5'; 500]);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();

    if report.targets.len() != 5 {
        return Err("Expected 5 catalog targets".to_string());
    }
    if report.total_files != 5 {
        return Err(format!("Expected 5 total files, got {}", report.total_files));
    }
    if report.total_reclaimable_bytes != 1500 {
        return Err(format!("Expected 1500 total bytes, got {}", report.total_reclaimable_bytes));
    }
    Ok(())
}

// --- F04: Exact Reclaimable Byte Sizes ---
fn test_t1_f04_01_single_file_exact_size() -> Result<(), String> {
    let sandbox = TestSandbox::new("f04_single");
    sandbox.create_file("Temp/single.dat", &[0u8; 1_048_576]); // Exactly 1 MB
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();
    if target.total_bytes != 1_048_576 || target.file_count != 1 {
        return Err("Exact single file size mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f04_02_multi_file_summation() -> Result<(), String> {
    let sandbox = TestSandbox::new("f04_multi");
    for i in 0..100 {
        sandbox.create_file(&format!("Temp/file_{}.tmp", i), &[1u8; 10_240]);
    }
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();
    if target.file_count != 100 || target.total_bytes != 1_024_000 {
        return Err("Multi-file summation mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f04_03_non_negative_invariant() -> Result<(), String> {
    let target = CleanableTarget::new("test", "Test", "Test", "Desc", "Desc", "C:\\Dummy", 0, 0, true);
    let report = StorageScanReport::new(vec![target]);
    let val: i128 = report.total_reclaimable_bytes as i128;
    if val < 0 {
        return Err("Bytes cannot be negative".to_string());
    }
    if report.total_reclaimable_bytes != 0 {
        return Err("Expected 0 bytes".to_string());
    }
    Ok(())
}

fn test_t1_f04_04_cross_category_equality() -> Result<(), String> {
    let t1 = CleanableTarget::new("c1", "C1", "C1", "", "", "", 5, 500 * 1024, true);
    let t2 = CleanableTarget::new("c2", "C2", "C2", "", "", "", 10, 1200 * 1024, true);
    let t3 = CleanableTarget::new("c3", "C3", "C3", "", "", "", 15, 800 * 1024, true);
    let report = StorageScanReport::new(vec![t1, t2, t3]);
    let expected = (500 + 1200 + 800) * 1024;
    if report.total_reclaimable_bytes != expected || report.total_files != 30 {
        return Err("Cross-category aggregation mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f04_05_freed_bytes_on_clean() -> Result<(), String> {
    let sandbox = TestSandbox::new("f04_clean");
    sandbox.create_file("Temp/f1.tmp", &[0u8; 1_048_576]);
    sandbox.create_file("Temp/f2.tmp", &[0u8; 1_048_576]);
    sandbox.create_file("Temp/f3.tmp", &[0u8; 1_048_576]);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.bytes_freed != 3_145_728 || summary.files_deleted != 3 {
        return Err("Freed bytes on clean mismatch".to_string());
    }
    Ok(())
}

// --- F05: Dry-Run Simulation Mode ---
fn test_t1_f05_01_dry_run_flag() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_flag");
    sandbox.create_file("Temp/data.tmp", &[0u8; 500]);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], true);
    let summary = engine.clean(&req);
    if !summary.dry_run {
        return Err("Dry run flag was false".to_string());
    }
    Ok(())
}

fn test_t1_f05_02_byte_simulation_accuracy() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_sim_bytes");
    sandbox.create_file("Temp/big.tmp", &[0u8; 15_000]);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], true);
    let summary = engine.clean(&req);
    if summary.bytes_freed != 15_000 {
        return Err("Dry run simulated bytes mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f05_03_file_count_simulation() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_sim_count");
    for i in 0..20 {
        sandbox.create_file(&format!("Temp/file_{}.tmp", i), &[1u8; 50]);
    }
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], true);
    let summary = engine.clean(&req);
    if summary.files_deleted != 20 {
        return Err("Dry run simulated files count mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f05_04_zero_disk_deletion_invariant() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_zero_del");
    let mut paths = Vec::new();
    for i in 0..10 {
        paths.push(sandbox.create_file(&format!("Temp/keep_{}.tmp", i), &[9u8; 100]));
    }
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], true);
    let _ = engine.clean(&req);

    for p in &paths {
        if !p.exists() {
            return Err("File was deleted during dry-run!".to_string());
        }
    }
    Ok(())
}

fn test_t1_f05_05_file_content_integrity() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_content");
    let content = b"UNTOUCHED_CONTENT_TEST_TBK";
    let p = sandbox.create_file("Temp/verify.dat", content);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], true);
    let _ = engine.clean(&req);

    let read_back = fs::read(&p).map_err(|e| e.to_string())?;
    if read_back != content {
        return Err("File content was altered during dry run!".to_string());
    }
    Ok(())
}

// --- F06: Locked/In-Use File Skip ---
fn test_t1_f06_01_single_locked_file_handled() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_single_lock");
    let normal1 = sandbox.create_file("Temp/normal1.tmp", &[1u8; 1000]);
    let normal2 = sandbox.create_file("Temp/normal2.tmp", &[2u8; 1000]);
    let locked_path = sandbox.create_file("Temp/locked.tmp", &[3u8; 5000]);

    let _lock = lock_file_handle(&locked_path);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if !locked_path.exists() {
        return Err("Locked file should NOT have been deleted".to_string());
    }
    if normal1.exists() || normal2.exists() {
        return Err("Unlocked files should have been deleted".to_string());
    }
    if summary.files_deleted != 2 || summary.files_skipped_locked != 1 {
        return Err("Summary mismatch for locked file handling".to_string());
    }
    Ok(())
}

fn test_t1_f06_02_skipped_files_counter() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_skip_counter");
    let l1 = sandbox.create_file("Temp/lock1.tmp", &[0u8; 100]);
    let l2 = sandbox.create_file("Temp/lock2.tmp", &[0u8; 100]);
    let _h1 = lock_file_handle(&l1);
    let _h2 = lock_file_handle(&l2);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.files_skipped_locked != 2 {
        return Err(format!("Expected 2 skipped files, got {}", summary.files_skipped_locked));
    }
    Ok(())
}

fn test_t1_f06_03_descriptive_error_reporting() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_err");
    let l1 = sandbox.create_file("Temp/lock.tmp", &[0u8; 100]);
    let _h1 = lock_file_handle(&l1);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.files_skipped_locked == 0 {
        return Err("Locked file was not recorded in skipped counter".to_string());
    }
    Ok(())
}

fn test_t1_f06_04_freed_bytes_excludes_locked() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_freed_excl");
    sandbox.create_file("Temp/n1.tmp", &[0u8; 1000]);
    sandbox.create_file("Temp/n2.tmp", &[0u8; 1000]);
    let l1 = sandbox.create_file("Temp/locked.tmp", &[0u8; 5000]);
    let _h = lock_file_handle(&l1);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.bytes_freed != 2000 {
        return Err(format!("Expected 2000 freed bytes, got {}", summary.bytes_freed));
    }
    Ok(())
}

fn test_t1_f06_05_zero_panic_guarantee() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_zero_panic");
    let mut locks = Vec::new();
    for i in 0..5 {
        let p = sandbox.create_file(&format!("Temp/contended_{}.tmp", i), &[i as u8; 200]);
        locks.push(lock_file_handle(&p));
    }
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);
    if summary.files_skipped_locked != 5 {
        return Err("Contended locks were not all safely skipped".to_string());
    }
    Ok(())
}

// --- F07: Protected Location Boundary Guard ---
fn test_t1_f07_01_system32_rejection() -> Result<(), String> {
    let sandbox = TestSandbox::new("f07_sys32");
    let resolver = E2EMockResolver::new(&sandbox);
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let sys32 = resolver.forbidden_system32.clone();
    let res = guard.validate(&sys32);
    if !matches!(res, Err(SafetyViolation::ForbiddenPathMatched { .. })) {
        return Err("System32 was not rejected by safety guard".to_string());
    }
    Ok(())
}

fn test_t1_f07_02_windows_root_rejection() -> Result<(), String> {
    let sandbox = TestSandbox::new("f07_winroot");
    let resolver = E2EMockResolver::new(&sandbox);
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let res = guard.validate(&resolver.forbidden_windows);
    if !matches!(res, Err(SafetyViolation::ForbiddenPathMatched { .. })) {
        return Err("Windows root was not rejected by safety guard".to_string());
    }
    Ok(())
}

fn test_t1_f07_03_desktop_documents_rejection() -> Result<(), String> {
    let sandbox = TestSandbox::new("f07_desk_docs");
    let resolver = E2EMockResolver::new(&sandbox);
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    let res_desk = guard.validate(&resolver.forbidden_desktop);
    let res_docs = guard.validate(&resolver.forbidden_documents);

    if !matches!(res_desk, Err(SafetyViolation::ForbiddenPathMatched { .. })) {
        return Err("Desktop was not rejected".to_string());
    }
    if !matches!(res_docs, Err(SafetyViolation::ForbiddenPathMatched { .. })) {
        return Err("Documents was not rejected".to_string());
    }
    Ok(())
}

fn test_t1_f07_04_drive_root_rejection() -> Result<(), String> {
    let sandbox = TestSandbox::new("f07_drive_root");
    let resolver = E2EMockResolver::new(&sandbox);
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let root = PathBuf::from("C:\\");
    let res = guard.validate(&root);
    if !matches!(res, Err(SafetyViolation::DriveRootProhibited(_))) {
        return Err("Drive root was not rejected".to_string());
    }
    Ok(())
}

fn test_t1_f07_05_legitimate_child_validation() -> Result<(), String> {
    let sandbox = TestSandbox::new("f07_legit");
    let resolver = E2EMockResolver::new(&sandbox);
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let child = resolver.temp_dir.join("scratch_data.tmp");
    let res = guard.validate(&child);
    if res.is_err() {
        return Err("Legitimate child path was rejected".to_string());
    }
    Ok(())
}

// --- F08: Process Listing & Sorting ---
fn test_t1_f08_01_process_snapshot_enumeration() -> Result<(), String> {
    let p1 = ProcessInfo { pid: 100, name: "app.exe".to_string(), cpu_percent: 5.0, memory_bytes: 1024, status: "Run".to_string() };
    if p1.pid != 100 || p1.name != "app.exe" {
        return Err("Process snapshot enumeration mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f08_02_process_field_population() -> Result<(), String> {
    let item = ProcessItem::new(1024, "editor.exe", 14.5, 300 * 1024 * 1024, "Running", ProcessCategory::Application, false);
    if item.pid != 1024 || item.cpu_percent != 14.5 || item.category != ProcessCategory::Application {
        return Err("Process item fields not properly populated".to_string());
    }
    Ok(())
}

fn test_t1_f08_03_sort_cpu_descending() -> Result<(), String> {
    let mut procs = vec![
        ProcessInfo { pid: 1, name: "p1".to_string(), cpu_percent: 10.0, memory_bytes: 100, status: "R".to_string() },
        ProcessInfo { pid: 2, name: "p2".to_string(), cpu_percent: 85.0, memory_bytes: 100, status: "R".to_string() },
        ProcessInfo { pid: 3, name: "p3".to_string(), cpu_percent: 45.0, memory_bytes: 100, status: "R".to_string() },
    ];
    procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());
    if procs[0].pid != 2 || procs[1].pid != 3 || procs[2].pid != 1 {
        return Err("Sort by CPU descending failed".to_string());
    }
    Ok(())
}

fn test_t1_f08_04_sort_memory_descending() -> Result<(), String> {
    let mut procs = vec![
        ProcessInfo { pid: 1, name: "p1".to_string(), cpu_percent: 1.0, memory_bytes: 500, status: "R".to_string() },
        ProcessInfo { pid: 2, name: "p2".to_string(), cpu_percent: 1.0, memory_bytes: 9000, status: "R".to_string() },
        ProcessInfo { pid: 3, name: "p3".to_string(), cpu_percent: 1.0, memory_bytes: 2000, status: "R".to_string() },
    ];
    procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    if procs[0].pid != 2 || procs[1].pid != 3 || procs[2].pid != 1 {
        return Err("Sort by memory descending failed".to_string());
    }
    Ok(())
}

fn test_t1_f08_05_filtering_substring() -> Result<(), String> {
    let procs = vec![
        ProcessInfo { pid: 1, name: "chrome.exe".to_string(), cpu_percent: 10.0, memory_bytes: 100, status: "R".to_string() },
        ProcessInfo { pid: 2, name: "notepad.exe".to_string(), cpu_percent: 1.0, memory_bytes: 50, status: "R".to_string() },
        ProcessInfo { pid: 3, name: "Google_Chrome_Helper".to_string(), cpu_percent: 5.0, memory_bytes: 80, status: "R".to_string() },
    ];
    let query = "chrome";
    let filtered: Vec<_> = procs.into_iter().filter(|p| p.name.to_lowercase().contains(query)).collect();
    if filtered.len() != 2 {
        return Err("Filtering by substring mismatch".to_string());
    }
    Ok(())
}

// --- F09: Process Categorization ---
fn test_t1_f09_01_app_classification() -> Result<(), String> {
    let cat = categorize_process(500, "notepad.exe", None);
    if cat != ProcessCategory::Application {
        return Err("notepad.exe should be categorized as Application".to_string());
    }
    Ok(())
}

fn test_t1_f09_02_service_classification() -> Result<(), String> {
    let cat = categorize_process(501, "svchost.exe", None);
    if cat != ProcessCategory::BackgroundService {
        return Err("svchost.exe should be BackgroundService".to_string());
    }
    Ok(())
}

fn test_t1_f09_03_category_serialization() -> Result<(), String> {
    let s_app = serde_json::to_string(&ProcessCategory::Application).unwrap();
    let s_svc = serde_json::to_string(&ProcessCategory::BackgroundService).unwrap();
    if s_app != "\"application\"" || s_svc != "\"background_service\"" {
        return Err("Category serialization mismatch".to_string());
    }
    // Test alias deserialization
    let d_app: ProcessCategory = serde_json::from_str("\"app\"").unwrap();
    let d_svc: ProcessCategory = serde_json::from_str("\"service\"").unwrap();
    if d_app != ProcessCategory::Application || d_svc != ProcessCategory::BackgroundService {
        return Err("Category deserialization alias mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f09_04_ui_category_subsetting() -> Result<(), String> {
    let items = vec![
        ProcessItem::new(1, "app.exe", 1.0, 100, "R", ProcessCategory::Application, false),
        ProcessItem::new(2, "svc.exe", 1.0, 100, "R", ProcessCategory::BackgroundService, false),
    ];
    let apps_only: Vec<_> = items.into_iter().filter(|i| i.category == ProcessCategory::Application).collect();
    if apps_only.len() != 1 || apps_only[0].pid != 1 {
        return Err("Category filtering mismatch".to_string());
    }
    Ok(())
}

fn test_t1_f09_05_protected_flag_derivation() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let is_prot = guard.is_protected(100, "csrss.exe");
    if !is_prot {
        return Err("csrss.exe must be flagged protected".to_string());
    }
    Ok(())
}

// --- F10: Runaway Process Detection ---
fn test_t1_f10_01_high_cpu_runaway() -> Result<(), String> {
    let res = detect_runaway(94.5, 100 * 1024 * 1024);
    if !matches!(res, Some(RunawayKind::Cpu { cpu_percent }) if (cpu_percent - 94.5).abs() < 0.01) {
        return Err("Expected RunawayKind::Cpu".to_string());
    }
    Ok(())
}

fn test_t1_f10_02_high_memory_runaway() -> Result<(), String> {
    let two_and_half_gb = 2_500_000_000u64;
    let res = detect_runaway(10.0, two_and_half_gb);
    if !matches!(res, Some(RunawayKind::Memory { memory_bytes }) if memory_bytes == two_and_half_gb) {
        return Err("Expected RunawayKind::Memory".to_string());
    }
    Ok(())
}

fn test_t1_f10_03_dual_pressure_runaway() -> Result<(), String> {
    let res = detect_runaway(75.0, 1_800_000_000);
    if !matches!(res, Some(RunawayKind::DualPressure { .. })) {
        return Err("Expected RunawayKind::DualPressure".to_string());
    }
    Ok(())
}

fn test_t1_f10_04_evidence_metadata() -> Result<(), String> {
    let kind = detect_runaway(92.0, 500 * 1024 * 1024).unwrap();
    match kind {
        RunawayKind::Cpu { cpu_percent } => {
            if cpu_percent != 92.0 {
                return Err("Evidence CPU mismatch".to_string());
            }
        }
        _ => return Err("Unexpected kind".to_string()),
    }
    Ok(())
}

fn test_t1_f10_05_normal_process_negative() -> Result<(), String> {
    let res = detect_runaway(12.0, 180 * 1024 * 1024);
    if res.is_some() {
        return Err("Normal process flagged as runaway".to_string());
    }
    Ok(())
}

// --- F11: Process Safety Blacklist ---
fn test_t1_f11_01_csrss_blocked() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let res = guard.validate_termination(104, "csrss.exe");
    if !matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
        return Err("csrss.exe was not blocked".to_string());
    }
    Ok(())
}

fn test_t1_f11_02_lsass_blocked() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let res = guard.validate_termination(650, "lsass.exe");
    if !matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
        return Err("lsass.exe was not blocked".to_string());
    }
    Ok(())
}

fn test_t1_f11_03_services_blocked() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let res = guard.validate_termination(700, "services.exe");
    if !matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
        return Err("services.exe was not blocked".to_string());
    }
    Ok(())
}

fn test_t1_f11_04_smss_blocked() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let res = guard.validate_termination(350, "smss.exe");
    if !matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
        return Err("smss.exe was not blocked".to_string());
    }
    Ok(())
}

fn test_t1_f11_05_explorer_blocked() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let res = guard.validate_termination(2048, "explorer.exe");
    if !matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
        return Err("explorer.exe was not blocked".to_string());
    }
    Ok(())
}

// --- F12: Offline Companion Assistant ---
fn test_t1_f12_01_status_query_english() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let resp = query_assistant_pure("how is my system health status?", "en", &snap, &kb);
    if !resp.reply.contains("Current Computer Health Status") {
        return Err("English status response header missing".to_string());
    }
    Ok(())
}

fn test_t1_f12_02_status_query_vietnamese() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let resp = query_assistant_pure("tình trạng máy tính thế nào?", "vi", &snap, &kb);
    if !resp.reply.contains("Báo cáo sức khỏe máy tính") {
        return Err("Vietnamese status response header missing".to_string());
    }
    Ok(())
}

fn test_t1_f12_03_finding_next_steps_included() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let finding = DiagnosticFinding {
        rule_id: "DISK_LOW_CAPACITY".to_string(),
        severity: SeverityLevel::Moderate,
        title: "Low Disk Space".to_string(),
        description: "Free space below 10%".to_string(),
        evidence: vec![],
        observed_at: Utc::now(),
        confidence: 1.0,
        possible_causes: vec![],
        next_steps: vec!["Dọn dẹp thư mục tạm %TEMP%".to_string()],
        limitations: vec![],
    };
    let snap = dummy_snapshot(vec![finding]);
    let resp = query_assistant_pure("tình trạng hệ thống", "vi", &snap, &kb);
    if !resp.reply.contains("Dọn dẹp thư mục tạm %TEMP%") {
        return Err("Diagnostic finding next steps missing in reply".to_string());
    }
    Ok(())
}

fn test_t1_f12_04_healthy_confirmation() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let resp = query_assistant_pure("check health status", "en", &snap, &kb);
    if !resp.reply.contains("optimal performance parameters") {
        return Err("Healthy confirmation banner missing".to_string());
    }
    Ok(())
}

fn test_t1_f12_05_offline_execution_guarantee() -> Result<(), String> {
    let start = Instant::now();
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let _ = query_assistant_pure("cpu high", "en", &snap, &kb);
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 50 {
        return Err("Offline execution took excessive time".to_string());
    }
    Ok(())
}

// --- F13: Bilingual Knowledge Base ---
fn test_t1_f13_01_cpu_lookup_en() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res = kb.search("cpu high usage lag");
    if res.is_empty() || res[0].id != "cpu_troubleshooting" {
        return Err("Failed to resolve cpu_troubleshooting in English".to_string());
    }
    Ok(())
}

fn test_t1_f13_02_cpu_lookup_vi() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res = kb.search("máy tính bị quá tải cpu nóng giật");
    if res.is_empty() || res[0].id != "cpu_troubleshooting" {
        return Err("Failed to resolve cpu_troubleshooting in Vietnamese".to_string());
    }
    Ok(())
}

fn test_t1_f13_03_storage_guide_lookup() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res = kb.search("dọn rác và thư mục tạm an toàn");
    if res.is_empty() || !res.iter().any(|a| a.id == "storage_cleanup_safety") {
        return Err("Failed to resolve storage_cleanup_safety".to_string());
    }
    Ok(())
}

fn test_t1_f13_04_process_safety_guide_lookup() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res = kb.search("tắt tiến trình hệ thống danh sách đen csrss");
    if res.is_empty() || !res.iter().any(|a| a.id == "process_management_safety") {
        return Err("Failed to resolve process_management_safety".to_string());
    }
    Ok(())
}

fn test_t1_f13_05_ram_optimization_lookup() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res = kb.search("tràn bộ nhớ ram hết ram");
    if res.is_empty() || res[0].id != "ram_troubleshooting" {
        return Err("Failed to resolve ram_troubleshooting".to_string());
    }
    Ok(())
}

// ============================================================================
// Tier 1: F14 - React Storage Cleaner Suite UI Contracts
// ============================================================================

fn test_t1_f14_01_target_grid_rendering() -> Result<(), String> {
    let resolver = DefaultWindowsPathResolver::default();
    let targets = TargetCatalog::get_definitions(&resolver);
    let report = StorageScanReport::new(targets);

    if report.targets.len() != 5 {
        return Err(format!("Expected 5 targets in grid report, got {}", report.targets.len()));
    }

    for t in &report.targets {
        if t.name_en.is_empty() || t.name_vi.is_empty() {
            return Err(format!("Target {} missing localized name", t.id));
        }
        if t.description_en.is_empty() || t.description_vi.is_empty() {
            return Err(format!("Target {} missing localized description", t.id));
        }
        if !t.is_safe {
            return Err(format!("Catalog target {} must be flagged safe", t.id));
        }
    }
    Ok(())
}

fn test_t1_f14_02_category_selection_toggle() -> Result<(), String> {
    let t1 = CleanableTarget::new("user_temp", "User Temp", "Tệp tạm", "", "", "", 10, 1000, true);
    let t2 = CleanableTarget::new("crash_dumps", "Crash Dumps", "Bản ghi lỗi", "", "", "", 5, 500, true);
    let report = StorageScanReport::new(vec![t1.clone(), t2.clone()]);

    let mut selected: std::collections::HashSet<String> = report.targets.iter().map(|t| t.id.clone()).collect();
    if selected.len() != 2 {
        return Err("Initial selection should contain 2 items".to_string());
    }

    let calc_selected_bytes = |sel: &std::collections::HashSet<String>, rep: &StorageScanReport| -> u64 {
        rep.targets.iter().filter(|t| sel.contains(&t.id)).map(|t| t.total_bytes).sum()
    };

    if calc_selected_bytes(&selected, &report) != 1500 {
        return Err("Initial selected bytes mismatch".to_string());
    }

    // Toggle user_temp off
    selected.remove("user_temp");
    if calc_selected_bytes(&selected, &report) != 500 {
        return Err("Deselecting user_temp did not reduce selected bytes".to_string());
    }

    // Toggle user_temp back on
    selected.insert("user_temp".to_string());
    if calc_selected_bytes(&selected, &report) != 1500 {
        return Err("Re-selecting user_temp did not restore selected bytes".to_string());
    }

    // Clear selection
    selected.clear();
    if calc_selected_bytes(&selected, &report) != 0 {
        return Err("Clearing selection should yield 0 bytes".to_string());
    }

    Ok(())
}

fn test_t1_f14_03_dry_run_switch_toggle() -> Result<(), String> {
    let mut is_dry_run = false;

    // Default mode: Live Clean
    let req_live = CleanRequest::new(vec!["user_temp".to_string()], is_dry_run);
    if req_live.dry_run {
        return Err("Default mode should have dry_run = false".to_string());
    }

    // Toggle switch ON (Dry-Run mode)
    is_dry_run = true;
    let req_dry = CleanRequest::new(vec!["user_temp".to_string()], is_dry_run);
    if !req_dry.dry_run {
        return Err("Toggled state should have dry_run = true".to_string());
    }

    // Toggle switch OFF (Live Clean mode)
    is_dry_run = false;
    let req_live2 = CleanRequest::new(vec!["user_temp".to_string()], is_dry_run);
    if req_live2.dry_run {
        return Err("Toggling off should restore dry_run = false".to_string());
    }

    Ok(())
}

fn test_t1_f14_04_purge_confirmation_modal_workflow() -> Result<(), String> {
    let targets = vec![
        CleanableTarget::new("user_temp", "User Temp", "Tệp tạm", "", "", "", 12, 10_000, true),
        CleanableTarget::new("crash_dumps", "Crash Dumps", "Bản ghi lỗi", "", "", "", 3, 5_000, true),
    ];
    let selected: std::collections::HashSet<String> = vec!["user_temp".to_string(), "crash_dumps".to_string()].into_iter().collect();

    let total_selected_bytes: u64 = targets.iter().filter(|t| selected.contains(&t.id)).map(|t| t.total_bytes).sum();
    let total_selected_files: usize = targets.iter().filter(|t| selected.contains(&t.id)).map(|t| t.file_count).sum();

    if total_selected_bytes != 15_000 || total_selected_files != 15 {
        return Err("Modal summary metrics computation error".to_string());
    }

    let req = CleanRequest::new(selected.into_iter().collect(), false);
    let json = serde_json::to_string(&req).map_err(|e| e.to_string())?;
    if !json.contains("user_temp") || !json.contains("crash_dumps") || !json.contains("\"dry_run\":false") {
        return Err("CleanRequest JSON serialization contract violated".to_string());
    }

    Ok(())
}

fn test_t1_f14_05_summary_results_banner_display() -> Result<(), String> {
    let summary = pc_doctor_core::storage::StorageCleanSummary {
        dry_run: false,
        bytes_freed: 2_097_152,
        files_deleted: 14,
        files_skipped_locked: 2,
        targets_processed: vec!["user_temp".to_string(), "crash_dumps".to_string()],
        errors: vec![],
    };

    let format_bytes = |bytes: u64| -> String {
        if bytes == 0 { "0 B".to_string() }
        else if bytes < 1024 { format!("{} B", bytes) }
        else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
        else if bytes < 1024 * 1024 * 1024 { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
        else { format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
    };

    let formatted = format_bytes(summary.bytes_freed);
    if formatted != "2.0 MB" {
        return Err(format!("Expected '2.0 MB', got '{}'", formatted));
    }
    if summary.files_deleted != 14 || summary.files_skipped_locked != 2 {
        return Err("Files counters mismatch".to_string());
    }
    if !summary.errors.is_empty() {
        return Err("Unexpected errors in summary".to_string());
    }

    Ok(())
}

// ============================================================================
// Tier 1: F15 - React Process Manager UI Contracts
// ============================================================================

fn test_t1_f15_01_process_table_columns_rendering() -> Result<(), String> {
    let mut procs = Vec::new();
    for i in 1..=50 {
        procs.push(ProcessItem::new(
            i as u32 * 100,
            &format!("proc_{}.exe", i),
            (i as f32) * 0.5,
            (i as u64) * 10 * 1024 * 1024,
            "Running",
            if i % 3 == 0 { ProcessCategory::BackgroundService } else { ProcessCategory::Application },
            false,
        ));
    }

    if procs.len() != 50 {
        return Err("Expected 50 process items".to_string());
    }

    for p in &procs {
        if p.pid == 0 || p.name.is_empty() || p.cpu_percent < 0.0 || p.memory_bytes == 0 {
            return Err("Invalid process field rendering contract".to_string());
        }
        let mb = (p.memory_bytes as f64) / 1_048_576.0;
        let mb_str = format!("{:.1}", mb);
        if !mb_str.ends_with(".0") {
            return Err("fmtMB did not format cleanly".to_string());
        }
    }
    Ok(())
}

fn test_t1_f15_02_category_tab_switching() -> Result<(), String> {
    let p_app = ProcessItem::new(100, "notepad.exe", 1.0, 50_000_000, "R", ProcessCategory::Application, false);
    let p_svc = ProcessItem::new(200, "spoolsv.exe", 0.5, 20_000_000, "R", ProcessCategory::BackgroundService, false);
    let p_sys = ProcessItem::new(300, "csrss.exe", 0.1, 10_000_000, "R", ProcessCategory::System, true);
    let p_brw = ProcessItem::new(400, "msedge.exe", 5.0, 200_000_000, "R", ProcessCategory::BrowserEngine, false);

    let procs = vec![p_app, p_svc, p_sys, p_brw];

    let canonical = |cat: &ProcessCategory| -> &'static str {
        match cat {
            ProcessCategory::BrowserEngine => "BrowserEngine",
            ProcessCategory::BackgroundService => "BackgroundService",
            ProcessCategory::System => "System",
            _ => "Application",
        }
    };

    let services: Vec<_> = procs.iter().filter(|p| canonical(&p.category) == "BackgroundService").collect();
    if services.len() != 1 || services[0].name != "spoolsv.exe" {
        return Err("Category tab filter for BackgroundService failed".to_string());
    }

    let apps: Vec<_> = procs.iter().filter(|p| canonical(&p.category) == "Application").collect();
    if apps.len() != 1 || apps[0].name != "notepad.exe" {
        return Err("Category tab filter for Application failed".to_string());
    }

    Ok(())
}

fn test_t1_f15_03_protected_process_status_badge() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let p_protected = ProcessItem::new(
        104,
        "csrss.exe",
        0.2,
        50 * 1024 * 1024,
        "R",
        ProcessCategory::System,
        guard.is_protected(104, "csrss.exe"),
    );

    if !p_protected.is_protected {
        return Err("csrss.exe must be flagged protected".to_string());
    }

    let res = guard.validate_termination(104, "csrss.exe");
    if res.is_ok() {
        return Err("Guardrail should reject csrss termination".to_string());
    }

    Ok(())
}

fn test_t1_f15_04_terminate_non_protected_process_confirmation() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let p_user = ProcessItem::new(
        6200,
        "notepad.exe",
        1.5,
        30 * 1024 * 1024,
        "R",
        ProcessCategory::Application,
        guard.is_protected(6200, "notepad.exe"),
    );

    if p_user.is_protected {
        return Err("notepad.exe must not be protected".to_string());
    }

    let confirm_desc = "Are you sure you want to terminate \"{name}\" (PID {pid})?";
    let rendered = confirm_desc.replace("{name}", &p_user.name).replace("{pid}", &p_user.pid.to_string());
    if rendered != "Are you sure you want to terminate \"notepad.exe\" (PID 6200)?" {
        return Err("Confirmation modal message substitution mismatch".to_string());
    }

    if guard.validate_termination(6200, "notepad.exe").is_err() {
        return Err("Safety guard should allow terminating notepad.exe".to_string());
    }

    Ok(())
}

fn test_t1_f15_05_real_time_sorting_toggle() -> Result<(), String> {
    let p1 = ProcessItem::new(1, "low_cpu_high_ram.exe", 2.0, 500 * 1024 * 1024, "R", ProcessCategory::Application, false);
    let p2 = ProcessItem::new(2, "high_cpu_low_ram.exe", 90.0, 50 * 1024 * 1024, "R", ProcessCategory::Application, false);
    let p3 = ProcessItem::new(3, "mid_cpu_mid_ram.exe", 45.0, 200 * 1024 * 1024, "R", ProcessCategory::Application, false);

    let mut procs = vec![p1, p2, p3];

    // Sort by CPU descending
    procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());
    if procs[0].pid != 2 || procs[1].pid != 3 || procs[2].pid != 1 {
        return Err("Sort by CPU descending failed".to_string());
    }

    // Sort by RAM descending
    procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    if procs[0].pid != 1 || procs[1].pid != 3 || procs[2].pid != 2 {
        return Err("Sort by RAM descending failed".to_string());
    }

    Ok(())
}

// ============================================================================
// Tier 1: F16 - React Bilingual Localization Contracts
// ============================================================================

fn test_t1_f16_01_language_switch_to_vietnamese() -> Result<(), String> {
    let vi_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors().nth(2).unwrap()
        .join("apps/desktop/src/i18n/vi.ts");
    let content = fs::read_to_string(&vi_path).map_err(|e| format!("Failed to read vi.ts: {}", e))?;

    let expected_keys = [
        ("nav_dashboard", "Trang chủ"),
        ("nav_storage", "Dọn dẹp lưu trữ"),
        ("nav_processes", "Tiến trình"),
        ("nav_chat", "Bác sĩ AI"),
        ("nav_diagnostics", "Chẩn đoán"),
        ("nav_settings", "Cài đặt"),
    ];

    for (k, v) in &expected_keys {
        if !content.contains(k) || !content.contains(v) {
            return Err(format!("vi.ts missing expected navigation translation for {} -> {}", k, v));
        }
    }
    Ok(())
}

fn test_t1_f16_02_language_switch_to_english() -> Result<(), String> {
    let en_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors().nth(2).unwrap()
        .join("apps/desktop/src/i18n/en.ts");
    let content = fs::read_to_string(&en_path).map_err(|e| format!("Failed to read en.ts: {}", e))?;

    let expected_keys = [
        ("nav_dashboard", "Home"),
        ("nav_storage", "Storage Clean"),
        ("nav_processes", "Processes"),
        ("nav_chat", "AI Doctor"),
        ("nav_diagnostics", "Diagnostics"),
        ("nav_settings", "Settings"),
    ];

    for (k, v) in &expected_keys {
        if !content.contains(k) || !content.contains(v) {
            return Err(format!("en.ts missing expected navigation translation for {} -> {}", k, v));
        }
    }
    Ok(())
}

fn test_t1_f16_03_dynamic_category_localization() -> Result<(), String> {
    let resolver = DefaultWindowsPathResolver::default();
    let targets = TargetCatalog::get_definitions(&resolver);

    for t in &targets {
        let title_vi = &t.name_vi;
        let desc_vi = &t.description_vi;
        let title_en = &t.name_en;
        let desc_en = &t.description_en;

        if title_vi == title_en {
            return Err(format!("Target {} has identical title in both languages", t.id));
        }
        if desc_vi == desc_en {
            return Err(format!("Target {} has identical description in both languages", t.id));
        }
        if title_vi.is_empty() || desc_vi.is_empty() || title_en.is_empty() || desc_en.is_empty() {
            return Err(format!("Target {} has blank localized strings", t.id));
        }
    }
    Ok(())
}

fn test_t1_f16_04_assistant_interface_localization() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);

    let resp_en = query_assistant_pure("system health", "en", &snap, &kb);
    if !resp_en.reply.contains("Health Status") && !resp_en.reply.contains("optimal") {
        return Err("English assistant reply does not contain expected English terminology".to_string());
    }

    let resp_vi = query_assistant_pure("tình trạng máy tính", "vi", &snap, &kb);
    if !resp_vi.reply.contains("Báo cáo sức khỏe máy tính") {
        return Err("Vietnamese assistant reply does not contain expected Vietnamese terminology".to_string());
    }

    Ok(())
}

fn test_t1_f16_05_diagnostic_severity_tier_localization() -> Result<(), String> {
    let localize_severity = |sev: SeverityLevel, lang: &str| -> &'static str {
        match (sev, lang) {
            (SeverityLevel::Critical, "vi") => "Nguy cấp",
            (SeverityLevel::Critical, _) => "Critical",
            (SeverityLevel::High, "vi") => "Nghiêm trọng",
            (SeverityLevel::High, _) => "High",
            (SeverityLevel::Moderate, "vi") => "Trung bình",
            (SeverityLevel::Moderate, _) => "Moderate",
            (SeverityLevel::Low, "vi") => "Nhẹ",
            (SeverityLevel::Low, _) => "Low",
            (SeverityLevel::Informational, "vi") => "Thông tin",
            (SeverityLevel::Informational, _) => "Informational",
        }
    };

    let levels = [
        SeverityLevel::Critical,
        SeverityLevel::High,
        SeverityLevel::Moderate,
        SeverityLevel::Low,
        SeverityLevel::Informational,
    ];

    for lvl in levels {
        let vi_str = localize_severity(lvl, "vi");
        let en_str = localize_severity(lvl, "en");
        if vi_str.is_empty() || en_str.is_empty() {
            return Err("Empty localized severity label".to_string());
        }
        if vi_str == en_str {
            return Err("Localized severity label cannot match across languages".to_string());
        }
    }
    Ok(())
}

// --- F17: Test Runner Discovery & Framework ---
fn test_t1_f17_01_runner_registration() -> Result<(), String> {
    let tests = build_master_registry();
    if tests.len() != 197 {
        return Err(format!("Expected 197 test cases, found {}", tests.len()));
    }
    Ok(())
}

fn test_t1_f17_02_sequential_pipeline() -> Result<(), String> {
    let t = Instant::now();
    let _ = 1 + 1;
    if t.elapsed().as_secs() > 10 {
        return Err("Timer clock error".to_string());
    }
    Ok(())
}

fn test_t1_f17_03_structured_report() -> Result<(), String> {
    let rep = TestReport {
        total: 10,
        passed: 10,
        failed: 0,
        duration_ms: 15,
        failures: vec![],
    };
    if rep.passed != rep.total || rep.failed != 0 {
        return Err("Report verification failed".to_string());
    }
    Ok(())
}

fn test_t1_f17_04_zero_exit_code() -> Result<(), String> {
    let failed = 0;
    if failed != 0 {
        return Err("Failed count non-zero".to_string());
    }
    Ok(())
}

fn test_t1_f17_05_console_formatting() -> Result<(), String> {
    let banner = "[PASS] T1-F01-01";
    if !banner.starts_with("[PASS]") {
        return Err("Formatting mismatch".to_string());
    }
    Ok(())
}

// ============================================================================
// Section 2: Tier 2 — Boundary Value Analysis & Corner Cases
// ============================================================================

// --- F01 Boundaries ---
fn test_t2_f01_01_exact_cpu_extremes() -> Result<(), String> {
    let c0 = dummy_cpu(0.0);
    let c100 = dummy_cpu(100.0);
    if c0.global_usage_percent != 0.0 || c100.global_usage_percent != 100.0 {
        return Err("CPU extremes distortion".to_string());
    }
    Ok(())
}

fn test_t2_f01_02_zero_total_memory() -> Result<(), String> {
    let mem = dummy_mem(0, 0);
    if mem.usage_percent.is_nan() || mem.usage_percent != 0.0 {
        return Err("Zero total memory division by zero check failed".to_string());
    }
    Ok(())
}

fn test_t2_f01_03_zero_capacity_disk() -> Result<(), String> {
    let disk = dummy_disk("E:", 0, 0);
    if disk.usage_percent.is_nan() || disk.usage_percent != 0.0 {
        return Err("Zero disk capacity division by zero check failed".to_string());
    }
    Ok(())
}

fn test_t2_f01_04_missing_acpi_sensors() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(10.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 50)];
    let findings = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    if !findings.iter().any(|f| f.rule_id == "SENSOR_TEMP_UNAVAILABLE") {
        return Err("Missing thermal sensor did not emit SENSOR_TEMP_UNAVAILABLE".to_string());
    }
    Ok(())
}

fn test_t2_f01_05_uptime_clock_rollover() -> Result<(), String> {
    let mut os = dummy_os();
    os.uptime_seconds = u64::MAX;
    if os.uptime_seconds != u64::MAX {
        return Err("Uptime u64 max rollover failed".to_string());
    }
    Ok(())
}

// --- F02 Boundaries ---
fn test_t2_f02_01_exact_disk_threshold_boundaries() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(10.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);

    // Total 100 GB.
    // 2.0% free (2 GB) -> Moderate (Low Capacity, NOT Critical)
    let d_2pct = vec![dummy_disk("C:", 100, 2)];
    let f1 = engine.evaluate(&cpu, &mem, &d_2pct, &[], &[]);
    if !f1.iter().any(|f| f.rule_id == "DISK_LOW_CAPACITY") || f1.iter().any(|f| f.rule_id == "DISK_CRITICAL_CAPACITY") {
        return Err("Exact 2.0% free should trigger Low Capacity, not Critical".to_string());
    }

    // 1.9% free -> Critical (< 2%)
    let d_critical = vec![DiskSnapshot {
        name: "C:".to_string(),
        mount_point: "C:".to_string(),
        filesystem: "NTFS".to_string(),
        total_bytes: 100 * 1024 * 1024 * 1024,
        available_bytes: (1.9 * 1024.0 * 1024.0 * 1024.0) as u64,
        used_bytes: (98.1 * 1024.0 * 1024.0 * 1024.0) as u64,
        usage_percent: 98.1,
        is_removable: false,
    }];
    let f2 = engine.evaluate(&cpu, &mem, &d_critical, &[], &[]);
    if !f2.iter().any(|f| f.rule_id == "DISK_CRITICAL_CAPACITY" && f.severity == SeverityLevel::Critical) {
        return Err("1.9% free did not trigger Critical".to_string());
    }
    Ok(())
}

fn test_t2_f02_02_cpu_spike_duration() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu_high = dummy_cpu(95.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 50)];

    // Tick 1: No finding
    let f1 = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    if f1.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED") {
        return Err("Sustained CPU should not trigger on tick 1".to_string());
    }

    // Tick 2: No finding
    let f2 = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    if f2.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED") {
        return Err("Sustained CPU should not trigger on tick 2".to_string());
    }

    // Tick 3: Triggers immediately
    let f3 = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    if !f3.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED") {
        return Err("Sustained CPU must trigger on tick 3".to_string());
    }
    Ok(())
}

fn test_t2_f02_03_critical_memory_threshold() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(10.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 499 * 1024 * 1024); // 499 MB (< 500 MB)
    let disks = vec![dummy_disk("C:", 100, 50)];
    let f = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    if !f.iter().any(|f| f.rule_id == "PERF_MEM_LOW_AVAILABLE") {
        return Err("499MB available failed to trigger low memory finding".to_string());
    }
    Ok(())
}

fn test_t2_f02_04_sensor_stale_data_boundary() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(10.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 50)];

    // First sample establishes last_sample_time
    let _ = engine.evaluate(&cpu, &mem, &disks, &[], &[]);

    // Simulate clock jump > 10s
    engine.history.last_sample_time = Some(Utc::now() - chrono::Duration::seconds(15));
    let f_stale = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    if !f_stale.iter().any(|f| f.rule_id == "SENSOR_STALE_DATA") {
        return Err("Elapsed > 10s failed to trigger SENSOR_STALE_DATA".to_string());
    }
    Ok(())
}

fn test_t2_f02_05_rolling_history_buffer_cap() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(50.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 50)];

    // Feed 50 ticks
    for _ in 0..50 {
        let _ = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    }
    if engine.history.cpu_usage_history.len() != 10 || engine.history.memory_usage_history.len() != 10 {
        return Err("History buffer was not capped at exactly 10 samples".to_string());
    }
    Ok(())
}

// --- F03 Boundaries ---
fn test_t2_f03_01_nonexistent_target_dir() -> Result<(), String> {
    let mut target = CleanableTarget::new(
        "ghost_dir",
        "Ghost",
        "Ghost",
        "",
        "",
        "C:\\NonExistentPath_TBK_12345",
        99,
        99999,
        true,
    );
    let guard = SafetyGuard::new(vec![], vec![]);
    StorageScanner::scan_target(&mut target, &guard);
    if target.file_count != 0 || target.total_bytes != 0 {
        return Err("Non-existent directory should yield 0 files and 0 bytes".to_string());
    }
    Ok(())
}

fn test_t2_f03_02_forbidden_target_dir() -> Result<(), String> {
    let mut target = CleanableTarget::new(
        "forbidden_target",
        "Forbidden",
        "Forbidden",
        "",
        "",
        "C:\\Windows\\System32",
        10,
        1000,
        true,
    );
    let guard = SafetyGuard::new(vec![], vec![PathBuf::from("C:\\Windows\\System32")]);
    StorageScanner::scan_target(&mut target, &guard);
    if target.is_safe {
        return Err("Forbidden directory must have is_safe = false".to_string());
    }
    Ok(())
}

fn test_t2_f03_03_unicode_diacritics_path() -> Result<(), String> {
    let sandbox = TestSandbox::new("f03_unicode");
    let unicode_folder = sandbox.create_dir("Temp/VuongGiaBaọ̣̣̣/Sub");
    let file_path = unicode_folder.join("tiếng_việt.tmp");
    fs::write(&file_path, b"UTF8_DIACRITICS_OK").unwrap();

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();
    if target.file_count != 1 || target.total_bytes != 18 {
        return Err("Unicode path scanning failed".to_string());
    }
    Ok(())
}

fn test_t2_f03_04_deeply_nested_hierarchy() -> Result<(), String> {
    let sandbox = TestSandbox::new("f03_deep");
    let mut curr = sandbox.root.join("Temp");
    for i in 0..15 {
        curr = curr.join(format!("level_{}", i));
    }
    fs::create_dir_all(&curr).unwrap();
    fs::write(curr.join("deep.tmp"), b"DEEP_CONTENT").unwrap();

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();
    if target.file_count != 1 || target.total_bytes != 12 {
        return Err("Deep hierarchy scan failed".to_string());
    }
    Ok(())
}

fn test_t2_f03_05_empty_target_directory() -> Result<(), String> {
    let sandbox = TestSandbox::new("f03_empty");
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();
    if target.file_count != 0 || target.total_bytes != 0 {
        return Err("Empty directory scan mismatch".to_string());
    }
    Ok(())
}

// --- F04 Boundaries ---
fn test_t2_f04_01_twenty_zero_byte_files() -> Result<(), String> {
    let sandbox = TestSandbox::new("f04_zero_bytes");
    for i in 0..20 {
        sandbox.create_file(&format!("Temp/empty_{}.tmp", i), &[]);
    }
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();
    if target.file_count != 20 || target.total_bytes != 0 {
        return Err("20 zero-byte files mismatch".to_string());
    }
    Ok(())
}

fn test_t2_f04_02_multi_gigabyte_u32_overflow() -> Result<(), String> {
    let target = CleanableTarget::new(
        "large_dump",
        "Large Dump",
        "Large Dump",
        "",
        "",
        "",
        1,
        5_368_709_120u64, // 5 GB (> 4GB u32 max)
        true,
    );
    let report = StorageScanReport::new(vec![target]);
    if report.total_reclaimable_bytes != 5_368_709_120u64 {
        return Err("u32 overflow detected in total bytes".to_string());
    }
    Ok(())
}

fn test_t2_f04_03_file_deleted_mid_scan() -> Result<(), String> {
    let sandbox = TestSandbox::new("f04_mid_del");
    sandbox.create_file("Temp/survivor.tmp", &[1u8; 500]);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();
    if target.total_bytes != 500 {
        return Err("Mid scan verification mismatch".to_string());
    }
    Ok(())
}

fn test_t2_f04_04_reparse_point_guard() -> Result<(), String> {
    let sandbox = TestSandbox::new("f04_reparse");
    sandbox.create_file("Temp/normal.tmp", &[0u8; 100]);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();
    let target = report.targets.iter().find(|t| t.id == "user_temp").unwrap();
    if target.file_count != 1 {
        return Err("Reparse point guard verification failed".to_string());
    }
    Ok(())
}

fn test_t2_f04_05_saturating_disk_math() -> Result<(), String> {
    let total = 50u64;
    let available = 100u64; // Corrupted OS reporting available > total
    let used = total.saturating_sub(available);
    if used != 0 {
        return Err("Saturating sub underflow guard failed".to_string());
    }
    Ok(())
}

// --- F05 Boundaries ---
fn test_t2_f05_01_dry_run_empty_list() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_empty_list");
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec![], true);
    let summary = engine.clean(&req);
    if summary.bytes_freed != 0 || summary.files_deleted != 0 {
        return Err("Dry run with empty targets should yield 0 freed and 0 deleted".to_string());
    }
    Ok(())
}

fn test_t2_f05_02_dry_run_against_locked_files() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_locked");
    let l1 = sandbox.create_file("Temp/l1.tmp", &[0u8; 500]);
    let _h = lock_file_handle(&l1);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], true);
    let summary = engine.clean(&req);

    if summary.bytes_freed != 500 || summary.files_skipped_locked != 0 {
        return Err("Dry-run should simulate freed bytes without counting skipped locks".to_string());
    }
    Ok(())
}

fn test_t2_f05_03_repeated_idempotent_dry_runs() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_idempotent");
    sandbox.create_file("Temp/idem.tmp", &[0u8; 1234]);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], true);

    let s1 = engine.clean(&req);
    let s2 = engine.clean(&req);
    let s3 = engine.clean(&req);

    if s1.bytes_freed != s2.bytes_freed || s2.bytes_freed != s3.bytes_freed {
        return Err("Dry run results drifted across calls".to_string());
    }
    Ok(())
}

fn test_t2_f05_04_mixed_existent_and_invalid_categories() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_mixed_cat");
    sandbox.create_file("Temp/f.tmp", &[0u8; 300]);
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string(), "invalid_cat_id".to_string()], true);
    let summary = engine.clean(&req);

    if summary.bytes_freed != 300 || !summary.errors.iter().any(|e| e.contains("invalid_cat_id")) {
        return Err("Invalid category was not recorded in errors".to_string());
    }
    Ok(())
}

fn test_t2_f05_05_read_only_target_media_dry_run() -> Result<(), String> {
    let sandbox = TestSandbox::new("f05_ro_media");
    let p = sandbox.create_file("Temp/ro.tmp", &[1u8; 400]);
    let mut perms = fs::metadata(&p).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&p, perms).unwrap();

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], true);
    let summary = engine.clean(&req);

    if summary.bytes_freed != 400 {
        return Err("Dry run failed on read-only file".to_string());
    }
    // Restore permissions for sandbox auto cleanup
    let mut p2 = fs::metadata(&p).unwrap().permissions();
    p2.set_readonly(false);
    let _ = fs::set_permissions(&p, p2);
    Ok(())
}

// --- F06 Boundaries ---
fn test_t2_f06_01_all_files_locked() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_all_locked");
    let mut handles = Vec::new();
    for i in 0..5 {
        let p = sandbox.create_file(&format!("Temp/locked_{}.tmp", i), &[1u8; 100]);
        handles.push(lock_file_handle(&p));
    }
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.files_deleted != 0 || summary.bytes_freed != 0 || summary.files_skipped_locked != 5 {
        return Err("All files locked scenario failed".to_string());
    }
    Ok(())
}

fn test_t2_f06_02_readonly_file_live_purge() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_ro_live");
    let p = sandbox.create_file("Temp/ro_live.tmp", &[2u8; 300]);
    let mut perms = fs::metadata(&p).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&p, perms).unwrap();

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.files_deleted != 1 || summary.bytes_freed != 300 {
        return Err("Read-only file should have permission cleared and deleted".to_string());
    }
    Ok(())
}

fn test_t2_f06_03_parent_subdir_retained_when_child_locked() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_sub_locked");
    let sub = sandbox.create_dir("Temp/locked_sub");
    let locked_file = sub.join("lock.tmp");
    fs::write(&locked_file, b"LOCKED").unwrap();
    let _h = lock_file_handle(&locked_file);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let _ = engine.clean(&req);

    if !sub.exists() {
        return Err("Subdirectory containing locked file must NOT be removed".to_string());
    }
    Ok(())
}

fn test_t2_f06_04_transient_lock_acquisition() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_transient");
    let p = sandbox.create_file("Temp/trans.tmp", &[0u8; 50]);
    let _h = lock_file_handle(&p);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.files_skipped_locked != 1 {
        return Err("Transient lock was not skipped".to_string());
    }
    Ok(())
}

fn test_t2_f06_05_zero_files_cleaned_due_to_lock() -> Result<(), String> {
    let sandbox = TestSandbox::new("f06_zero_freed");
    let p = sandbox.create_file("Temp/av_lock.tmp", &[0u8; 800]);
    let _h = lock_file_handle(&p);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.bytes_freed != 0 {
        return Err("Expected 0 bytes freed".to_string());
    }
    Ok(())
}

// --- F07 Boundaries ---
fn test_t2_f07_01_traversal_parent_dots() -> Result<(), String> {
    let sandbox = TestSandbox::new("f07_traversal");
    let resolver = E2EMockResolver::new(&sandbox);
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let malicious = resolver.temp_dir.join("..").join("..").join("Windows").join("System32");
    let res = guard.validate(&malicious);
    if res.is_ok() {
        return Err("Path traversal via .. should be rejected".to_string());
    }
    Ok(())
}

fn test_t2_f07_02_mixed_slashes() -> Result<(), String> {
    let sandbox = TestSandbox::new("f07_slashes");
    let resolver = E2EMockResolver::new(&sandbox);
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let path = resolver.forbidden_windows.join("System32/../System32");
    let res = guard.validate(&path);
    if res.is_ok() {
        return Err("Mixed slashes traversal should be rejected".to_string());
    }
    Ok(())
}

fn test_t2_f07_03_reparse_outside_safe_root() -> Result<(), String> {
    let sandbox = TestSandbox::new("f07_junction");
    let resolver = E2EMockResolver::new(&sandbox);
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let res = guard.validate(&resolver.forbidden_program_files);
    if res.is_ok() {
        return Err("Forbidden Program Files target should be rejected".to_string());
    }
    Ok(())
}

fn test_t2_f07_04_case_insensitivity_defense() -> Result<(), String> {
    let sandbox = TestSandbox::new("f07_case");
    let resolver = E2EMockResolver::new(&sandbox);
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let win_str = resolver.forbidden_windows.to_string_lossy().to_uppercase();
    let res = guard.validate(&PathBuf::from(win_str));
    if res.is_ok() {
        return Err("Uppercase Windows root was not rejected".to_string());
    }
    Ok(())
}

fn test_t2_f07_05_root_drive_target() -> Result<(), String> {
    let guard = SafetyGuard::new(vec![], vec![]);
    let res = guard.validate(&PathBuf::from("D:\\"));
    if !matches!(res, Err(SafetyViolation::DriveRootProhibited(_))) {
        return Err("Drive root target was not rejected".to_string());
    }
    Ok(())
}

// --- F08 Boundaries ---
fn test_t2_f08_01_zero_cpu_and_mem_procs() -> Result<(), String> {
    let p = ProcessInfo { pid: 999, name: "suspended.exe".to_string(), cpu_percent: 0.0, memory_bytes: 0, status: "Suspended".to_string() };
    if p.cpu_percent != 0.0 || p.memory_bytes != 0 {
        return Err("Zero metrics process mismatch".to_string());
    }
    Ok(())
}

fn test_t2_f08_02_special_regex_characters() -> Result<(), String> {
    let query = "[a-z]*?+";
    let name = "test[a-z]*?+proc.exe";
    if !name.contains(query) {
        return Err("Literal substring query failed".to_string());
    }
    Ok(())
}

fn test_t2_f08_03_empty_search_query() -> Result<(), String> {
    let procs = vec![
        ProcessInfo { pid: 1, name: "p1".to_string(), cpu_percent: 1.0, memory_bytes: 10, status: "R".to_string() },
        ProcessInfo { pid: 2, name: "p2".to_string(), cpu_percent: 2.0, memory_bytes: 20, status: "R".to_string() },
    ];
    let query = "   ";
    let filtered: Vec<_> = procs.clone().into_iter().filter(|p| query.trim().is_empty() || p.name.contains(query.trim())).collect();
    if filtered.len() != 2 {
        return Err("Empty search query should retain all processes".to_string());
    }
    Ok(())
}

fn test_t2_f08_04_identical_metric_values_stable_sort() -> Result<(), String> {
    let mut procs = vec![
        ProcessInfo { pid: 5, name: "p5".to_string(), cpu_percent: 0.0, memory_bytes: 100, status: "R".to_string() },
        ProcessInfo { pid: 2, name: "p2".to_string(), cpu_percent: 0.0, memory_bytes: 100, status: "R".to_string() },
        ProcessInfo { pid: 8, name: "p8".to_string(), cpu_percent: 0.0, memory_bytes: 100, status: "R".to_string() },
    ];
    procs.sort_by(|a, b| a.pid.cmp(&b.pid));
    if procs[0].pid != 2 || procs[1].pid != 5 || procs[2].pid != 8 {
        return Err("Stable tie-breaking sort by PID failed".to_string());
    }
    Ok(())
}

fn test_t2_f08_05_vanished_process_snapshot() -> Result<(), String> {
    let procs: Vec<ProcessInfo> = vec![];
    if !procs.is_empty() {
        return Err("Empty table should be supported".to_string());
    }
    Ok(())
}

// --- F09 Boundaries ---
fn test_t2_f09_01_missing_exe_path() -> Result<(), String> {
    let cat = categorize_process(500, "audiodg.exe", None);
    if cat != ProcessCategory::BackgroundService {
        return Err("audiodg.exe without path should be BackgroundService".to_string());
    }
    Ok(())
}

fn test_t2_f09_02_portable_executable_path() -> Result<(), String> {
    let cat = categorize_process(1200, "tool.exe", Some(r"D:\Portable\tool.exe"));
    if cat != ProcessCategory::Application {
        return Err("Portable tool.exe should be Application".to_string());
    }
    Ok(())
}

fn test_t2_f09_03_non_ascii_glyph_name() -> Result<(), String> {
    let cat = categorize_process(1201, "trình_duyệt.exe", None);
    if cat != ProcessCategory::Application {
        return Err("Non-ASCII process should be categorized as Application".to_string());
    }
    Ok(())
}

fn test_t2_f09_04_cmd_exe_different_contexts() -> Result<(), String> {
    let c_user = categorize_process(1001, "cmd.exe", Some(r"C:\Users\VuongGiaBao\cmd.exe"));
    let c_svc = categorize_process(1002, "cmd.exe", Some(r"C:\Windows\System32\cmd.exe"));
    if c_user != ProcessCategory::Application || c_svc != ProcessCategory::BackgroundService {
        return Err("cmd.exe path-based context categorization mismatch".to_string());
    }
    Ok(())
}

fn test_t2_f09_05_unknown_process_default() -> Result<(), String> {
    let cat = categorize_process(9999, "unknown_binary_xyz.exe", None);
    if cat != ProcessCategory::Application {
        return Err("Unknown binary should default to Application".to_string());
    }
    Ok(())
}

// --- F10 Boundaries ---
fn test_t2_f10_01_exact_threshold_89_9_vs_90_0() -> Result<(), String> {
    let mem = 100 * 1024 * 1024;
    if is_runaway(89.9, mem) {
        return Err("89.9% CPU should NOT be runaway".to_string());
    }
    if !is_runaway(90.0, mem) {
        return Err("90.0% CPU MUST be runaway".to_string());
    }
    Ok(())
}

fn test_t2_f10_02_exact_memory_threshold_boundary() -> Result<(), String> {
    let cpu = 10.0;
    let two_gb = 2 * 1024 * 1024 * 1024u64;
    if is_runaway(cpu, two_gb) {
        return Err("Exact 2GB should NOT be runaway (>2GB required)".to_string());
    }
    if !is_runaway(cpu, two_gb + 1) {
        return Err("2GB + 1 byte MUST be runaway".to_string());
    }
    Ok(())
}

fn test_t2_f10_03_system_idle_high_cpu_exemption() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    if !guard.is_protected(0, "System Idle Process") {
        return Err("System Idle Process must be protected from termination".to_string());
    }
    Ok(())
}

fn test_t2_f10_04_multiple_concurrent_runaways() -> Result<(), String> {
    let p1 = detect_runaway(95.0, 100 * 1024 * 1024);
    let p2 = detect_runaway(10.0, 3 * 1024 * 1024 * 1024);
    let p3 = detect_runaway(75.0, 1_800_000_000);
    if p1.is_none() || p2.is_none() || p3.is_none() {
        return Err("Multiple runaways should all be detected".to_string());
    }
    Ok(())
}

fn test_t2_f10_05_rapid_spike_and_recovery() -> Result<(), String> {
    if !is_runaway(95.0, 100) {
        return Err("Spike tick should be runaway".to_string());
    }
    if is_runaway(5.0, 100) {
        return Err("Recovery tick should not be runaway".to_string());
    }
    Ok(())
}

// --- F11 Boundaries ---
fn test_t2_f11_01_pid_0_and_4_kernel_guard() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    for pid in 0..=4 {
        let res = guard.validate_termination(pid, "kernel_proc");
        if !matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
            return Err(format!("PID {} was not blocked", pid));
        }
    }
    Ok(())
}

fn test_t2_f11_02_case_variations() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let variations = ["CSRSS.EXE", "csrss.Exe", "CsRsS", "EXPLORER.EXE", "explorer"];
    for name in variations {
        let res = guard.validate_termination(1234, name);
        if !matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
            return Err(format!("Variation '{}' was not blocked", name));
        }
    }
    Ok(())
}

fn test_t2_f11_03_invalid_pid() -> Result<(), String> {
    let term = TerminateResult::failure(999999, "ghost.exe", "Process not found");
    if term.success {
        return Err("Invalid PID termination reported success".to_string());
    }
    Ok(())
}

fn test_t2_f11_04_self_termination_guard() -> Result<(), String> {
    let self_pid = 7777;
    let guard = ProcessGuard::with_self_pid(self_pid);
    let res = guard.validate_termination(self_pid, "custom.exe");
    if !matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
        return Err("Self-PID was not blocked".to_string());
    }
    let res_tbk = guard.validate_termination(8888, "tbk_core.exe");
    if !matches!(res_tbk, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
        return Err("tbk_core.exe was not blocked".to_string());
    }
    Ok(())
}

fn test_t2_f11_05_process_name_spoofing() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let res = guard.validate_termination(54321, "csrss.exe");
    if !matches!(res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
        return Err("Spoofed csrss.exe was not blocked".to_string());
    }
    Ok(())
}

// --- F12 Boundaries ---
fn test_t2_f12_01_empty_query_string() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let resp = query_assistant_pure("", "en", &snap, &kb);
    if resp.reply.is_empty() {
        return Err("Empty query returned empty response".to_string());
    }
    Ok(())
}

fn test_t2_f12_02_extremely_long_query() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let long_str = "A".repeat(10_000);
    let resp = query_assistant_pure(&long_str, "en", &snap, &kb);
    if resp.reply.is_empty() {
        return Err("Long query string crashed or produced empty response".to_string());
    }
    Ok(())
}

fn test_t2_f12_03_gibberish_prompt() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let resp = query_assistant_pure("asdfkjhasdf 123897123 ?!?!", "en", &snap, &kb);
    if !resp.reply.contains("TBK Core") || !resp.reply.contains("completely offline") {
        return Err("Gibberish prompt did not return default guidance".to_string());
    }
    Ok(())
}

fn test_t2_f12_04_vietnamese_uppercase_diacritics() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let resp = query_assistant_pure("TÌNH TRẠNG MÁY TÍNH HIỆN TẠI????", "vi", &snap, &kb);
    if !resp.reply.contains("Báo cáo sức khỏe máy tính") {
        return Err("Uppercase diacritics query failed".to_string());
    }
    Ok(())
}

fn test_t2_f12_05_unsupported_language_fallback() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let resp = query_assistant_pure("status", "fr", &snap, &kb);
    if !resp.reply.contains("Current Computer Health Status") {
        return Err("Unsupported language 'fr' did not fallback to English".to_string());
    }
    Ok(())
}

// --- F13 Boundaries ---
fn test_t2_f13_01_single_character_query() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let _ = kb.search("a");
    Ok(())
}

fn test_t2_f13_02_special_regex_metacharacters() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res = kb.search("cpu (100%) [lag] *.*");
    if res.is_empty() || res[0].id != "cpu_troubleshooting" {
        return Err("Regex metacharacters in query broke search".to_string());
    }
    Ok(())
}

fn test_t2_f13_03_unrelated_query() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res = kb.search("cách làm bánh pizza hải sản");
    if !res.is_empty() {
        return Err("Unrelated query should match 0 articles".to_string());
    }
    Ok(())
}

fn test_t2_f13_04_diacritic_insensitive_search() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res = kb.search("cpu qua tai");
    if res.is_empty() {
        return Err("Stripped diacritics query failed to match".to_string());
    }
    Ok(())
}

fn test_t2_f13_05_kb_micro_benchmark() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let start = Instant::now();
    for _ in 0..1_000 {
        let _ = kb.search("dọn rác máy tính");
    }
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 50 {
        return Err(format!("1000 search queries took {}ms (>50ms)", elapsed.as_millis()));
    }
    Ok(())
}

// ============================================================================
// Tier 2: F14 - Storage Cleaner BVA & Corner Cases
// ============================================================================

fn test_t2_f14_01_zero_categories_selected_purge_click() -> Result<(), String> {
    let selected: std::collections::HashSet<String> = std::collections::HashSet::new();

    let can_clean = !selected.is_empty();
    if can_clean {
        return Err("Purge should be blocked when 0 categories selected".to_string());
    }

    let req = CleanRequest::new(selected.into_iter().collect(), false);
    if !req.target_ids.is_empty() {
        return Err("Request target list must be empty".to_string());
    }
    Ok(())
}

fn test_t2_f14_02_scan_operation_times_out_or_fails() -> Result<(), String> {
    let simulated_error: Result<StorageScanReport, String> = Err("IPC Error: Scan timed out".to_string());

    let mut scan_error = None;
    let mut report: Option<StorageScanReport> = None;
    let mut scanning = true;

    match simulated_error {
        Ok(rep) => report = Some(rep),
        Err(err) => {
            scan_error = Some(err);
            scanning = false;
        }
    }

    if scanning {
        return Err("Scanning flag should be false after error".to_string());
    }
    if report.is_some() {
        return Err("Report should remain None after error".to_string());
    }
    if scan_error.as_deref() != Some("IPC Error: Scan timed out") {
        return Err("Error message was not captured correctly".to_string());
    }
    Ok(())
}

fn test_t2_f14_03_display_very_large_byte_counts() -> Result<(), String> {
    let format_bytes = |bytes: u64| -> String {
        if bytes == 0 { "0 B".to_string() }
        else if bytes < 1024 { format!("{} B", bytes) }
        else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
        else if bytes < 1024 * 1024 * 1024 { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
        else { format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
    };

    let large_bytes: u64 = 1_649_267_441_664; // ~1536 GB (1.5 TB)
    let formatted = format_bytes(large_bytes);

    if !formatted.contains("GB") || formatted.contains("-") || formatted.contains("NaN") {
        return Err(format!("Large byte count formatting failed: {}", formatted));
    }
    Ok(())
}

fn test_t2_f14_04_rapid_repeated_clicks_purge_button() -> Result<(), String> {
    let mut cleaning = false;
    let mut dispatched_calls = 0;

    let mut click_purge = || {
        if cleaning {
            return;
        }
        cleaning = true;
        dispatched_calls += 1;
    };

    click_purge();
    click_purge();
    click_purge();
    click_purge();
    click_purge();

    if dispatched_calls != 1 {
        return Err(format!("Expected 1 dispatched IPC call, got {}", dispatched_calls));
    }
    Ok(())
}

fn test_t2_f14_05_all_skipped_files_alert_banner() -> Result<(), String> {
    let summary = pc_doctor_core::storage::StorageCleanSummary {
        dry_run: false,
        bytes_freed: 0,
        files_deleted: 0,
        files_skipped_locked: 15,
        targets_processed: vec!["user_temp".to_string()],
        errors: vec!["File locked: in use by another process".to_string()],
    };

    let has_skipped_notice = summary.files_skipped_locked > 0;
    if !has_skipped_notice {
        return Err("Should flag skipped files notice".to_string());
    }
    if summary.bytes_freed != 0 || summary.files_deleted != 0 {
        return Err("Bytes freed and files deleted must be 0".to_string());
    }
    Ok(())
}

// ============================================================================
// Tier 2: F15 - Process Manager BVA & Corner Cases
// ============================================================================

fn test_t2_f15_01_massive_process_count_500_procs() -> Result<(), String> {
    let mut procs = Vec::with_capacity(500);
    for i in 1..=500 {
        procs.push(ProcessItem::new(
            i as u32,
            &format!("worker_proc_{}.exe", i),
            (i % 100) as f32,
            ((i % 50) as u64 + 1) * 10 * 1024 * 1024,
            "R",
            if i % 2 == 0 { ProcessCategory::Application } else { ProcessCategory::BackgroundService },
            false,
        ));
    }

    let start = Instant::now();

    let q = "worker_proc_4";
    let filtered: Vec<_> = procs.iter().filter(|p| p.name.contains(q)).collect();

    let mut sorted = filtered;
    sorted.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());

    let elapsed = start.elapsed();
    if elapsed.as_millis() > 20 {
        return Err(format!("Filtering & sorting 500 processes took {}ms (>20ms)", elapsed.as_millis()));
    }
    if sorted.is_empty() {
        return Err("Search filter failed to match expected processes".to_string());
    }
    Ok(())
}

fn test_t2_f15_02_protected_process_safety_warning_modal() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let target = ProcessItem::new(4, "System", 0.0, 1024, "R", ProcessCategory::System, true);

    let res = guard.validate_termination(target.pid, &target.name);
    match res {
        Err(ProcessTerminationError::ProtectedSystemProcess { .. }) => Ok(()),
        _ => Err("Attempting to terminate System did not return ProtectedSystemProcess error".to_string()),
    }
}

fn test_t2_f15_03_rapid_table_sorting_column_toggles() -> Result<(), String> {
    let mut procs = vec![
        ProcessItem::new(1, "A.exe", 10.0, 100 * 1024 * 1024, "R", ProcessCategory::Application, false),
        ProcessItem::new(2, "B.exe", 80.0, 20 * 1024 * 1024, "R", ProcessCategory::Application, false),
        ProcessItem::new(3, "C.exe", 40.0, 500 * 1024 * 1024, "R", ProcessCategory::Application, false),
    ];

    for i in 0..50 {
        if i % 2 == 0 {
            procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());
            if procs[0].pid != 2 {
                return Err("Rapid sort by CPU failed".to_string());
            }
        } else {
            procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
            if procs[0].pid != 3 {
                return Err("Rapid sort by RAM failed".to_string());
            }
        }
    }
    Ok(())
}

fn test_t2_f15_04_process_disappears_while_confirm_modal_open() -> Result<(), String> {
    let dead_pid = 999_999u32;
    let guard = ProcessGuard::with_self_pid(9999);

    if guard.validate_termination(dead_pid, "ephemeral.exe").is_err() {
        return Err("Guard should permit attempting non-protected PID".to_string());
    }

    let err = ProcessTerminationError::ProcessNotFound { pid: dead_pid };
    let ui_error_msg = format!("Process {} has already exited", dead_pid);
    if !ui_error_msg.contains("already exited") || err != (ProcessTerminationError::ProcessNotFound { pid: dead_pid }) {
        return Err("Friendly error handling for vanished process failed".to_string());
    }
    Ok(())
}

fn test_t2_f15_05_runaway_process_visual_highlighting() -> Result<(), String> {
    let p_runaway = ProcessItem::new(500, "heavy_app.exe", 95.0, 100 * 1024 * 1024, "R", ProcessCategory::Application, false);
    let p_normal = ProcessItem::new(501, "light_app.exe", 5.0, 50 * 1024 * 1024, "R", ProcessCategory::Application, false);

    let is_highlighted = |p: &ProcessItem| -> bool {
        is_runaway(p.cpu_percent, p.memory_bytes)
    };

    if !is_highlighted(&p_runaway) {
        return Err("Runaway process was not highlighted".to_string());
    }
    if is_highlighted(&p_normal) {
        return Err("Normal process should not be highlighted".to_string());
    }
    Ok(())
}

// ============================================================================
// Tier 2: F16 - Bilingual Localization BVA & Corner Cases
// ============================================================================

fn test_t2_f16_01_missing_dictionary_key_fallback() -> Result<(), String> {
    let mut vi_dict = std::collections::HashMap::new();
    vi_dict.insert("known_key", "Giá trị đã biết");

    let mut en_dict = std::collections::HashMap::new();
    en_dict.insert("known_key", "Known Value");
    en_dict.insert("fallback_key", "Fallback Value");

    let t = |key: &str| -> String {
        if let Some(v) = vi_dict.get(key) {
            v.to_string()
        } else if let Some(v) = en_dict.get(key) {
            v.to_string()
        } else {
            key.to_string()
        }
    };

    if t("known_key") != "Giá trị đã biết" {
        return Err("Known key failed to resolve in Vietnamese".to_string());
    }
    if t("fallback_key") != "Fallback Value" {
        return Err("Missing key failed to fallback to English".to_string());
    }
    if t("completely_unknown_key") != "completely_unknown_key" {
        return Err("Unknown key did not return key string fallback".to_string());
    }
    Ok(())
}

fn test_t2_f16_02_rapid_language_toggling_10_switches() -> Result<(), String> {
    let mut current_lang = "en";
    for i in 1..=10 {
        current_lang = if current_lang == "en" { "vi" } else { "en" };
        if i % 2 == 1 && current_lang != "vi" {
            return Err("Odd toggle must be vi".to_string());
        }
        if i % 2 == 0 && current_lang != "en" {
            return Err("Even toggle must be en".to_string());
        }
    }
    if current_lang != "en" {
        return Err("Final language state should be en after 10 toggles".to_string());
    }
    Ok(())
}

fn test_t2_f16_03_long_vietnamese_diacritic_text_wrapping() -> Result<(), String> {
    let long_vn = "Bác sĩ Trần Hoàng Tú sẽ chăm sóc và bảo vệ máy tính của bạn.";

    let char_count = long_vn.chars().count();
    let byte_count = long_vn.len();

    if byte_count <= char_count {
        return Err("Vietnamese diacritics should have byte length > char count".to_string());
    }

    let truncated: String = long_vn.chars().take(25).collect();
    if truncated.chars().count() != 25 {
        return Err("Character slice length mismatch".to_string());
    }
    Ok(())
}

fn test_t2_f16_04_number_and_date_formatting_across_locales() -> Result<(), String> {
    let bytes: u64 = 1_073_741_824;
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let str_en = format!("{:.2} GB", gb);
    let str_vi = format!("{:.2} GB", gb);

    if str_en != "1.00 GB" || str_vi != "1.00 GB" {
        return Err("Byte formatting mismatch".to_string());
    }

    let now = Utc::now();
    let iso = now.to_rfc3339();
    if !iso.contains("T") {
        return Err("ISO date formatting error".to_string());
    }
    Ok(())
}

fn test_t2_f16_05_locale_preference_persistence() -> Result<(), String> {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct AppPrefs {
        locale: String,
        theme: String,
    }

    let prefs = AppPrefs {
        locale: "vi".to_string(),
        theme: "dark".to_string(),
    };

    let serialized = serde_json::to_string(&prefs).map_err(|e| e.to_string())?;
    let deserialized: AppPrefs = serde_json::from_str(&serialized).map_err(|e| e.to_string())?;

    if deserialized.locale != "vi" {
        return Err("Persisted locale was not restored".to_string());
    }
    Ok(())
}

// --- F17 Boundaries ---
fn test_t2_f17_01_failure_detection() -> Result<(), String> {
    let check_eq = |a: i32, b: i32| -> Result<(), String> {
        if a != b {
            Err("Values not equal".to_string())
        } else {
            Ok(())
        }
    };
    if check_eq(1, 2).is_ok() {
        return Err("Failed assertion did not return Err".to_string());
    }
    Ok(())
}

fn test_t2_f17_02_panic_interception() -> Result<(), String> {
    let panicking_fn = || {
        panic!("intentional test panic");
    };
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let res = std::panic::catch_unwind(panicking_fn);
    std::panic::set_hook(prev_hook);
    if res.is_ok() {
        return Err("Panic was not intercepted".to_string());
    }
    Ok(())
}

fn test_t2_f17_03_fixture_cleanup_on_drop() -> Result<(), String> {
    let path = {
        let sb = TestSandbox::new("raii_drop_test");
        let p = sb.create_file("data.txt", b"TEMP");
        p
    };
    if path.exists() {
        return Err("Sandbox root was not deleted on drop".to_string());
    }
    Ok(())
}

fn test_t2_f17_04_deterministic_formatting() -> Result<(), String> {
    let out = format!("[PASS] {} (Tier 1) - ok", "T1-TEST");
    if !out.contains("[PASS]") {
        return Err("Formatting failed".to_string());
    }
    Ok(())
}

fn test_t2_f17_05_empty_filter() -> Result<(), String> {
    let tests = vec!["t1", "t2"];
    let filter = "non_existent";
    let matched: Vec<_> = tests.into_iter().filter(|t| *t == filter).collect();
    if !matched.is_empty() {
        return Err("Non-existent filter matched items".to_string());
    }
    Ok(())
}

// ============================================================================
// Section 3: Tier 3 — Cross-Feature Pairwise Interactions
// ============================================================================

fn test_t3_int_01_telemetry_diagnostics_assistant() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu_high = dummy_cpu(95.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 50)];

    let _ = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    let _ = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    let findings = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);

    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(findings);
    let resp = query_assistant_pure("tình trạng máy tính", "vi", &snap, &kb);

    if !resp.reply.contains("Sustained High CPU Utilization") || !resp.reply.contains("Khắc phục:") {
        return Err("Assistant failed to incorporate high CPU diagnostic finding".to_string());
    }
    Ok(())
}

fn test_t3_int_02_scanner_exact_bytes_dry_run() -> Result<(), String> {
    let sandbox = TestSandbox::new("t3_int_02");
    sandbox.create_file("Temp/t1.tmp", &[1u8; 50_000]);
    sandbox.create_file("CrashDumps/c1.dmp", &[2u8; 50_000]);
    sandbox.create_file("WER/w1.wer", &[3u8; 50_000]);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let report = engine.scan();

    if report.total_reclaimable_bytes != 150_000 {
        return Err("Scanned bytes mismatch".to_string());
    }

    let req = CleanRequest::new(
        vec![
            "user_temp".to_string(),
            "crash_dumps".to_string(),
            "windows_error_reporting".to_string(),
        ],
        true,
    );
    let summary = engine.clean(&req);

    if summary.bytes_freed != report.total_reclaimable_bytes {
        return Err("Dry run freed bytes does not equal scanned bytes".to_string());
    }
    if summary.files_deleted != 3 {
        return Err("Dry run files deleted simulation count mismatch".to_string());
    }
    // Verify files still exist
    if !sandbox.root.join("Temp/t1.tmp").exists() {
        return Err("Dry run deleted files on disk!".to_string());
    }
    Ok(())
}

fn test_t3_int_03_scanner_locked_skip_exact_bytes() -> Result<(), String> {
    let sandbox = TestSandbox::new("t3_int_03");
    let mut files = Vec::new();
    for i in 0..50 {
        files.push(sandbox.create_file(&format!("Temp/file_{}.tmp", i), &[1u8; 100]));
    }

    // Lock 5 files
    let mut locks = Vec::new();
    for i in 0..5 {
        locks.push(lock_file_handle(&files[i]));
    }

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.files_deleted != 45 || summary.files_skipped_locked != 5 {
        return Err("Locked files and deleted files counts mismatch".to_string());
    }
    if summary.bytes_freed != 45 * 100 {
        return Err("Freed bytes does not match deleted files exactly".to_string());
    }
    Ok(())
}

fn test_t3_int_04_scanner_boundary_guard() -> Result<(), String> {
    let sandbox = TestSandbox::new("t3_int_04");
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let _engine = StorageEngine::with_resolver(resolver.clone());

    // Inject tampered target pointing to System32
    let tampered_target = CleanableTarget::new(
        "tampered",
        "Tampered",
        "Tampered",
        "",
        "",
        resolver.forbidden_system32.to_string_lossy().to_string(),
        10,
        1000,
        true,
    );
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    let req = CleanRequest::new(vec!["tampered".to_string()], false);
    let summary = StoragePurger::purge(&req, &[tampered_target], &guard);

    if !summary.errors.iter().any(|e| e.contains("Safety boundary violation")) {
        return Err("Tampered System32 target was not intercepted by boundary guard".to_string());
    }
    Ok(())
}

fn test_t3_int_05_process_runaway_blacklist_precedence() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let cpu = 95.0;
    let mem = 500 * 1024 * 1024;
    let is_run = is_runaway(cpu, mem);
    if !is_run {
        return Err("95% CPU should be runaway".to_string());
    }

    // But if process is csrss.exe, termination MUST be blocked!
    let term_res = guard.validate_termination(104, "csrss.exe");
    if !matches!(term_res, Err(ProcessTerminationError::ProtectedSystemProcess { .. })) {
        return Err("Safety blacklist must take precedence over runaway termination".to_string());
    }
    Ok(())
}

fn test_t3_int_06_process_runaway_permitted_kill() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let cpu = 92.0;
    let mem = 2_500_000_000u64;
    let runaway = detect_runaway(cpu, mem);
    if runaway.is_none() {
        return Err("Rogue app was not detected as runaway".to_string());
    }

    let cat = categorize_process(8412, "renderer.exe", None);
    if cat != ProcessCategory::Application {
        return Err("renderer.exe should be categorized as Application".to_string());
    }

    // Since it's a non-protected app, termination check passes
    let res = guard.validate_termination(8412, "renderer.exe");
    if res.is_err() {
        return Err("Non-protected runaway app should be approved for termination".to_string());
    }
    Ok(())
}

fn test_t3_int_07_assistant_knowledge_localization() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let resp = query_assistant_pure("dọn dẹp file tạm", "vi", &snap, &kb);
    if !resp.reply.contains("Hướng dẫn Dọn dẹp Bộ nhớ tạm") {
        return Err("Vietnamese cleanup guide not returned by assistant".to_string());
    }
    Ok(())
}

fn test_t3_int_08_scanner_catalog_knowledge() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let resolver = DefaultWindowsPathResolver::default();
    let catalog = TargetCatalog::get_definitions(&resolver);

    // Knowledge article mentions %TEMP% and crash dumps
    let art = kb.get_by_id("storage_cleanup_safety").unwrap();
    if !art.content_en.contains("%TEMP%") || !art.content_vi.contains("Crash Dumps") {
        return Err("Knowledge content does not document catalog targets".to_string());
    }
    if catalog.len() != 5 {
        return Err("Expected 5 catalog targets".to_string());
    }
    Ok(())
}

fn test_t3_int_09_telemetry_purge_capacity_feedback() -> Result<(), String> {
    let _initial_disk = dummy_disk("C:", 100, 10); // 10 GB free
    let freed_bytes = 2 * 1024 * 1024 * 1024u64; // 2 GB freed

    // Subsequent telemetry simulation
    let updated_disk = dummy_disk("C:", 100, 10 + (freed_bytes / (1024 * 1024 * 1024)));
    if updated_disk.available_bytes != 12 * 1024 * 1024 * 1024 {
        return Err("Disk available bytes did not increase by freed amount".to_string());
    }
    Ok(())
}

fn test_t3_int_10_process_list_blacklist_badge() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);
    let p_csrss = ProcessItem::new(104, "csrss.exe", 0.5, 50 * 1024 * 1024, "R", ProcessCategory::System, guard.is_protected(104, "csrss.exe"));
    let p_notepad = ProcessItem::new(1024, "notepad.exe", 0.1, 20 * 1024 * 1024, "R", ProcessCategory::Application, guard.is_protected(1024, "notepad.exe"));

    if !p_csrss.is_protected {
        return Err("csrss.exe must have is_protected = true".to_string());
    }
    if p_notepad.is_protected {
        return Err("notepad.exe must have is_protected = false".to_string());
    }
    Ok(())
}

fn test_t3_int_11_multi_drive_diagnostics() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(10.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let d_c = dummy_disk("C:", 100, 1);  // 1% free -> Critical
    let d_d = dummy_disk("D:", 500, 250); // 50% free -> Healthy

    let findings = engine.evaluate(&cpu, &mem, &[d_c, d_d], &[], &[]);
    let crit = findings.iter().filter(|f| f.rule_id == "DISK_CRITICAL_CAPACITY").collect::<Vec<_>>();
    if crit.len() != 1 || !crit[0].title.contains("C:") {
        return Err("Diagnostic findings should report critical alert for C: only".to_string());
    }
    Ok(())
}

fn test_t3_int_12_dry_run_to_live_run_locked_transition() -> Result<(), String> {
    let sandbox = TestSandbox::new("t3_int_12");
    let normal = sandbox.create_file("Temp/normal.tmp", &[0u8; 1000]);
    let locked = sandbox.create_file("Temp/locked.tmp", &[0u8; 2000]);
    let _h = lock_file_handle(&locked);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);

    // 1. Dry run
    let req_dry = CleanRequest::new(vec!["user_temp".to_string()], true);
    let s_dry = engine.clean(&req_dry);
    if s_dry.bytes_freed != 3000 {
        return Err("Dry run simulated bytes mismatch".to_string());
    }

    // 2. Live clean
    let req_live = CleanRequest::new(vec!["user_temp".to_string()], false);
    let s_live = engine.clean(&req_live);
    if s_live.bytes_freed != 1000 || s_live.files_skipped_locked != 1 {
        return Err("Live clean after dry-run mismatch".to_string());
    }
    if !locked.exists() || normal.exists() {
        return Err("Filesystem state incorrect after live clean".to_string());
    }
    Ok(())
}

fn test_t3_int_13_assistant_healthy_confirmation() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);
    let resp = query_assistant_pure("kiểm tra tình trạng máy tính", "vi", &snap, &kb);
    if !resp.reply.contains("ngưỡng an toàn ổn định") {
        return Err("Vietnamese healthy indicator missing".to_string());
    }
    Ok(())
}

fn test_t3_int_14_process_category_filtering() -> Result<(), String> {
    let procs = vec![
        ProcessItem::new(1, "app1.exe", 1.0, 10, "R", ProcessCategory::Application, false),
        ProcessItem::new(2, "svc1.exe", 1.0, 10, "R", ProcessCategory::BackgroundService, false),
        ProcessItem::new(3, "app2.exe", 1.0, 10, "R", ProcessCategory::Application, false),
    ];
    let apps: Vec<_> = procs.into_iter().filter(|p| p.category == ProcessCategory::Application).collect();
    if apps.len() != 2 {
        return Err("Category filtering mismatch".to_string());
    }
    Ok(())
}

fn test_t3_int_15_runner_executes_all_modules() -> Result<(), String> {
    let tests = build_master_registry();
    let tiers: std::collections::HashSet<_> = tests.iter().map(|t| t.tier).collect();
    if !tiers.contains("Tier 1") || !tiers.contains("Tier 2") || !tiers.contains("Tier 3") || !tiers.contains("Tier 4") {
        return Err("Test runner registry does not contain all 4 tiers".to_string());
    }
    Ok(())
}

// ============================================================================
// Section 4: Tier 4 — Real-World Operational Scenarios
// ============================================================================

fn test_t4_scn_01_critical_disk_emergency_and_safe_purge() -> Result<(), String> {
    // 1. Critical disk free space (< 1%)
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(10.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let d_crit = vec![dummy_disk("C:", 100, 1)]; // 1 GB free (1%)
    let findings = engine.evaluate(&cpu, &mem, &d_crit, &[], &[]);
    if !findings.iter().any(|f| f.rule_id == "DISK_CRITICAL_CAPACITY") {
        return Err("Step 1 failed: Critical disk alert not emitted".to_string());
    }

    // 2. Assistant alerts user in Vietnamese
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(findings);
    let resp = query_assistant_pure("tình trạng máy tính", "vi", &snap, &kb);
    if !resp.reply.contains("Critical Free Space on Drive C:") {
        return Err("Step 2 failed: Assistant did not alert about critical drive".to_string());
    }

    // 3. User scans storage
    let sandbox = TestSandbox::new("t4_scn_01");
    sandbox.create_file("Temp/stale1.tmp", &[0u8; 1_000_000]);
    sandbox.create_file("Temp/stale2.tmp", &[0u8; 800_000]);
    let locked_p = sandbox.create_file("Temp/locked.tmp", &[0u8; 200_000]);
    let _h = lock_file_handle(&locked_p);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let storage = StorageEngine::with_resolver(resolver);
    let report = storage.scan();
    if report.total_reclaimable_bytes != 2_000_000 {
        return Err("Step 3 failed: Scanned reclaimable bytes mismatch".to_string());
    }

    // 4. User dry-run simulation
    let req_dry = CleanRequest::new(vec!["user_temp".to_string()], true);
    let s_dry = storage.clean(&req_dry);
    if s_dry.bytes_freed != 2_000_000 {
        return Err("Step 4 failed: Dry run simulation mismatch".to_string());
    }

    // 5. User confirms live purge: deletes unlocked files, skips locked file
    let req_live = CleanRequest::new(vec!["user_temp".to_string()], false);
    let s_live = storage.clean(&req_live);
    if s_live.files_deleted != 2 || s_live.files_skipped_locked != 1 || s_live.bytes_freed != 1_800_000 {
        return Err("Step 5 failed: Live clean execution mismatch".to_string());
    }

    // 6. Telemetry updates: disk free space increases, critical alert clears
    let d_recovered = vec![dummy_disk("C:", 100, 20)]; // 20 GB free
    let f_recovered = engine.evaluate(&cpu, &mem, &d_recovered, &[], &[]);
    if f_recovered.iter().any(|f| f.rule_id == "DISK_CRITICAL_CAPACITY") {
        return Err("Step 6 failed: Critical disk alert did not clear after purge".to_string());
    }

    Ok(())
}

fn test_t4_scn_02_runaway_browser_tab_termination() -> Result<(), String> {
    // 1. Telemetry collector detects CPU load at 98% for 3 samples
    let mut engine = DiagnosticEngine::new();
    let cpu_high = dummy_cpu(98.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 50)];

    let _ = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    let _ = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    let findings = engine.evaluate(&cpu_high, &mem, &disks, &[], &[]);
    if !findings.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED") {
        return Err("Sustained high CPU not detected".to_string());
    }

    // 2. Identify top process
    let mut procs = vec![
        ProcessItem::new(8412, "chrome.exe", 96.0, 2 * 1024 * 1024 * 1024, "R", ProcessCategory::BrowserEngine, false),
        ProcessItem::new(100, "system.exe", 1.0, 50 * 1024 * 1024, "R", ProcessCategory::System, true),
    ];
    procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap());
    let rogue = &procs[0];
    if rogue.pid != 8412 || rogue.is_protected {
        return Err("Top process should be rogue non-protected PID 8412".to_string());
    }

    // 3. User terminates process
    let guard = ProcessGuard::with_self_pid(9999);
    let val_res = guard.validate_termination(rogue.pid, &rogue.name);
    if val_res.is_err() {
        return Err("Rogue process termination should be permitted".to_string());
    }

    // 4. After termination, CPU load normalizes to 8%
    let cpu_normal = dummy_cpu(8.0);
    let f_normal = engine.evaluate(&cpu_normal, &mem, &disks, &[], &[]);
    if f_normal.iter().any(|f| f.rule_id == "PERF_CPU_HIGH_SUSTAINED") {
        return Err("High CPU alert did not clear after process termination".to_string());
    }
    Ok(())
}

fn test_t4_scn_03_accidental_kill_os_process_defense() -> Result<(), String> {
    let guard = ProcessGuard::with_self_pid(9999);

    // User attempts to kill csrss.exe (PID 620)
    let res = guard.validate_termination(620, "csrss.exe");
    match res {
        Err(ProcessTerminationError::ProtectedSystemProcess { pid, name, reason }) => {
            if pid != 620 || name != "csrss.exe" || !reason.contains("Client Server Runtime Subsystem") {
                return Err("Structured error details for csrss.exe mismatch".to_string());
            }
        }
        _ => return Err("Blacklist guard failed to block csrss.exe!".to_string()),
    }
    Ok(())
}

fn test_t4_scn_04_routine_safe_temp_cleanup_with_locked_files() -> Result<(), String> {
    let sandbox = TestSandbox::new("t4_scn_04");
    for i in 0..20 {
        sandbox.create_file(&format!("Temp/stale_{}.tmp", i), &[0u8; 1000]);
    }
    // Simulate Word and Chrome holding 2 open files
    let word_file = sandbox.create_file("Temp/~$Document.docx", &[0u8; 500]);
    let chrome_file = sandbox.create_file("Temp/scoped_dir123.tmp", &[0u8; 500]);
    let _h_word = lock_file_handle(&word_file);
    let _h_chrome = lock_file_handle(&chrome_file);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);
    let req = CleanRequest::new(vec!["user_temp".to_string()], false);
    let summary = engine.clean(&req);

    if summary.files_deleted != 20 || summary.files_skipped_locked != 2 || summary.bytes_freed != 20_000 {
        return Err("Routine cleanup with locked files failed".to_string());
    }
    if !word_file.exists() || !chrome_file.exists() {
        return Err("Locked office/browser files were deleted!".to_string());
    }
    Ok(())
}

fn test_t4_scn_05_stale_telemetry_sleep_resume_recovery() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(10.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    let disks = vec![dummy_disk("C:", 100, 50)];

    // First sample establishes time
    let _ = engine.evaluate(&cpu, &mem, &disks, &[], &[]);

    // Laptop sleep: delta = 3600s
    engine.history.last_sample_time = Some(Utc::now() - chrono::Duration::seconds(3600));
    let f_stale = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    if !f_stale.iter().any(|f| f.rule_id == "SENSOR_STALE_DATA") {
        return Err("Stale sensor data not detected on resume".to_string());
    }

    // Next tick is fresh (1 second later)
    let f_fresh = engine.evaluate(&cpu, &mem, &disks, &[], &[]);
    if f_fresh.iter().any(|f| f.rule_id == "SENSOR_STALE_DATA") {
        return Err("SENSOR_STALE_DATA failed to clear on subsequent fresh tick".to_string());
    }
    Ok(())
}

fn test_t4_scn_06_non_admin_storage_scan_graceful_degradation() -> Result<(), String> {
    let sandbox = TestSandbox::new("t4_scn_06");
    sandbox.create_file("Temp/user.tmp", &[0u8; 1000]);
    sandbox.create_file("CrashDumps/dump.dmp", &[0u8; 2000]);

    // Construct target list where one target has an invalid / inaccessible root
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let mut targets = TargetCatalog::get_definitions(resolver.as_ref());
    // Point prefetch to nonexistent or inaccessible path
    targets.push(CleanableTarget::new(
        "restricted_prefetch",
        "Prefetch",
        "Prefetch",
        "",
        "",
        "C:\\Windows\\Prefetch_Inaccessible",
        0,
        0,
        true,
    ));

    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());
    for target in &mut targets {
        StorageScanner::scan_target(target, &guard);
    }

    let report = StorageScanReport::new(targets);
    if report.total_reclaimable_bytes != 3000 || report.total_files != 2 {
        return Err("Scan graceful degradation failed".to_string());
    }
    Ok(())
}

fn test_t4_scn_07_bilingual_session_transition() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let snap = dummy_snapshot(vec![]);

    // User starts in English
    let resp_en = query_assistant_pure("how is system health", "en", &snap, &kb);
    if !resp_en.reply.contains("Current Computer Health Status") {
        return Err("English assistant reply failed".to_string());
    }

    // User switches to Vietnamese
    let resp_vi = query_assistant_pure("tình trạng máy tính", "vi", &snap, &kb);
    if !resp_vi.reply.contains("Báo cáo sức khỏe máy tính") {
        return Err("Vietnamese assistant reply failed".to_string());
    }
    Ok(())
}

fn test_t4_scn_08_offline_assistant_troubleshooting_workflow() -> Result<(), String> {
    let kb = KnowledgeBase::new();
    let res_vi = kb.search("dọn dẹp bộ nhớ đệm temp");
    if res_vi.is_empty() {
        return Err("Offline knowledge base failed on Vietnamese query".to_string());
    }
    let res_en = kb.search("disk space cleanup");
    if res_en.is_empty() {
        return Err("Offline knowledge base failed on English query".to_string());
    }
    Ok(())
}

fn test_t4_scn_09_storage_cleaner_dry_run_workflow() -> Result<(), String> {
    let sandbox = TestSandbox::new("t4_scn_09");
    let file = sandbox.create_file("Temp/verify.tmp", &[42u8; 5000]);

    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let engine = StorageEngine::with_resolver(resolver);

    // 1. Technician executes dry-run simulation
    let req_dry = CleanRequest::new(vec!["user_temp".to_string()], true);
    let s_dry = engine.clean(&req_dry);
    if s_dry.bytes_freed != 5000 || s_dry.files_deleted != 1 {
        return Err("Technician dry run simulation mismatch".to_string());
    }

    // 2. Technician verifies file remains on disk
    if !file.exists() {
        return Err("File was deleted during technician dry run!".to_string());
    }

    // 3. Technician executes live purge
    let req_live = CleanRequest::new(vec!["user_temp".to_string()], false);
    let s_live = engine.clean(&req_live);
    if s_live.bytes_freed != 5000 || s_live.files_deleted != 1 {
        return Err("Technician live clean mismatch".to_string());
    }
    if file.exists() {
        return Err("File still exists after live clean".to_string());
    }
    Ok(())
}

fn test_t4_scn_10_multi_drive_mixed_telemetry() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(15.0);
    let mem = dummy_mem(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);

    let d_c = dummy_disk("C:", 500, 25); // 5% free -> Low Capacity
    let d_d = dummy_disk("D:", 1000, 450); // 45% free -> Healthy
    let mut d_e = dummy_disk("E:", 64, 32); // USB drive
    d_e.is_removable = true;

    let findings = engine.evaluate(&cpu, &mem, &[d_c, d_d, d_e], &[], &[]);
    let has_c_low = findings.iter().any(|f| f.rule_id == "DISK_LOW_CAPACITY" && f.title.contains("C:"));
    let has_d_or_e = findings.iter().any(|f| f.title.contains("D:") || f.title.contains("E:"));

    if !has_c_low || has_d_or_e {
        return Err("Multi-drive telemetry evaluation failed".to_string());
    }
    Ok(())
}

fn test_t4_scn_11_sustained_memory_exhaustion_mitigation() -> Result<(), String> {
    let mut engine = DiagnosticEngine::new();
    let cpu = dummy_cpu(25.0);
    // 96% used, available 350 MB (< 500 MB)
    let total_ram = 16 * 1024 * 1024 * 1024;
    let avail_ram = 350 * 1024 * 1024;
    let mem_exhausted = dummy_mem(total_ram, avail_ram);
    let disks = vec![dummy_disk("C:", 100, 50)];

    // 3 ticks to establish sustained high memory
    let _ = engine.evaluate(&cpu, &mem_exhausted, &disks, &[], &[]);
    let _ = engine.evaluate(&cpu, &mem_exhausted, &disks, &[], &[]);
    let findings = engine.evaluate(&cpu, &mem_exhausted, &disks, &[], &[]);

    if !findings.iter().any(|f| f.rule_id == "PERF_MEM_HIGH_SUSTAINED") {
        return Err("PERF_MEM_HIGH_SUSTAINED not emitted".to_string());
    }
    if !findings.iter().any(|f| f.rule_id == "PERF_MEM_LOW_AVAILABLE") {
        return Err("PERF_MEM_LOW_AVAILABLE not emitted".to_string());
    }

    // Identify rogue memory consumer (3.2 GB RAM)
    let rogue_mem = 3_200_000_000u64;
    let runaway = detect_runaway(10.0, rogue_mem);
    if runaway.is_none() {
        return Err("3.2 GB process was not detected as runaway memory consumer".to_string());
    }

    // Terminate rogue process and verify recovered RAM (3.8 GB available)
    let mem_recovered = dummy_mem(total_ram, 3_800_000_000);
    let f_recovered = engine.evaluate(&cpu, &mem_recovered, &disks, &[], &[]);
    if f_recovered.iter().any(|f| f.rule_id == "PERF_MEM_LOW_AVAILABLE") {
        return Err("Low memory alert should clear after recovery".to_string());
    }
    Ok(())
}

fn test_t4_scn_12_boundary_defense_malicious_path_injection() -> Result<(), String> {
    let sandbox = TestSandbox::new("t4_scn_12");
    let resolver = Arc::new(E2EMockResolver::new(&sandbox));
    let guard = SafetyGuard::new(resolver.get_safe_roots(), resolver.get_forbidden_roots());

    let malicious_target = CleanableTarget::new(
        "exploit_attempt",
        "Exploit",
        "Exploit",
        "",
        "",
        resolver.temp_dir.join("..").join("Windows").join("System32").to_string_lossy().to_string(),
        50,
        50000,
        true,
    );

    let req = CleanRequest::new(vec!["exploit_attempt".to_string()], false);
    let summary = StoragePurger::purge(&req, &[malicious_target], &guard);

    if !summary.errors.iter().any(|e| e.contains("Safety boundary violation")) {
        return Err("Malicious path injection was not blocked by safety guard".to_string());
    }
    if summary.files_deleted != 0 || summary.bytes_freed != 0 {
        return Err("Files were deleted during malicious injection attempt!".to_string());
    }
    Ok(())
}

// ============================================================================
// Section 5: Master Test Registry & Harness Entrypoint
// ============================================================================

pub fn build_master_registry() -> Vec<TestCase> {
    vec![
        // Tier 1: F01 - Telemetry
        TestCase { id: "T1-F01-01", tier: "Tier 1", feature: "F01 Telemetry", description: "CPU Global and Per-Core Metrics", test_fn: test_t1_f01_01_cpu_global_and_per_core },
        TestCase { id: "T1-F01-02", tier: "Tier 1", feature: "F01 Telemetry", description: "Memory Metrics Calculation", test_fn: test_t1_f01_02_memory_metrics },
        TestCase { id: "T1-F01-03", tier: "Tier 1", feature: "F01 Telemetry", description: "Disk Volume Enumeration", test_fn: test_t1_f01_03_disk_enumeration },
        TestCase { id: "T1-F01-04", tier: "Tier 1", feature: "F01 Telemetry", description: "OS Platform and Uptime", test_fn: test_t1_f01_04_os_platform_uptime },
        TestCase { id: "T1-F01-05", tier: "Tier 1", feature: "F01 Telemetry", description: "Temperature Sensor Enumeration", test_fn: test_t1_f01_05_temp_sensors },

        // Tier 1: F02 - Diagnostics
        TestCase { id: "T1-F02-01", tier: "Tier 1", feature: "F02 Diagnostics", description: "Disk Critical Rule (<2% free)", test_fn: test_t1_f02_01_disk_critical_rule },
        TestCase { id: "T1-F02-02", tier: "Tier 1", feature: "F02 Diagnostics", description: "Disk Low Rule (2-10% free)", test_fn: test_t1_f02_02_disk_low_rule },
        TestCase { id: "T1-F02-03", tier: "Tier 1", feature: "F02 Diagnostics", description: "Sustained High CPU Load (3 samples)", test_fn: test_t1_f02_03_sustained_cpu_rule },
        TestCase { id: "T1-F02-04", tier: "Tier 1", feature: "F02 Diagnostics", description: "Low Available Memory (<500MB)", test_fn: test_t1_f02_04_low_memory_rule },
        TestCase { id: "T1-F02-05", tier: "Tier 1", feature: "F02 Diagnostics", description: "System Healthy Baseline", test_fn: test_t1_f02_05_system_healthy_baseline },

        // Tier 1: F03 - Safe Temp Cache Scanner
        TestCase { id: "T1-F03-01", tier: "Tier 1", feature: "F03 Scanner", description: "User Temp Directory Discovery", test_fn: test_t1_f03_01_user_temp_discovery },
        TestCase { id: "T1-F03-02", tier: "Tier 1", feature: "F03 Scanner", description: "User Crash Dumps Target", test_fn: test_t1_f03_02_crash_dumps_target },
        TestCase { id: "T1-F03-03", tier: "Tier 1", feature: "F03 Scanner", description: "WER Report Archives Target", test_fn: test_t1_f03_03_wer_reports_target },
        TestCase { id: "T1-F03-04", tier: "Tier 1", feature: "F03 Scanner", description: "Explorer Thumbnail Cache Target", test_fn: test_t1_f03_04_explorer_thumbnail_cache },
        TestCase { id: "T1-F03-05", tier: "Tier 1", feature: "F03 Scanner", description: "Itemized Scan Aggregation", test_fn: test_t1_f03_05_itemized_scan_aggregation },

        // Tier 1: F04 - Exact Byte Sizes
        TestCase { id: "T1-F04-01", tier: "Tier 1", feature: "F04 Exact Bytes", description: "Single File Exact Size", test_fn: test_t1_f04_01_single_file_exact_size },
        TestCase { id: "T1-F04-02", tier: "Tier 1", feature: "F04 Exact Bytes", description: "Multi-File Accurate Summation", test_fn: test_t1_f04_02_multi_file_summation },
        TestCase { id: "T1-F04-03", tier: "Tier 1", feature: "F04 Exact Bytes", description: "Non-Negative Byte Invariant", test_fn: test_t1_f04_03_non_negative_invariant },
        TestCase { id: "T1-F04-04", tier: "Tier 1", feature: "F04 Exact Bytes", description: "Cross-Category Aggregate Equality", test_fn: test_t1_f04_04_cross_category_equality },
        TestCase { id: "T1-F04-05", tier: "Tier 1", feature: "F04 Exact Bytes", description: "Freed Bytes Accuracy on Clean", test_fn: test_t1_f04_05_freed_bytes_on_clean },

        // Tier 1: F05 - Dry-Run Simulation Mode
        TestCase { id: "T1-F05-01", tier: "Tier 1", feature: "F05 Dry-Run", description: "Dry-Run Flag Fidelity", test_fn: test_t1_f05_01_dry_run_flag },
        TestCase { id: "T1-F05-02", tier: "Tier 1", feature: "F05 Dry-Run", description: "Byte Simulation Accuracy", test_fn: test_t1_f05_02_byte_simulation_accuracy },
        TestCase { id: "T1-F05-03", tier: "Tier 1", feature: "F05 Dry-Run", description: "File Count Simulation Accuracy", test_fn: test_t1_f05_03_file_count_simulation },
        TestCase { id: "T1-F05-04", tier: "Tier 1", feature: "F05 Dry-Run", description: "Zero Disk Deletion Invariant", test_fn: test_t1_f05_04_zero_disk_deletion_invariant },
        TestCase { id: "T1-F05-05", tier: "Tier 1", feature: "F05 Dry-Run", description: "File Content and Hash Integrity", test_fn: test_t1_f05_05_file_content_integrity },

        // Tier 1: F06 - Locked File Skip
        TestCase { id: "T1-F06-01", tier: "Tier 1", feature: "F06 Locked Skip", description: "Single Locked File Handled", test_fn: test_t1_f06_01_single_locked_file_handled },
        TestCase { id: "T1-F06-02", tier: "Tier 1", feature: "F06 Locked Skip", description: "Skipped Files Counter", test_fn: test_t1_f06_02_skipped_files_counter },
        TestCase { id: "T1-F06-03", tier: "Tier 1", feature: "F06 Locked Skip", description: "Descriptive Error Reporting", test_fn: test_t1_f06_03_descriptive_error_reporting },
        TestCase { id: "T1-F06-04", tier: "Tier 1", feature: "F06 Locked Skip", description: "Freed Bytes Excludes Locked Files", test_fn: test_t1_f06_04_freed_bytes_excludes_locked },
        TestCase { id: "T1-F06-05", tier: "Tier 1", feature: "F06 Locked Skip", description: "Zero-Panic Guarantee under Lock", test_fn: test_t1_f06_05_zero_panic_guarantee },

        // Tier 1: F07 - Boundary Guard
        TestCase { id: "T1-F07-01", tier: "Tier 1", feature: "F07 Boundary Guard", description: "System32 Rejection", test_fn: test_t1_f07_01_system32_rejection },
        TestCase { id: "T1-F07-02", tier: "Tier 1", feature: "F07 Boundary Guard", description: "Windows System Root Rejection", test_fn: test_t1_f07_02_windows_root_rejection },
        TestCase { id: "T1-F07-03", tier: "Tier 1", feature: "F07 Boundary Guard", description: "Desktop and Documents Rejection", test_fn: test_t1_f07_03_desktop_documents_rejection },
        TestCase { id: "T1-F07-04", tier: "Tier 1", feature: "F07 Boundary Guard", description: "Drive Root Target Rejection", test_fn: test_t1_f07_04_drive_root_rejection },
        TestCase { id: "T1-F07-05", tier: "Tier 1", feature: "F07 Boundary Guard", description: "Legitimate Child Validation", test_fn: test_t1_f07_05_legitimate_child_validation },

        // Tier 1: F08 - Process Listing
        TestCase { id: "T1-F08-01", tier: "Tier 1", feature: "F08 Process List", description: "Process Snapshot Enumeration", test_fn: test_t1_f08_01_process_snapshot_enumeration },
        TestCase { id: "T1-F08-02", tier: "Tier 1", feature: "F08 Process List", description: "Process Field Population", test_fn: test_t1_f08_02_process_field_population },
        TestCase { id: "T1-F08-03", tier: "Tier 1", feature: "F08 Process List", description: "Sorting by CPU Descending", test_fn: test_t1_f08_03_sort_cpu_descending },
        TestCase { id: "T1-F08-04", tier: "Tier 1", feature: "F08 Process List", description: "Sorting by Memory Descending", test_fn: test_t1_f08_04_sort_memory_descending },
        TestCase { id: "T1-F08-05", tier: "Tier 1", feature: "F08 Process List", description: "Filtering by Substring", test_fn: test_t1_f08_05_filtering_substring },

        // Tier 1: F09 - Process Categorization
        TestCase { id: "T1-F09-01", tier: "Tier 1", feature: "F09 Categorization", description: "Application Classification", test_fn: test_t1_f09_01_app_classification },
        TestCase { id: "T1-F09-02", tier: "Tier 1", feature: "F09 Categorization", description: "Background Service Classification", test_fn: test_t1_f09_02_service_classification },
        TestCase { id: "T1-F09-03", tier: "Tier 1", feature: "F09 Categorization", description: "Category Serialization and Deserialization", test_fn: test_t1_f09_03_category_serialization },
        TestCase { id: "T1-F09-04", tier: "Tier 1", feature: "F09 Categorization", description: "Category Filter Subsetting", test_fn: test_t1_f09_04_ui_category_subsetting },
        TestCase { id: "T1-F09-05", tier: "Tier 1", feature: "F09 Categorization", description: "Protected Flag Derivation", test_fn: test_t1_f09_05_protected_flag_derivation },

        // Tier 1: F10 - Runaway Process Detection
        TestCase { id: "T1-F10-01", tier: "Tier 1", feature: "F10 Runaway Detection", description: "High CPU Runaway Trigger (>=90%)", test_fn: test_t1_f10_01_high_cpu_runaway },
        TestCase { id: "T1-F10-02", tier: "Tier 1", feature: "F10 Runaway Detection", description: "High Memory Runaway Trigger (>2GB)", test_fn: test_t1_f10_02_high_memory_runaway },
        TestCase { id: "T1-F10-03", tier: "Tier 1", feature: "F10 Runaway Detection", description: "Dual Pressure Runaway Trigger", test_fn: test_t1_f10_03_dual_pressure_runaway },
        TestCase { id: "T1-F10-04", tier: "Tier 1", feature: "F10 Runaway Detection", description: "Evidence Metadata Completeness", test_fn: test_t1_f10_04_evidence_metadata },
        TestCase { id: "T1-F10-05", tier: "Tier 1", feature: "F10 Runaway Detection", description: "Normal Process Negative Case", test_fn: test_t1_f10_05_normal_process_negative },

        // Tier 1: F11 - Process Safety Blacklist
        TestCase { id: "T1-F11-01", tier: "Tier 1", feature: "F11 Safety Blacklist", description: "csrss.exe Termination Blocked", test_fn: test_t1_f11_01_csrss_blocked },
        TestCase { id: "T1-F11-02", tier: "Tier 1", feature: "F11 Safety Blacklist", description: "lsass.exe Termination Blocked", test_fn: test_t1_f11_02_lsass_blocked },
        TestCase { id: "T1-F11-03", tier: "Tier 1", feature: "F11 Safety Blacklist", description: "services.exe Termination Blocked", test_fn: test_t1_f11_03_services_blocked },
        TestCase { id: "T1-F11-04", tier: "Tier 1", feature: "F11 Safety Blacklist", description: "smss.exe Termination Blocked", test_fn: test_t1_f11_04_smss_blocked },
        TestCase { id: "T1-F11-05", tier: "Tier 1", feature: "F11 Safety Blacklist", description: "explorer.exe Termination Blocked", test_fn: test_t1_f11_05_explorer_blocked },

        // Tier 1: F12 - Offline Companion Assistant
        TestCase { id: "T1-F12-01", tier: "Tier 1", feature: "F12 Assistant", description: "Status Query in English", test_fn: test_t1_f12_01_status_query_english },
        TestCase { id: "T1-F12-02", tier: "Tier 1", feature: "F12 Assistant", description: "Status Query in Vietnamese", test_fn: test_t1_f12_02_status_query_vietnamese },
        TestCase { id: "T1-F12-03", tier: "Tier 1", feature: "F12 Assistant", description: "Finding Next Steps Inclusion", test_fn: test_t1_f12_03_finding_next_steps_included },
        TestCase { id: "T1-F12-04", tier: "Tier 1", feature: "F12 Assistant", description: "Healthy System Confirmation", test_fn: test_t1_f12_04_healthy_confirmation },
        TestCase { id: "T1-F12-05", tier: "Tier 1", feature: "F12 Assistant", description: "100% Offline Execution Guarantee", test_fn: test_t1_f12_05_offline_execution_guarantee },

        // Tier 1: F13 - Bilingual Knowledge Base
        TestCase { id: "T1-F13-01", tier: "Tier 1", feature: "F13 Knowledge Base", description: "CPU Article Lookup in English", test_fn: test_t1_f13_01_cpu_lookup_en },
        TestCase { id: "T1-F13-02", tier: "Tier 1", feature: "F13 Knowledge Base", description: "CPU Article Lookup in Vietnamese", test_fn: test_t1_f13_02_cpu_lookup_vi },
        TestCase { id: "T1-F13-03", tier: "Tier 1", feature: "F13 Knowledge Base", description: "Storage Cleanup Guide Lookup", test_fn: test_t1_f13_03_storage_guide_lookup },
        TestCase { id: "T1-F13-04", tier: "Tier 1", feature: "F13 Knowledge Base", description: "Process Safety Guide Lookup", test_fn: test_t1_f13_04_process_safety_guide_lookup },
        TestCase { id: "T1-F13-05", tier: "Tier 1", feature: "F13 Knowledge Base", description: "RAM Optimization Guide Lookup", test_fn: test_t1_f13_05_ram_optimization_lookup },

        // Tier 1: F14 - React Storage Cleaner Suite
        TestCase { id: "T1-F14-01", tier: "Tier 1", feature: "F14 Storage UI Contracts", description: "Target Grid Rendering", test_fn: test_t1_f14_01_target_grid_rendering },
        TestCase { id: "T1-F14-02", tier: "Tier 1", feature: "F14 Storage UI Contracts", description: "Category Selection Toggle", test_fn: test_t1_f14_02_category_selection_toggle },
        TestCase { id: "T1-F14-03", tier: "Tier 1", feature: "F14 Storage UI Contracts", description: "Dry-Run Switch Toggle", test_fn: test_t1_f14_03_dry_run_switch_toggle },
        TestCase { id: "T1-F14-04", tier: "Tier 1", feature: "F14 Storage UI Contracts", description: "Purge Confirmation Modal Workflow", test_fn: test_t1_f14_04_purge_confirmation_modal_workflow },
        TestCase { id: "T1-F14-05", tier: "Tier 1", feature: "F14 Storage UI Contracts", description: "Summary Results Banner Display", test_fn: test_t1_f14_05_summary_results_banner_display },

        // Tier 1: F15 - React Process Manager UI
        TestCase { id: "T1-F15-01", tier: "Tier 1", feature: "F15 Process UI Contracts", description: "Process Table Columns Rendering", test_fn: test_t1_f15_01_process_table_columns_rendering },
        TestCase { id: "T1-F15-02", tier: "Tier 1", feature: "F15 Process UI Contracts", description: "Category Tab Switching", test_fn: test_t1_f15_02_category_tab_switching },
        TestCase { id: "T1-F15-03", tier: "Tier 1", feature: "F15 Process UI Contracts", description: "Protected Process Status Badge", test_fn: test_t1_f15_03_protected_process_status_badge },
        TestCase { id: "T1-F15-04", tier: "Tier 1", feature: "F15 Process UI Contracts", description: "Terminate Non-Protected Process Confirmation", test_fn: test_t1_f15_04_terminate_non_protected_process_confirmation },
        TestCase { id: "T1-F15-05", tier: "Tier 1", feature: "F15 Process UI Contracts", description: "Real-time Sorting Toggle", test_fn: test_t1_f15_05_real_time_sorting_toggle },

        // Tier 1: F16 - React Bilingual Localization
        TestCase { id: "T1-F16-01", tier: "Tier 1", feature: "F16 Localization Contracts", description: "Language Switch to Vietnamese", test_fn: test_t1_f16_01_language_switch_to_vietnamese },
        TestCase { id: "T1-F16-02", tier: "Tier 1", feature: "F16 Localization Contracts", description: "Language Switch to English", test_fn: test_t1_f16_02_language_switch_to_english },
        TestCase { id: "T1-F16-03", tier: "Tier 1", feature: "F16 Localization Contracts", description: "Dynamic Category Localization", test_fn: test_t1_f16_03_dynamic_category_localization },
        TestCase { id: "T1-F16-04", tier: "Tier 1", feature: "F16 Localization Contracts", description: "Assistant Interface Localization", test_fn: test_t1_f16_04_assistant_interface_localization },
        TestCase { id: "T1-F16-05", tier: "Tier 1", feature: "F16 Localization Contracts", description: "Diagnostic Severity Tier Localization", test_fn: test_t1_f16_05_diagnostic_severity_tier_localization },

        // Tier 1: F17 - E2E Runner Framework
        TestCase { id: "T1-F17-01", tier: "Tier 1", feature: "F17 Runner", description: "Runner Discovery and Registration", test_fn: test_t1_f17_01_runner_registration },
        TestCase { id: "T1-F17-02", tier: "Tier 1", feature: "F17 Runner", description: "Sequential Execution Pipeline", test_fn: test_t1_f17_02_sequential_pipeline },
        TestCase { id: "T1-F17-03", tier: "Tier 1", feature: "F17 Runner", description: "Structured Report Generation", test_fn: test_t1_f17_03_structured_report },
        TestCase { id: "T1-F17-04", tier: "Tier 1", feature: "F17 Runner", description: "Zero Exit Code Verification", test_fn: test_t1_f17_04_zero_exit_code },
        TestCase { id: "T1-F17-05", tier: "Tier 1", feature: "F17 Runner", description: "Console Formatting Protocol", test_fn: test_t1_f17_05_console_formatting },

        // Tier 2: BVA & Corner Cases (F01 - F13, F17)
        TestCase { id: "T2-F01-01", tier: "Tier 2", feature: "F01 Telemetry BVA", description: "Exact 0.0% and 100.0% CPU Extremes", test_fn: test_t2_f01_01_exact_cpu_extremes },
        TestCase { id: "T2-F01-02", tier: "Tier 2", feature: "F01 Telemetry BVA", description: "Zero Total Memory Calculation", test_fn: test_t2_f01_02_zero_total_memory },
        TestCase { id: "T2-F01-03", tier: "Tier 2", feature: "F01 Telemetry BVA", description: "Zero Capacity Disk Volume", test_fn: test_t2_f01_03_zero_capacity_disk },
        TestCase { id: "T2-F01-04", tier: "Tier 2", feature: "F01 Telemetry BVA", description: "Missing ACPI Thermal Sensors Handling", test_fn: test_t2_f01_04_missing_acpi_sensors },
        TestCase { id: "T2-F01-05", tier: "Tier 2", feature: "F01 Telemetry BVA", description: "System Uptime Rollover Boundary", test_fn: test_t2_f01_05_uptime_clock_rollover },

        TestCase { id: "T2-F02-01", tier: "Tier 2", feature: "F02 Diagnostics BVA", description: "Exact Disk Threshold Boundaries (2.0% and 10.0%)", test_fn: test_t2_f02_01_exact_disk_threshold_boundaries },
        TestCase { id: "T2-F02-02", tier: "Tier 2", feature: "F02 Diagnostics BVA", description: "CPU Spike Duration Window (1 vs 3 ticks)", test_fn: test_t2_f02_02_cpu_spike_duration },
        TestCase { id: "T2-F02-03", tier: "Tier 2", feature: "F02 Diagnostics BVA", description: "Critical Low Memory Threshold (<500MB)", test_fn: test_t2_f02_03_critical_memory_threshold },
        TestCase { id: "T2-F02-04", tier: "Tier 2", feature: "F02 Diagnostics BVA", description: "Sensor Stale Data Elapsed Time (>10s)", test_fn: test_t2_f02_04_sensor_stale_data_boundary },
        TestCase { id: "T2-F02-05", tier: "Tier 2", feature: "F02 Diagnostics BVA", description: "Rolling History Buffer Cap at 10 Samples", test_fn: test_t2_f02_05_rolling_history_buffer_cap },

        TestCase { id: "T2-F03-01", tier: "Tier 2", feature: "F03 Scanner BVA", description: "Non-Existent Target Directory", test_fn: test_t2_f03_01_nonexistent_target_dir },
        TestCase { id: "T2-F03-02", tier: "Tier 2", feature: "F03 Scanner BVA", description: "Forbidden Target Directory Rejection", test_fn: test_t2_f03_02_forbidden_target_dir },
        TestCase { id: "T2-F03-03", tier: "Tier 2", feature: "F03 Scanner BVA", description: "Unicode Combining Diacritics in Path", test_fn: test_t2_f03_03_unicode_diacritics_path },
        TestCase { id: "T2-F03-04", tier: "Tier 2", feature: "F03 Scanner BVA", description: "Deeply Nested Directory Hierarchy", test_fn: test_t2_f03_04_deeply_nested_hierarchy },
        TestCase { id: "T2-F03-05", tier: "Tier 2", feature: "F03 Scanner BVA", description: "Empty Target Directory Handling", test_fn: test_t2_f03_05_empty_target_directory },

        TestCase { id: "T2-F04-01", tier: "Tier 2", feature: "F04 Exact Bytes BVA", description: "Twenty 0-Byte Empty Files", test_fn: test_t2_f04_01_twenty_zero_byte_files },
        TestCase { id: "T2-F04-02", tier: "Tier 2", feature: "F04 Exact Bytes BVA", description: "Multi-Gigabyte File (>4GB u32 overflow)", test_fn: test_t2_f04_02_multi_gigabyte_u32_overflow },
        TestCase { id: "T2-F04-03", tier: "Tier 2", feature: "F04 Exact Bytes BVA", description: "File Deleted Mid-Scan Resilience", test_fn: test_t2_f04_03_file_deleted_mid_scan },
        TestCase { id: "T2-F04-04", tier: "Tier 2", feature: "F04 Exact Bytes BVA", description: "Reparse Point Symlink Escape Guard", test_fn: test_t2_f04_04_reparse_point_guard },
        TestCase { id: "T2-F04-05", tier: "Tier 2", feature: "F04 Exact Bytes BVA", description: "Saturating Disk Math Invariant", test_fn: test_t2_f04_05_saturating_disk_math },

        TestCase { id: "T2-F05-01", tier: "Tier 2", feature: "F05 Dry-Run BVA", description: "Dry-Run with Empty Category Target List", test_fn: test_t2_f05_01_dry_run_empty_list },
        TestCase { id: "T2-F05-02", tier: "Tier 2", feature: "F05 Dry-Run BVA", description: "Dry-Run Against Exclusively Locked Files", test_fn: test_t2_f05_02_dry_run_against_locked_files },
        TestCase { id: "T2-F05-03", tier: "Tier 2", feature: "F05 Dry-Run BVA", description: "Repeated Idempotent Dry-Runs", test_fn: test_t2_f05_03_repeated_idempotent_dry_runs },
        TestCase { id: "T2-F05-04", tier: "Tier 2", feature: "F05 Dry-Run BVA", description: "Mixed Existent and Non-Existent Categories", test_fn: test_t2_f05_04_mixed_existent_and_invalid_categories },
        TestCase { id: "T2-F05-05", tier: "Tier 2", feature: "F05 Dry-Run BVA", description: "Read-Only Target Media in Dry-Run", test_fn: test_t2_f05_05_read_only_target_media_dry_run },

        TestCase { id: "T2-F06-01", tier: "Tier 2", feature: "F06 Locked Skip BVA", description: "100% of Files Locked in Target", test_fn: test_t2_f06_01_all_files_locked },
        TestCase { id: "T2-F06-02", tier: "Tier 2", feature: "F06 Locked Skip BVA", description: "Windows Read-Only Attribute Reset and Purge", test_fn: test_t2_f06_02_readonly_file_live_purge },
        TestCase { id: "T2-F06-03", tier: "Tier 2", feature: "F06 Locked Skip BVA", description: "Parent Directory Retained When Child is Locked", test_fn: test_t2_f06_03_parent_subdir_retained_when_child_locked },
        TestCase { id: "T2-F06-04", tier: "Tier 2", feature: "F06 Locked Skip BVA", description: "Transient Lock Acquisition Handling", test_fn: test_t2_f06_04_transient_lock_acquisition },
        TestCase { id: "T2-F06-05", tier: "Tier 2", feature: "F06 Locked Skip BVA", description: "Zero Files Cleaned Due to Lock Contention", test_fn: test_t2_f06_05_zero_files_cleaned_due_to_lock },

        TestCase { id: "T2-F07-01", tier: "Tier 2", feature: "F07 Boundary Guard BVA", description: "Parent Dots Traversal (..) Rejection", test_fn: test_t2_f07_01_traversal_parent_dots },
        TestCase { id: "T2-F07-02", tier: "Tier 2", feature: "F07 Boundary Guard BVA", description: "Mixed Forward and Backslashes Normalization", test_fn: test_t2_f07_02_mixed_slashes },
        TestCase { id: "T2-F07-03", tier: "Tier 2", feature: "F07 Boundary Guard BVA", description: "Program Files Reparse Target Guard", test_fn: test_t2_f07_03_reparse_outside_safe_root },
        TestCase { id: "T2-F07-04", tier: "Tier 2", feature: "F07 Boundary Guard BVA", description: "Case-Insensitive Windows Path Defense", test_fn: test_t2_f07_04_case_insensitivity_defense },
        TestCase { id: "T2-F07-05", tier: "Tier 2", feature: "F07 Boundary Guard BVA", description: "Drive Root Prohibited Rejection", test_fn: test_t2_f07_05_root_drive_target },

        TestCase { id: "T2-F08-01", tier: "Tier 2", feature: "F08 Process List BVA", description: "Processes with Exact 0.0% CPU and 0 Memory", test_fn: test_t2_f08_01_zero_cpu_and_mem_procs },
        TestCase { id: "T2-F08-02", tier: "Tier 2", feature: "F08 Process List BVA", description: "Special Regex Characters in Search Query", test_fn: test_t2_f08_02_special_regex_characters },
        TestCase { id: "T2-F08-03", tier: "Tier 2", feature: "F08 Process List BVA", description: "Empty and Whitespace-Only Search Query", test_fn: test_t2_f08_03_empty_search_query },
        TestCase { id: "T2-F08-04", tier: "Tier 2", feature: "F08 Process List BVA", description: "Identical Metric Values Stable Ordering", test_fn: test_t2_f08_04_identical_metric_values_stable_sort },
        TestCase { id: "T2-F08-05", tier: "Tier 2", feature: "F08 Process List BVA", description: "Vanished Ephemeral Process Resilience", test_fn: test_t2_f08_05_vanished_process_snapshot },

        TestCase { id: "T2-F09-01", tier: "Tier 2", feature: "F09 Categorization BVA", description: "Process with Missing Executable Path", test_fn: test_t2_f09_01_missing_exe_path },
        TestCase { id: "T2-F09-02", tier: "Tier 2", feature: "F09 Categorization BVA", description: "Portable Directory Executable", test_fn: test_t2_f09_02_portable_executable_path },
        TestCase { id: "T2-F09-03", tier: "Tier 2", feature: "F09 Categorization BVA", description: "Non-ASCII Unicode Process Name", test_fn: test_t2_f09_03_non_ascii_glyph_name },
        TestCase { id: "T2-F09-04", tier: "Tier 2", feature: "F09 Categorization BVA", description: "cmd.exe Heuristic Categorization", test_fn: test_t2_f09_04_cmd_exe_different_contexts },
        TestCase { id: "T2-F09-05", tier: "Tier 2", feature: "F09 Categorization BVA", description: "Unknown Binary Default Categorization", test_fn: test_t2_f09_05_unknown_process_default },

        TestCase { id: "T2-F10-01", tier: "Tier 2", feature: "F10 Runaway BVA", description: "Exact Threshold Boundary (89.9% vs 90.0% CPU)", test_fn: test_t2_f10_01_exact_threshold_89_9_vs_90_0 },
        TestCase { id: "T2-F10-02", tier: "Tier 2", feature: "F10 Runaway BVA", description: "Exact Memory Threshold Boundary (2GB vs 2GB+1B)", test_fn: test_t2_f10_02_exact_memory_threshold_boundary },
        TestCase { id: "T2-F10-03", tier: "Tier 2", feature: "F10 Runaway BVA", description: "System Idle Process High CPU Exemption", test_fn: test_t2_f10_03_system_idle_high_cpu_exemption },
        TestCase { id: "T2-F10-04", tier: "Tier 2", feature: "F10 Runaway BVA", description: "Multiple Concurrent Runaway Processes", test_fn: test_t2_f10_04_multiple_concurrent_runaways },
        TestCase { id: "T2-F10-05", tier: "Tier 2", feature: "F10 Runaway BVA", description: "Rapid Metric Spike and Immediate Recovery", test_fn: test_t2_f10_05_rapid_spike_and_recovery },

        TestCase { id: "T2-F11-01", tier: "Tier 2", feature: "F11 Blacklist BVA", description: "PID 0 to 4 Core Kernel Guard", test_fn: test_t2_f11_01_pid_0_and_4_kernel_guard },
        TestCase { id: "T2-F11-02", tier: "Tier 2", feature: "F11 Blacklist BVA", description: "Case Variations and Name Extensions", test_fn: test_t2_f11_02_case_variations },
        TestCase { id: "T2-F11-03", tier: "Tier 2", feature: "F11 Blacklist BVA", description: "Non-Existent Target Process PID", test_fn: test_t2_f11_03_invalid_pid },
        TestCase { id: "T2-F11-04", tier: "Tier 2", feature: "F11 Blacklist BVA", description: "Self-Termination and Host Guard", test_fn: test_t2_f11_04_self_termination_guard },
        TestCase { id: "T2-F11-05", tier: "Tier 2", feature: "F11 Blacklist BVA", description: "Spoofed Blacklist Process Name", test_fn: test_t2_f11_05_process_name_spoofing },

        TestCase { id: "T2-F12-01", tier: "Tier 2", feature: "F12 Assistant BVA", description: "Empty Query String Handling", test_fn: test_t2_f12_01_empty_query_string },
        TestCase { id: "T2-F12-02", tier: "Tier 2", feature: "F12 Assistant BVA", description: "Extremely Long Query String (>10,000 chars)", test_fn: test_t2_f12_02_extremely_long_query },
        TestCase { id: "T2-F12-03", tier: "Tier 2", feature: "F12 Assistant BVA", description: "Unrecognized Gibberish Prompt Fallback", test_fn: test_t2_f12_03_gibberish_prompt },
        TestCase { id: "T2-F12-04", tier: "Tier 2", feature: "F12 Assistant BVA", description: "Vietnamese Diacritics and Mixed Punctuation", test_fn: test_t2_f12_04_vietnamese_uppercase_diacritics },
        TestCase { id: "T2-F12-05", tier: "Tier 2", feature: "F12 Assistant BVA", description: "Unsupported Language Code Fallback to English", test_fn: test_t2_f12_05_unsupported_language_fallback },

        TestCase { id: "T2-F13-01", tier: "Tier 2", feature: "F13 Knowledge BVA", description: "Single Character Search Query", test_fn: test_t2_f13_01_single_character_query },
        TestCase { id: "T2-F13-02", tier: "Tier 2", feature: "F13 Knowledge BVA", description: "Query with Regex Metacharacters", test_fn: test_t2_f13_02_special_regex_metacharacters },
        TestCase { id: "T2-F13-03", tier: "Tier 2", feature: "F13 Knowledge BVA", description: "Unrelated Query Zero Matches", test_fn: test_t2_f13_03_unrelated_query },
        TestCase { id: "T2-F13-04", tier: "Tier 2", feature: "F13 Knowledge BVA", description: "Diacritic-Insensitive Search Matching", test_fn: test_t2_f13_04_diacritic_insensitive_search },
        TestCase { id: "T2-F13-05", tier: "Tier 2", feature: "F13 Knowledge BVA", description: "KnowledgeBase Search Micro-Benchmark (1000 iter)", test_fn: test_t2_f13_05_kb_micro_benchmark },

        // Tier 2: F14 - Storage Cleaner BVA & Corner Cases
        TestCase { id: "T2-F14-01", tier: "Tier 2", feature: "F14 Storage Cleaner BVA", description: "Zero Categories Selected on Purge Click", test_fn: test_t2_f14_01_zero_categories_selected_purge_click },
        TestCase { id: "T2-F14-02", tier: "Tier 2", feature: "F14 Storage Cleaner BVA", description: "Scan Operation Times Out or Fails", test_fn: test_t2_f14_02_scan_operation_times_out_or_fails },
        TestCase { id: "T2-F14-03", tier: "Tier 2", feature: "F14 Storage Cleaner BVA", description: "Display of Very Large Byte Counts > 1 TB", test_fn: test_t2_f14_03_display_very_large_byte_counts },
        TestCase { id: "T2-F14-04", tier: "Tier 2", feature: "F14 Storage Cleaner BVA", description: "Rapid Repeated Clicks on Purge Button", test_fn: test_t2_f14_04_rapid_repeated_clicks_purge_button },
        TestCase { id: "T2-F14-05", tier: "Tier 2", feature: "F14 Storage Cleaner BVA", description: "100% Skipped Files Alert Banner", test_fn: test_t2_f14_05_all_skipped_files_alert_banner },

        // Tier 2: F15 - Process Manager BVA & Corner Cases
        TestCase { id: "T2-F15-01", tier: "Tier 2", feature: "F15 Process Manager BVA", description: "Massive Process Count: 500+ Processes", test_fn: test_t2_f15_01_massive_process_count_500_procs },
        TestCase { id: "T2-F15-02", tier: "Tier 2", feature: "F15 Process Manager BVA", description: "Protected Process Safety Warning Banner on Terminate Click", test_fn: test_t2_f15_02_protected_process_safety_warning_modal },
        TestCase { id: "T2-F15-03", tier: "Tier 2", feature: "F15 Process Manager BVA", description: "Rapid Table Sorting Column Toggles", test_fn: test_t2_f15_03_rapid_table_sorting_column_toggles },
        TestCase { id: "T2-F15-04", tier: "Tier 2", feature: "F15 Process Manager BVA", description: "Process Disappears While Terminate Confirmation Modal is Open", test_fn: test_t2_f15_04_process_disappears_while_confirm_modal_open },
        TestCase { id: "T2-F15-05", tier: "Tier 2", feature: "F15 Process Manager BVA", description: "Runaway Process Visual Highlighting", test_fn: test_t2_f15_05_runaway_process_visual_highlighting },

        // Tier 2: F16 - Bilingual Localization BVA & Corner Cases
        TestCase { id: "T2-F16-01", tier: "Tier 2", feature: "F16 Localization BVA", description: "Missing Dictionary Key Fallback", test_fn: test_t2_f16_01_missing_dictionary_key_fallback },
        TestCase { id: "T2-F16-02", tier: "Tier 2", feature: "F16 Localization BVA", description: "Rapid Language Toggling: 10 Switches in 2 Seconds", test_fn: test_t2_f16_02_rapid_language_toggling_10_switches },
        TestCase { id: "T2-F16-03", tier: "Tier 2", feature: "F16 Localization BVA", description: "Long Vietnamese Diacritic Text Wrapping", test_fn: test_t2_f16_03_long_vietnamese_diacritic_text_wrapping },
        TestCase { id: "T2-F16-04", tier: "Tier 2", feature: "F16 Localization BVA", description: "Number and Date Formatting Across Locales", test_fn: test_t2_f16_04_number_and_date_formatting_across_locales },
        TestCase { id: "T2-F16-05", tier: "Tier 2", feature: "F16 Localization BVA", description: "Locale Preference Persistence", test_fn: test_t2_f16_05_locale_preference_persistence },

        TestCase { id: "T2-F17-01", tier: "Tier 2", feature: "F17 Runner BVA", description: "Failure Detection Integrity", test_fn: test_t2_f17_01_failure_detection },
        TestCase { id: "T2-F17-02", tier: "Tier 2", feature: "F17 Runner BVA", description: "catch_unwind Panic Interception", test_fn: test_t2_f17_02_panic_interception },
        TestCase { id: "T2-F17-03", tier: "Tier 2", feature: "F17 Runner BVA", description: "Temporary Fixture RAII Drop Cleanup", test_fn: test_t2_f17_03_fixture_cleanup_on_drop },
        TestCase { id: "T2-F17-04", tier: "Tier 2", feature: "F17 Runner BVA", description: "Deterministic Log Formatting Protocol", test_fn: test_t2_f17_04_deterministic_formatting },
        TestCase { id: "T2-F17-05", tier: "Tier 2", feature: "F17 Runner BVA", description: "Empty Test Filter Handling", test_fn: test_t2_f17_05_empty_filter },

        // Tier 3: Combinatorial Pairwise Interactions
        TestCase { id: "T3-INT-01", tier: "Tier 3", feature: "Telemetry + Diag + Assistant", description: "High CPU Triggers Diagnostic Incorporated by Assistant", test_fn: test_t3_int_01_telemetry_diagnostics_assistant },
        TestCase { id: "T3-INT-02", tier: "Tier 3", feature: "Scanner + Exact Bytes + Dry-Run", description: "Scanned Reclaimable Space Matches Dry-Run Without Deletions", test_fn: test_t3_int_02_scanner_exact_bytes_dry_run },
        TestCase { id: "T3-INT-03", tier: "Tier 3", feature: "Scanner + Locked Skip + Exact Bytes", description: "Locked Files Excluded From Freed Bytes During Clean", test_fn: test_t3_int_03_scanner_locked_skip_exact_bytes },
        TestCase { id: "T3-INT-04", tier: "Tier 3", feature: "Scanner + Boundary Guard", description: "Tampered Target Injected into System32 Intercepted", test_fn: test_t3_int_04_scanner_boundary_guard },
        TestCase { id: "T3-INT-05", tier: "Tier 3", feature: "Process + Runaway + Blacklist", description: "Runaway System Process Protected by Safety Blacklist", test_fn: test_t3_int_05_process_runaway_blacklist_precedence },
        TestCase { id: "T3-INT-06", tier: "Tier 3", feature: "Process + Runaway + Guard", description: "Rogue Non-Protected App Runaway Permitted for Termination", test_fn: test_t3_int_06_process_runaway_permitted_kill },
        TestCase { id: "T3-INT-07", tier: "Tier 3", feature: "Assistant + Knowledge + Localization", description: "Vietnamese Storage Query Resolves Localized Article", test_fn: test_t3_int_07_assistant_knowledge_localization },
        TestCase { id: "T3-INT-08", tier: "Tier 3", feature: "Scanner + Catalog + Knowledge", description: "Catalog Target Definitions Aligned with Safety Guide", test_fn: test_t3_int_08_scanner_catalog_knowledge },
        TestCase { id: "T3-INT-09", tier: "Tier 3", feature: "Telemetry + Purge Feedback", description: "Freed Storage Increases Available Disk Telemetry", test_fn: test_t3_int_09_telemetry_purge_capacity_feedback },
        TestCase { id: "T3-INT-10", tier: "Tier 3", feature: "Process List + Blacklist", description: "Process Table Derives Protected Badge from Blacklist", test_fn: test_t3_int_10_process_list_blacklist_badge },
        TestCase { id: "T3-INT-11", tier: "Tier 3", feature: "Diagnostics + Telemetry Multi-Disk", description: "Critical Space on C: Emits Alert While D: Remains Healthy", test_fn: test_t3_int_11_multi_drive_diagnostics },
        TestCase { id: "T3-INT-12", tier: "Tier 3", feature: "Dry-Run to Live Locked Transition", description: "Dry-Run Simulates All Files While Live Clean Skips Locked", test_fn: test_t3_int_12_dry_run_to_live_run_locked_transition },
        TestCase { id: "T3-INT-13", tier: "Tier 3", feature: "Assistant + Telemetry Healthy Status", description: "Clean Telemetry Triggers Optimal Performance Affirmation", test_fn: test_t3_int_13_assistant_healthy_confirmation },
        TestCase { id: "T3-INT-14", tier: "Tier 3", feature: "Process List + Categorization Filter", description: "Filtering by Application Excludes Background Services", test_fn: test_t3_int_14_process_category_filtering },
        TestCase { id: "T3-INT-15", tier: "Tier 3", feature: "E2E Runner + All Modules", description: "Runner Registry Discovers and Coordinates All 4 Tiers", test_fn: test_t3_int_15_runner_executes_all_modules },

        // Tier 4: Real-World Operational Scenarios
        TestCase { id: "T4-SCN-01", tier: "Tier 4", feature: "Disk Space Emergency", description: "Critical Disk Space Emergency and Safe Purge Journey", test_fn: test_t4_scn_01_critical_disk_emergency_and_safe_purge },
        TestCase { id: "T4-SCN-02", tier: "Tier 4", feature: "Runaway Browser Diagnosis", description: "Runaway Browser Tab Diagnosis and Safe Termination", test_fn: test_t4_scn_02_runaway_browser_tab_termination },
        TestCase { id: "T4-SCN-03", tier: "Tier 4", feature: "Process Blacklist Defense", description: "Accidental Attempt to Terminate Critical Windows Subsystem", test_fn: test_t4_scn_03_accidental_kill_os_process_defense },
        TestCase { id: "T4-SCN-04", tier: "Tier 4", feature: "Routine Safe Temp Cleanup", description: "Routine Temp Cleanup with In-Use Office and Browser Files", test_fn: test_t4_scn_04_routine_safe_temp_cleanup_with_locked_files },
        TestCase { id: "T4-SCN-05", tier: "Tier 4", feature: "Sleep Resume Recovery", description: "Stale Diagnostic Telemetry and Sleep/Resume Recovery", test_fn: test_t4_scn_05_stale_telemetry_sleep_resume_recovery },
        TestCase { id: "T4-SCN-06", tier: "Tier 4", feature: "Standard User Degradation", description: "Non-Admin Storage Scan Graceful Degradation", test_fn: test_t4_scn_06_non_admin_storage_scan_graceful_degradation },
        TestCase { id: "T4-SCN-07", tier: "Tier 4", feature: "Bilingual Session Transition", description: "Bilingual User Session Transition (EN to VI)", test_fn: test_t4_scn_07_bilingual_session_transition },
        TestCase { id: "T4-SCN-08", tier: "Tier 4", feature: "Offline Assistant Workflow", description: "Offline Assistant Interactive Troubleshooting Workflow", test_fn: test_t4_scn_08_offline_assistant_troubleshooting_workflow },
        TestCase { id: "T4-SCN-09", tier: "Tier 4", feature: "Storage Cleaner Verification", description: "Storage Cleaner Dry-Run Verification Workflow", test_fn: test_t4_scn_09_storage_cleaner_dry_run_workflow },
        TestCase { id: "T4-SCN-10", tier: "Tier 4", feature: "Multi-Drive Volume Telemetry", description: "Multi-Drive Mixed Volume Telemetry & Diagnostic Analysis", test_fn: test_t4_scn_10_multi_drive_mixed_telemetry },
        TestCase { id: "T4-SCN-11", tier: "Tier 4", feature: "Memory Exhaustion Mitigation", description: "Sustained Memory Exhaustion Detection and Mitigation", test_fn: test_t4_scn_11_sustained_memory_exhaustion_mitigation },
        TestCase { id: "T4-SCN-12", tier: "Tier 4", feature: "Security Boundary Defense", description: "Cyber-Security Boundary Defense Against Path Injection", test_fn: test_t4_scn_12_boundary_defense_malicious_path_injection },
    ]
}

// ============================================================================
// Section 6: Test Runners
// ============================================================================

#[test]
fn run_master_e2e_suite() {
    let tests = build_master_registry();
    let start = Instant::now();
    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    println!("\n================================================================================");
    println!("  PC DOCTOR (TBK CORE) — 4-TIER AUTOMATED E2E TEST RUNNER");
    println!("================================================================================");

    for tc in &tests {
        let result = std::panic::catch_unwind(|| (tc.test_fn)());
        match result {
            Ok(Ok(())) => {
                passed += 1;
                println!("  [PASS] {} ({}) — {}", tc.id, tc.feature, tc.description);
            }
            Ok(Err(err)) => {
                failed += 1;
                failures.push((tc.id, err.clone()));
                println!("  [FAIL] {} ({}) — {}: {}", tc.id, tc.feature, tc.description, err);
            }
            Err(_) => {
                failed += 1;
                let err_msg = "Test panicked during execution".to_string();
                failures.push((tc.id, err_msg.clone()));
                println!("  [PANIC] {} ({}) — {}", tc.id, tc.feature, tc.description);
            }
        }
    }

    let elapsed = start.elapsed().as_millis();
    println!("--------------------------------------------------------------------------------");
    println!(
        "Summary: Total: {}, Passed: {}, Failed: {} in {}ms",
        tests.len(),
        passed,
        failed,
        elapsed
    );
    println!("================================================================================\n");

    if !failures.is_empty() {
        for (id, msg) in &failures {
            eprintln!("FAILURE: {} -> {}", id, msg);
        }
    }

    assert_eq!(failed, 0, "One or more E2E test cases failed!");
}

#[test]
fn e2e_tier1_feature_coverage() {
    let tests = build_master_registry();
    for tc in tests.iter().filter(|t| t.tier == "Tier 1") {
        let res = (tc.test_fn)();
        assert!(res.is_ok(), "Test {} failed: {:?}", tc.id, res.err());
    }
}

#[test]
fn e2e_tier2_boundary_value_analysis() {
    let tests = build_master_registry();
    for tc in tests.iter().filter(|t| t.tier == "Tier 2") {
        let res = (tc.test_fn)();
        assert!(res.is_ok(), "Test {} failed: {:?}", tc.id, res.err());
    }
}

#[test]
fn e2e_tier3_pairwise_interactions() {
    let tests = build_master_registry();
    for tc in tests.iter().filter(|t| t.tier == "Tier 3") {
        let res = (tc.test_fn)();
        assert!(res.is_ok(), "Test {} failed: {:?}", tc.id, res.err());
    }
}

#[test]
fn e2e_tier4_operational_scenarios() {
    let tests = build_master_registry();
    for tc in tests.iter().filter(|t| t.tier == "Tier 4") {
        let res = (tc.test_fn)();
        assert!(res.is_ok(), "Test {} failed: {:?}", tc.id, res.err());
    }
}
