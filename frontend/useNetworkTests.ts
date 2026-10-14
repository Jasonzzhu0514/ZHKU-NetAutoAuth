import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { WebsiteTests } from './networkTests';

export function useNetworkTests(active: boolean) {
  const [snapshot, setSnapshot] = useState<WebsiteTests | null>(null);
  const [ready, setReady] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [error, setError] = useState('');
  const locked = useRef(false);
  const merge = useCallback((next: WebsiteTests) => {
    setSnapshot(old => !old || next.revision >= old.revision ? next : old);
  }, []);

  useEffect(() => {
    if (!active) return;
    let alive = true;
    let unsubscribe: (() => void) | undefined;
    setReady(false);
    const start = async () => {
      if (!isTauri()) throw new Error('请从桌面客户端启动网络测试。');
      const stop = await listen<WebsiteTests>('network-test-state', event => { if (alive) merge(event.payload); });
      if (!alive) { stop(); return; }
      unsubscribe = stop;
      const next = await invoke<WebsiteTests>('network_tests_snapshot');
      if (alive) { merge(next); setReady(true); }
    };
    void start().catch(e => { if (alive) setError(String(e)); });
    return () => { alive = false; unsubscribe?.(); };
  }, [active, merge]);

  const run = async (action: 'test' | 'add' | 'remove', args: { id?: number; name?: string; url?: string } = {}) => {
    if (!ready || locked.current || snapshot?.busy) return false;
    locked.current = true;
    setExecuting(true);
    setError('');
    try {
      merge(await invoke<WebsiteTests>('network_tests_action', { action, ...args }));
      return true;
    } catch (e) { setError(String(e)); return false; }
    finally { locked.current = false; setExecuting(false); }
  };

  return { snapshot, ready, executing, error, setError, run };
}
