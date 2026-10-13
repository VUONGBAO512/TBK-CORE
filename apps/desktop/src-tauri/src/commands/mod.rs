use crate::monitoring::CollectorState;
use pc_doctor_core::knowledge::KnowledgeBase;
use pc_doctor_core::models::{ProcessInfo, SystemSnapshot};
use tauri::State;

use std::sync::Arc;

#[tauri::command]
pub fn get_system_snapshot(state: State<'_, Arc<CollectorState>>) -> Result<SystemSnapshot, String> {
    let mut collector = state
        .collector
        .lock()
        .map_err(|e| format!("Failed to acquire collector lock: {}", e))?;
    Ok(collector.take_snapshot())
}

#[tauri::command]
pub fn get_process_list(state: State<'_, Arc<CollectorState>>) -> Result<Vec<ProcessInfo>, String> {
    let collector = state
        .collector
        .lock()
        .map_err(|e| format!("Failed to acquire collector lock: {}", e))?;
    Ok(collector.get_top_processes(30))
}

#[derive(serde::Serialize)]
pub struct AssistantResponse {
    pub reply: String,
    pub references: Vec<String>,
}

#[tauri::command]
pub fn query_assistant(
    prompt: String,
    lang: String,
    state: State<'_, Arc<CollectorState>>,
) -> Result<AssistantResponse, String> {
    let snapshot = {
        let mut collector = state
            .collector
            .lock()
            .map_err(|e| format!("Failed to lock collector: {}", e))?;
        collector.take_snapshot()
    };

    let kb = KnowledgeBase::new();
    let is_vi = lang.to_lowercase() == "vi";
    let query_lower = prompt.to_lowercase();

    // Check if the user is asking about current status / health
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
        // Query offline knowledge base
        let matches = kb.search(&prompt);
        if let Some(first) = matches.first() {
            references.push(first.id.clone());
            if is_vi {
                reply.push_str(&format!("### {}\n\n{}", first.title_vi, first.content_vi));
            } else {
                reply.push_str(&format!("### {}\n\n{}", first.title_en, first.content_en));
            }
        } else {
            // General guidance
            if is_vi {
                reply.push_str("Tôi là trợ lý **TBK Core** chạy hoàn toàn trên máy tính của bạn.\nBạn có thể hỏi tôi về:\n- Tình trạng máy tính / kiểm tra CPU, RAM, ổ đĩa\n- Cách khắc phục khi máy bị giật lag hoặc đầy ổ\n- Hướng dẫn bảo trì máy tính định kỳ");
            } else {
                reply.push_str("I am your native **TBK Core** assistant operating completely offline.\nYou can ask me about:\n- System health overview & CPU/RAM/Disk stats\n- Troubleshooting performance bottlenecks or low disk space\n- Routine computer optimization practices");
            }
        }
    }

    Ok(AssistantResponse { reply, references })
}
