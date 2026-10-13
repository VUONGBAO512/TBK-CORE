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
  status: 'available' | 'unavailable';
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

export interface ChatMessage {
  id: string;
  sender: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: string;
  findings?: DiagnosticFinding[];
}
