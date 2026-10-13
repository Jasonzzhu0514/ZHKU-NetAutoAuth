import { invoke } from '@tauri-apps/api/core';
import { Button, Card, Checkbox, Tooltip } from '@fluentui/react-components';
import { CopyRegular, DeleteRegular, DocumentTextRegular } from '@fluentui/react-icons';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { Desktop } from '../useDesktop';
import type { LogEntry } from '../types';
import { logTime } from '../logTime';

export function LogsCard({ desktop, active }: { desktop: Desktop; active: boolean }) {
  const [errorsOnly, setErrorsOnly] = useState(false);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const generation = desktop.view?.client.logGeneration;
  const count = desktop.view?.client.logCount;
  const lastId = useRef(0);
  const scroll = useRef<HTMLDivElement>(null);
  const tail = useRef(true);
  useEffect(() => { lastId.current = 0; setLogs([]); }, [generation]);
  useEffect(() => {
    if (!active || !desktop.ready) return;
    let alive = true;
    async function load() {
      while (alive) {
        const batch = await invoke<LogEntry[]>('get_logs', { after: lastId.current });
        if (!alive || !batch.length) break;
        lastId.current = batch[batch.length - 1].id;
        setLogs((old) => [...old, ...batch]);
        if (batch.length < 256) break;
      }
    }
    void load().catch((error) => { if (alive) desktop.setError(String(error)); });
    return () => { alive = false; };
  }, [active, desktop.ready, generation, count]);
  const visible = useMemo(() => errorsOnly ? logs.filter((log) => log.level === 'ERROR') : logs, [logs, errorsOnly]);
  const virtual = useVirtualizer({ count: visible.length, getScrollElement: () => scroll.current,
    estimateSize: () => 52, overscan: 6, getItemKey: (index) => visible[index].id });
  useEffect(() => { if (active) virtual.measure(); }, [active]);
  useEffect(() => { if (active && tail.current && visible.length) virtual.scrollToIndex(visible.length - 1, { align: 'end' }); }, [active, visible.length]);
  const copy = () => desktop.run(async () => {
    await navigator.clipboard.writeText(visible.map((log) => `${logTime(log.timestampMs).text} ${log.level} ${log.message}`).join('\n'));
  });
  return <Card className="logs-card expanded">
    <div className="logs-toolbar">
      <span className="logs-caption">原始日志</span>
      <div className="log-actions"><Checkbox label="仅看异常" checked={errorsOnly} onChange={(_, data) => setErrorsOnly(!!data.checked)} />
        <Tooltip content="复制当前日志" relationship="label"><Button appearance="subtle" icon={<CopyRegular />} aria-label="复制当前日志" disabled={!visible.length} onClick={() => { void copy(); }} /></Tooltip>
        <Button appearance="subtle" icon={<DeleteRegular />} disabled={!desktop.ready || !count} onClick={() => { void desktop.run(() => invoke('clear_logs')); }}>清空</Button>
      </div>
    </div>
    <div className="log-scroll" ref={scroll} role="log" aria-label="运行日志" onScroll={() => {
      const pane = scroll.current; if (pane) tail.current = pane.scrollHeight - pane.scrollTop - pane.clientHeight < 48;
    }}>
      {!visible.length ? <div className="logs-empty"><DocumentTextRegular /><span>{errorsOnly ? '暂无异常日志' : '暂无运行日志'}</span></div>
        : <div className="virtual-log-list" style={{ height: virtual.getTotalSize() }}>
          {virtual.getVirtualItems().map((row) => {
            const log = visible[row.index];
            const time = logTime(log.timestampMs);
            const success = log.level === 'INFO' && /^(认证成功|已通过认证)/.test(log.message);
            return <div key={row.key} data-index={row.index} ref={virtual.measureElement}
              className={`log-row ${log.level === 'ERROR' ? 'error' : success ? 'success' : ''}`} style={{ transform: `translateY(${row.start}px)` }}>
              <time dateTime={time.iso} title={time.text}><span>{time.date}</span><span>{time.clock}</span></time><span className="log-level">{log.level}</span><span className="log-message">{log.message}</span>
            </div>;
          })}
        </div>}
    </div>
  </Card>;
}
