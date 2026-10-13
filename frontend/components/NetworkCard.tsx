import { Button, Card, Spinner, Tooltip } from '@fluentui/react-components';
import { ArrowSyncRegular } from '@fluentui/react-icons';
import type { Desktop } from '../useDesktop';

export function NetworkCard({ desktop }: { desktop: Desktop }) {
  const state = desktop.view?.client;
  const network = state?.network;
  const online = network?.state === 'online';
  const status = !desktop.ready ? (desktop.error ? '桌面服务未连接' : '正在连接客户端') : online ? '已通过认证'
    : network?.state === 'offline' ? '尚未认证' : '状态待确认';
  const busy = state?.busy || desktop.executing;
  const mac = network?.mac && /^[\da-f]{12}$/i.test(network.mac)
    ? network.mac.match(/.{2}/g)?.join(':').toUpperCase() : network?.mac;
  const fields = [
    ['所在区域', network?.region], ['IPv4 地址', network?.ip], ['在线账号', network?.account],
    ['在线 MAC', mac], ['VLAN', [network?.cvid, network?.pvid].filter(Boolean).join(' / ')],
  ];
  return <Card className="network-card">
    <div className="network-heading">
      <div className="network-title">
        <span className="eyebrow">网络状态</span>
        <Tooltip content={network?.detail || status} relationship="description">
          <h2><span aria-hidden="true" className={`state-dot ${online ? 'online' : ''}`} />{status}</h2>
        </Tooltip>
      </div>
      <Button className="refresh-button" appearance="secondary" icon={busy ? <Spinner size="tiny" /> : <ArrowSyncRegular />}
        aria-disabled={!desktop.ready || busy} onClick={() => { if (desktop.ready && !busy) void desktop.networkAction('check'); }}>
        刷新状态
      </Button>
    </div>
    <div className="network-details">
      {fields.map(([label, value]) => <div className="detail-item" key={label}>
        <span>{label}</span><Tooltip content={value || '暂无数据'} relationship="description">
          <strong>{value || '—'}</strong>
        </Tooltip>
      </div>)}
    </div>
  </Card>;
}
