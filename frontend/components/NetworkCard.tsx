import { Button, Card, Spinner, Tooltip } from '@fluentui/react-components';
import { ArrowSyncRegular, OpenRegular } from '@fluentui/react-icons';
import { invoke } from '@tauri-apps/api/core';
import type { Desktop } from '../useDesktop';
import { logTime } from '../logTime';

export function NetworkCard({ desktop }: { desktop: Desktop }) {
  const state = desktop.view?.client;
  const network = state?.network;
  const online = network?.state === 'online';
  const status = !desktop.ready ? (desktop.error ? '桌面服务未连接' : '正在连接客户端') : online ? '已通过认证'
    : network?.state === 'offline' ? '尚未认证' : '状态待确认';
  const busy = state?.busy || desktop.executing;
  const mac = network?.mac && /^[\da-f]{12}$/i.test(network.mac)
    ? network.mac.match(/.{2}/g)?.join(':').toUpperCase() : network?.mac;
  const checked = state?.lastCheckedMs != null ? logTime(state.lastCheckedMs) : null;
  const fields = [
    ['所在区域', network?.region], ['IPv4 地址', network?.ip], ['在线账号', network?.account],
    ['在线 MAC', mac], ['最近检查', checked?.clock, checked ? `${checked.text}（检查完成时间，包含失败）` : undefined],
  ];
  return <Card className="network-card">
    <div className="network-heading">
      <div className="network-title">
        <span className="eyebrow">网络状态</span>
        <Tooltip content={network?.detail || status} relationship="description">
          <h2><span aria-hidden="true" className={`state-dot ${online ? 'online' : ''}`} />{status}</h2>
        </Tooltip>
      </div>
      <div className="network-actions">
      <Tooltip content={state?.authenticationPage || '先刷新状态确定所在区域'} relationship="description">
        <Button icon={<OpenRegular />} disabled={!desktop.ready || !state?.authenticationPage || desktop.executing}
          onClick={() => { void desktop.run(() => invoke('open_authentication_page')); }}>认证页面</Button>
      </Tooltip>
      <Button className="refresh-button" appearance="secondary" icon={state?.busy ? <Spinner size="tiny" /> : <ArrowSyncRegular />}
        aria-disabled={!desktop.ready || busy} onClick={() => { if (desktop.ready && !busy) void desktop.networkAction('check'); }}>
        刷新状态
      </Button>
      </div>
    </div>
    <div className="network-details">
      {fields.map(([label, value, description]) => <div className="detail-item" key={label}>
        <span>{label}</span><Tooltip content={description || value || '暂无数据'} relationship="description">
          <strong>{value || '—'}</strong>
        </Tooltip>
      </div>)}
    </div>
  </Card>;
}
