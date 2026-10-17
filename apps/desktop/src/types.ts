export type SeverityLevel = 'informational' | 'low' | 'moderate' | 'high' | 'critical';

export type CompanionMood = 'idle' | 'thinking' | 'healthy' | 'warning' | 'critical' | 'offline';

export interface CpuCoreInfo {
  core_id: number;
  usage_percent: number;
  frequency_mhz: number;
}

export interface CpuSnapshot {
  model_name: string;
  logical_cores: number;
  physical_cores: number;
  global_usage_percent: number;
  cores: CpuCoreInfo[];
}

export interface MemorySnapshot {
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  usage_percent: number;
  swap_total_bytes: number;
  swap_used_bytes: number;
}

export interface DiskSnapshot {
  name: string;
  mount_point: string;
  filesystem: string;
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  usage_percent: number;
  is_removable: boolean;
}

export interface OsSnapshot {
  name: string;
  kernel_version: string;
  os_version: string;
  host_name: string;
  uptime_seconds: number;
}

export interface TemperatureSensor {
  label: string;
  celsius: number | null;
  status: string;
}

export interface DiagnosticFinding {
  rule_id: string;
  severity: SeverityLevel;
  title: string;
  description: string;
  evidence: string[];
  observed_at: string;
  confidence: number;
  possible_causes: string[];
  next_steps: string[];
  limitations: string[];
}

export interface SystemSnapshot {
  timestamp: string;
  cpu: CpuSnapshot;
  memory: MemorySnapshot;
  disks: DiskSnapshot[];
  os: OsSnapshot;
  temperatures: TemperatureSensor[];
  findings: DiagnosticFinding[];
}

export interface ProcessInfo {
  pid: number;
  name: string;
  cpu_percent: number;
  memory_bytes: number;
  status: string;
}

export interface ChatMessage {
  id: string;
  sender: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: string;
  references?: string[];
  findings?: DiagnosticFinding[];
}

// ==========================================
// Milestone 3 Interface Contract Models
// ==========================================

export interface CleanableTarget {
  id: string;
  name_en: string;
  name_vi: string;
  description_en: string;
  description_vi: string;
  root_path: string;
  file_count: number;
  total_bytes: number;
  is_safe: boolean;
}

export interface StorageScanReport {
  targets: CleanableTarget[];
  total_reclaimable_bytes: number;
  total_files: number;
  scanned_at: string;
}

export interface CleanRequest {
  target_ids: string[];
  dry_run: boolean;
}

export interface StorageCleanSummary {
  dry_run: boolean;
  bytes_freed: number;
  files_deleted: number;
  files_skipped_locked: number;
  targets_processed: string[];
  errors: string[];
}

export type ProcessCategory =
  | 'System'
  | 'BackgroundService'
  | 'Application'
  | 'BrowserEngine'
  | 'system'
  | 'background_service'
  | 'application'
  | 'browser_engine';

export interface ProcessItem {
  pid: number;
  name: string;
  cpu_percent: number;
  memory_bytes: number;
  status: string;
  category: ProcessCategory;
  is_protected: boolean;
}

export interface TerminateResult {
  pid: number;
  process_name: string;
  success: boolean;
  message: string;
}

export interface AssistantResponse {
  response: string;
  suggested_actions: string[];
  confidence: number;
  sources: string[];
  reply?: string;
  references?: string[];
}
