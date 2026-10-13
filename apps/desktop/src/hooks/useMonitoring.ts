import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CompanionMood, ProcessInfo, SystemSnapshot } from '../types';

export function useMonitoring(pollingIntervalMs: number = 2000) {
  const [snapshot, setSnapshot] = useState<SystemSnapshot | null>(null);
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const isFetchingRef = useRef<boolean>(false);

  const fetchMetrics = useCallback(async () => {
    if (isFetchingRef.current) return;
    isFetchingRef.current = true;
    try {
      const snap = await invoke<SystemSnapshot>('get_system_snapshot');
      const procs = await invoke<ProcessInfo[]>('get_process_list');
      setSnapshot(snap);
      setProcesses(procs);
      setError(null);
    } catch (err: unknown) {
      console.error('Monitoring fetch error:', err);
      setError(String(err));
    } finally {
      setLoading(false);
      isFetchingRef.current = false;
    }
  }, []);

  useEffect(() => {
    fetchMetrics();
    const interval = setInterval(fetchMetrics, pollingIntervalMs);
    return () => clearInterval(interval);
  }, [fetchMetrics, pollingIntervalMs]);

  // Compute companion mood state based on real findings & load
  let mood: CompanionMood = 'idle';
  if (error || !snapshot) {
    mood = 'offline';
  } else {
    const hasCritical = snapshot.findings.some((f) => f.severity === 'critical');
    const hasHigh = snapshot.findings.some((f) => f.severity === 'high');

    if (hasCritical) {
      mood = 'critical';
    } else if (hasHigh || snapshot.cpu.global_usage_percent >= 85 || snapshot.memory.usage_percent >= 90) {
      mood = 'warning';
    } else {
      mood = 'healthy';
    }
  }

  return {
    snapshot,
    processes,
    loading,
    error,
    mood,
    refetch: fetchMetrics,
  };
}
