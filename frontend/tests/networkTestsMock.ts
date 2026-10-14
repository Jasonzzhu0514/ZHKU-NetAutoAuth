import type { Page } from '@playwright/test';
import type { WebsiteTests } from '../networkTests';
import { mockDesktop } from './desktopMock';

export async function mockNetworkTests(page: Page) {
  await mockDesktop(page, 'startup-race');
  await page.addInitScript(() => {
    const api = (window as any).__TAURI_INTERNALS__;
    const callbacks = new Map<number, (event: unknown) => void>();
    const listeners: number[] = [];
    const calls: string[] = [];
    let state: WebsiteTests = { revision: 0, busy: false, pending: [], warning: null,
      entries: [ ['百度', 'https://www.baidu.com/'], ['必应', 'https://www.bing.com/'],
        ['Google', 'https://www.google.com/'], ['YouTube', 'https://www.youtube.com/'] ]
        .map(([name, url], index) => ({ target: { id: index + 1, name, url }, result: null })) };
    const originalTransform = api.transformCallback.bind(api);
    api.transformCallback = (fn: (event: unknown) => void) => {
      const id = originalTransform(fn); callbacks.set(id, fn); return id;
    };
    const emit = () => { for (const id of listeners) callbacks.get(id)?.({ event: 'network-test-state', payload: structuredClone(state) }); };
    Object.assign(window, { networkTestsFixture: { calls, snapshot: () => structuredClone(state) } });
    const originalInvoke = api.invoke.bind(api);
    api.invoke = async (command: string, args: any = {}) => {
      if (command === 'plugin:event|listen' && args.event === 'network-test-state') listeners.push(args.handler);
      if (command === 'network_tests_snapshot') return structuredClone(state);
      if (command !== 'network_tests_action') return originalInvoke(command, args);
      calls.push(args.action);
      if (state.busy) throw new Error('请等待当前网站测试完成');
      if (args.action === 'add') {
        let url: URL;
        try { url = new URL(args.url); } catch { throw new Error('请输入完整的 http:// 或 https:// 网址'); }
        if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) throw new Error('网址不合法');
        if (state.entries.some(entry => entry.target.url === url.toString())) throw new Error('此网址已在列表中');
        state = { ...state, revision: state.revision + 1, entries: [...state.entries,
          { target: { id: Math.max(0, ...state.entries.map(entry => entry.target.id)) + 1, name: args.name, url: url.toString() }, result: null }] };
      } else if (args.action === 'remove') {
        state = { ...state, revision: state.revision + 1, entries: state.entries.filter(entry => entry.target.id !== args.id) };
      } else if (args.action === 'test') {
        const ids = state.entries.filter(entry => args.id == null || entry.target.id === args.id).map(entry => entry.target.id);
        state = { ...state, revision: state.revision + 1, busy: true, pending: ids,
          entries: state.entries.map(entry => ids.includes(entry.target.id) ? { ...entry, result: null } : entry) };
        setTimeout(() => {
          state = { ...state, revision: state.revision + 1, busy: false, pending: [], entries: state.entries.map(entry =>
            !ids.includes(entry.target.id) ? entry : { ...entry, result: { testedAtMs: Date.parse('2026-10-14T04:00:00Z'),
              latencyMs: entry.target.id === 3 || entry.target.id === 2 ? null : 28,
              failure: entry.target.id === 3 ? '连接超时' : entry.target.id === 2 ? 'TLS 握手失败，请检查证书或网络' : null } }) };
          emit();
        }, 1500);
      }
      emit();
      return structuredClone(state);
    };
  });
}
