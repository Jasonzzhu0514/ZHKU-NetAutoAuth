import type { Page } from '@playwright/test';
import type { LogEntry, UpdateState } from '../types';

export async function mockDesktop(page: Page, mode: 'startup-race' | 'save-failure', logs: LogEntry[] = [], holdAuthPage = false) {
  await page.addInitScript(({ mode, logs, holdAuthPage }) => {
    const callbacks = new Map<number, (event: unknown) => void>();
    const listeners = new Map<string, number[]>();
    let next = 1;
    const calls: string[] = [];
    const clipboard: string[] = [];
    Object.defineProperty(navigator, 'clipboard', { configurable: true,
      value: { writeText: async (text: string) => { clipboard.push(text); } } });
    let lateSnapshotUsed = false;
    const account = { username: '', password: '', rememberPassword: false, autoLogin: false };
    let client = {
      revision: 1, busy: mode === 'startup-race', guarding: false, username: '', savedUsername: null,
      lastCheckedMs: null as number | null, authenticationPage: 'http://10.255.255.254/a79.htm',
      rememberPassword: false, autoLogin: false, hasPassword: false, logGeneration: 0, logCount: logs.length,
      network: { state: 'online', region: '测试区域', ip: '192.0.2.1', account: '', mac: '', cvid: '', pvid: '', detail: '' },
      preferences: { closeBehavior: 'exit', noticeAcknowledged: true, autoCheckUpdates: false, guardIntervalSeconds: 30 },
    };
    let updater: UpdateState = { revision: 1, version: '2.1.0', edition: '便携版', message: '尚未检查更新',
      failed: false, rollback: false, busy: false, downloading: false, bytes: 0, prepared: false, release: null };
    if (mode === 'save-failure') updater = { ...updater, prepared: true,
      release: { version: '2.2.0', notes: '', page: 'https://example.com/test-release', size: 10 } };
    const emit = (event: string, payload: unknown) => {
      for (const id of listeners.get(event) || []) callbacks.get(id)?.({ event, id, payload });
    };
    const snapshot = () => structuredClone({ client, updater, trayReady: true });
    Object.assign(window, {
      isTauri: true,
      desktopTest: { calls, clipboard, finishAuthPage: () => {} },
      __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
      __TAURI_INTERNALS__: {
        metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
        transformCallback: (callback: (event: unknown) => void) => { const id = next++; callbacks.set(id, callback); return id; },
        unregisterCallback: (id: number) => callbacks.delete(id),
        invoke: async (command: string, args: Record<string, any> = {}) => {
          if (command === 'plugin:event|listen') {
            listeners.set(args.event, [...(listeners.get(args.event) || []), args.handler]);
            return args.handler;
          }
          if (command === 'plugin:event|unlisten') return;
          if (command === 'plugin:window|is_maximized') return false;
          if (command === 'snapshot') {
            const captured = snapshot();
            if (mode === 'startup-race' && calls.includes('check') && !lateSnapshotUsed) {
              lateSnapshotUsed = true;
              client = { ...client, revision: client.revision + 1, busy: false };
              emit('client-state', client);
            }
            return captured;
          }
          if (command === 'credentials') {
            if (mode === 'startup-race') {
              client = { ...client, revision: 2, busy: false, lastCheckedMs: Date.now() };
              updater = { ...updater, revision: 2, message: '检查已完成' };
              emit('client-state', client);
              emit('updates-state', updater);
              await new Promise((resolve) => setTimeout(resolve, 40));
            }
            return { ...account };
          }
          if (command === 'save_account') {
            Object.assign(account, args.account);
            client = { ...client, revision: client.revision + 1, username: account.username, hasPassword: !!account.password };
            emit('client-state', client);
            return '模拟：凭据目录不可写';
          }
          if (command === 'persist_account') throw new Error('模拟：凭据目录不可写');
          if (command === 'save_preferences') {
            client = { ...client, revision: client.revision + 1, preferences: args.preferences };
            calls.push(`interval:${args.preferences.guardIntervalSeconds}`);
            emit('client-state', client);
            return;
          }
          if (command === 'network_action') {
            calls.push(args.action);
            client = { ...client, revision: client.revision + 1, busy: mode === 'startup-race',
              network: { ...client.network, state: args.action === 'logout' ? 'offline' : 'online' } };
            emit('client-state', client);
            return;
          }
          if (command === 'set_guard') {
            calls.push(args.enabled ? 'guard-on' : 'guard-off');
            client = { ...client, revision: client.revision + 1, guarding: args.enabled };
            emit('client-state', client);
            return;
          }
          if (command === 'plugin:window|close') { emit('request-close', null); return; }
          if (command === 'close_window') { calls.push('close'); return; }
          if (command === 'frontend_ready') { calls.push('ready'); return; }
          if (command === 'install_update') { calls.push('install'); return; }
          if (command === 'open_authentication_page') {
            calls.push('open-auth-page');
            if (holdAuthPage) await new Promise<void>((resolve) => {
              (window as unknown as { desktopTest: { finishAuthPage: () => void } }).desktopTest.finishAuthPage = resolve;
            });
            return;
          }
          if (command === 'open_project_page') { calls.push(`project:${args.target}`); return; }
          if (command === 'get_logs') return logs.filter((log) => log.id > args.after).slice(0, 256);
          return;
        },
      },
    });
  }, { mode, logs, holdAuthPage });
}

export async function desktopCalls(page: Page) {
  return page.evaluate(() => (window as unknown as { desktopTest: { calls: string[] } }).desktopTest.calls);
}
