import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { Account, ClientState, Preferences, UpdateState, View } from './types';
import { mergeView, newest } from './stateSync';

const emptyAccount: Account = { username: '', password: '', rememberPassword: false, autoLogin: false };
const equalAccount = (a: Account, b: Account) => a.username === b.username && a.password === b.password
  && a.rememberPassword === b.rememberPassword && a.autoLogin === b.autoLogin;

export function useDesktop() {
  const [view, setView] = useState<View | null>(null);
  const [account, setAccount] = useState<Account>(emptyAccount);
  const [ready, setReady] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [error, setError] = useState('');
  const [persistenceWarning, setPersistenceWarning] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [notice, setNotice] = useState(false);
  const viewRef = useRef(view);
  const formRef = useRef(account);
  const syncedRef = useRef(account);
  const timer = useRef<ReturnType<typeof setTimeout>>();
  const saveQueue = useRef(Promise.resolve());
  viewRef.current = view;
  formRef.current = account;

  const flushAccount = useCallback(async () => {
    clearTimeout(timer.current);
    const task = saveQueue.current.catch(() => {}).then(async () => {
      const current = formRef.current;
      if (equalAccount(current, syncedRef.current)) return;
      if (viewRef.current?.client.guarding || viewRef.current?.client.busy) {
        throw new Error('请先停止守护并等待网络请求完成');
      }
      const warning = await invoke<string | null>('save_account', { account: current });
      syncedRef.current = { ...current };
      setPersistenceWarning(warning ? `${warning}。本次仍可认证，重启后可能需要重新配置。` : '');
    });
    saveQueue.current = task;
    await task;
  }, []);

  useEffect(() => {
    if (!ready) return;
    timer.current = setTimeout(() => { void flushAccount().catch((e) => setError(String(e))); }, 300);
    return () => clearTimeout(timer.current);
  }, [account, ready, flushAccount]);

  useEffect(() => {
    let alive = true;
    const subscriptions: UnlistenFn[] = [];
    let pendingClient: ClientState | null = null;
    let pendingUpdater: UpdateState | null = null;
    async function start() {
      if (!isTauri()) { setError('请从桌面客户端启动，以连接认证服务。'); return; }
      const subscribe = async <T,>(name: string, handler: (value: T) => void) => {
        const unsubscribe = await listen<T>(name, (event) => { if (alive) handler(event.payload); });
        if (alive) subscriptions.push(unsubscribe); else unsubscribe();
      };
      await subscribe<ClientState>('client-state', (client) => {
        pendingClient = newest(pendingClient, client);
        setView((old) => old ? { ...old, client: newest(old.client, client) } : old);
      });
      await subscribe<UpdateState>('updates-state', (updater) => {
        pendingUpdater = newest(pendingUpdater, updater);
        setView((old) => old ? { ...old, updater: newest(old.updater, updater) } : old);
      });
      await subscribe('window-hidden', () => setShowPassword(false));
      await subscribe('show-notice', () => setNotice(true));
      await subscribe('request-close', () => {
        void flushAccount().catch((e) => setError(String(e)))
          .then(() => invoke('close_window')).catch((e) => setError(String(e)));
      });
      const [initial, form] = await Promise.all([invoke<View>('snapshot'), invoke<Account>('credentials')]);
      if (!alive) return;
      syncedRef.current = form;
      formRef.current = form;
      setAccount(form);
      const reconciled = { ...initial, client: newest(pendingClient, initial.client),
        updater: newest(pendingUpdater, initial.updater) };
      setView((old) => mergeView(old, reconciled));
      setNotice(!reconciled.client.preferences.noticeAcknowledged);
      setReady(true);
      requestAnimationFrame(() => requestAnimationFrame(() => { void invoke('frontend_ready'); }));
    }
    void start().catch((e) => { if (alive) setError(String(e)); });
    return () => { alive = false; subscriptions.forEach((unsubscribe) => unsubscribe()); clearTimeout(timer.current); };
  }, [flushAccount]);

  const run = useCallback(async (action: () => Promise<unknown>) => {
    if (!ready || executing) return;
    setError('');
    setExecuting(true);
    try { await action(); const next = await invoke<View>('snapshot'); setView((old) => mergeView(old, next)); }
    catch (e) { setError(String(e)); }
    finally { setExecuting(false); }
  }, [ready, executing]);

  const networkAction = (action: 'check' | 'login' | 'logout') => run(async () => {
    if (action === 'login') await flushAccount();
    await invoke('network_action', { action });
    if (action === 'logout') {
      const next = { ...formRef.current, autoLogin: false };
      syncedRef.current = { ...syncedRef.current, autoLogin: false };
      formRef.current = next;
      setAccount(next);
    }
  });
  const guard = (enabled: boolean) => run(async () => {
    if (enabled) await flushAccount();
    await invoke('set_guard', { enabled });
  });
  const restore = () => run(async () => {
    clearTimeout(timer.current);
    await saveQueue.current.catch(() => {});
    const restored = await invoke<Account>('restore_account');
    syncedRef.current = restored;
    formRef.current = restored;
    setAccount(restored);
    setPersistenceWarning('');
    setShowPassword(false);
  });
  const forget = () => run(async () => {
    clearTimeout(timer.current);
    await saveQueue.current.catch(() => {});
    await invoke('delete_account');
    syncedRef.current = emptyAccount;
    formRef.current = emptyAccount;
    setAccount(emptyAccount);
    setPersistenceWarning('');
    setShowPassword(false);
  });
  const preferences = (patch: Partial<Preferences>) => run(async () => {
    const current = viewRef.current?.client.preferences;
    if (current) await invoke('save_preferences', { preferences: { ...current, ...patch } });
  });
  const acknowledge = () => run(async () => {
    const current = viewRef.current?.client.preferences;
    if (current) await invoke('save_preferences', { preferences: { ...current, noticeAcknowledged: true } });
    setNotice(false);
  });
  const install = () => run(async () => {
    await flushAccount();
    await invoke('persist_account');
    setPersistenceWarning('');
    await invoke('install_update');
  });

  return { view, account, setAccount, ready, executing, error, setError, showPassword, setShowPassword,
    persistenceWarning, setPersistenceWarning, notice, acknowledge, networkAction, guard, restore, forget, preferences, install, run, flushAccount };
}

export type Desktop = ReturnType<typeof useDesktop>;
