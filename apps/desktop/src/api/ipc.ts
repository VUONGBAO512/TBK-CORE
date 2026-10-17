import { invoke } from '@tauri-apps/api/core';
import type {
  SystemSnapshot,
  ProcessItem,
  TerminateResult,
  StorageScanReport,
  CleanRequest,
  StorageCleanSummary,
  AssistantResponse,
} from '../types';

/**
 * Tauri 2 typed IPC invoke wrappers for PC Doctor (TBK Core).
 */

export async function getSystemSnapshot(): Promise<SystemSnapshot> {
  return await invoke<SystemSnapshot>('get_system_snapshot');
}

export async function getProcessList(): Promise<ProcessItem[]> {
  return await invoke<ProcessItem[]>('get_process_list');
}

export async function terminateProcess(pid: number, processName: string): Promise<TerminateResult> {
  return await invoke<TerminateResult>('terminate_process', {
    pid,
    process_name: processName,
    processName,
  });
}

export async function scanCleanableStorage(): Promise<StorageScanReport> {
  return await invoke<StorageScanReport>('scan_cleanable_storage');
}

export async function cleanStorageTargets(request: CleanRequest): Promise<StorageCleanSummary> {
  return await invoke<StorageCleanSummary>('clean_storage_targets', { request });
}

export async function queryAssistant(prompt: string, lang: string): Promise<AssistantResponse> {
  const raw = await invoke<{ reply: string; references: string[] }>('query_assistant', {
    prompt,
    lang,
  });
  return {
    response: raw.reply,
    reply: raw.reply,
    suggested_actions: [],
    confidence: 1.0,
    sources: raw.references,
    references: raw.references,
  };
}
