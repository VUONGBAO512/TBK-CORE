import { invoke } from '@tauri-apps/api/core';

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

/**
 * Invokes the Tauri command to scan cleanable storage locations.
 */
export async function scanCleanableStorage(): Promise<StorageScanReport> {
  return await invoke<StorageScanReport>('scan_cleanable_storage');
}

/**
 * Invokes the Tauri command to clean or dry-run purge storage targets.
 */
export async function cleanStorageTargets(request: CleanRequest): Promise<StorageCleanSummary> {
  return await invoke<StorageCleanSummary>('clean_storage_targets', { request });
}
