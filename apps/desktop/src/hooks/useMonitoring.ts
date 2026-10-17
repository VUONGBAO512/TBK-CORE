import { useState, useEffect, useCallback, useRef } from 'react';
import type { CompanionMood, ProcessItem, SystemSnapshot } from '../types';
import { getSystemSnapshot, getProcessList } from '../api/ipc';

export function useMonitoring(pollingIntervalMs: number = 2000) {
  const [snapshot, setSnapshot] = useState<SystemSnapshot | null>(null);
  const [processes, setProcesses] = useState<ProcessItem[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const isFetchingRef = useRef<boolean>(false);

  const fetchMetrics = useCallback(async () => {
    if (isFetchingRef.current) return;
    isFetchingRef.current = true;
    try {
      const snap = await getSystemSnapshot();
      const procs = await getProcessList();
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
