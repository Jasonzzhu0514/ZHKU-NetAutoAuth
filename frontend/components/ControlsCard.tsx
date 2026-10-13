import { Button, Card, Divider, Switch, Tooltip } from '@fluentui/react-components';
import { PlugConnectedRegular, PlugDisconnectedRegular } from '@fluentui/react-icons';
import type { Desktop } from '../useDesktop';

export function ControlsCard({ desktop }: { desktop: Desktop }) {
  const state = desktop.view?.client;
  const configured = !!desktop.account.username.trim() && !!desktop.account.password;
  const busy = !desktop.ready || !!state?.busy || desktop.executing;
  return <Card className="control-card">
    <h3>连接控制</h3>
    <div className="connection-buttons">
      <Button appearance="primary" icon={<PlugConnectedRegular />} disabled={!desktop.ready || !configured}
        aria-disabled={busy} onClick={() => { if (!busy) void desktop.networkAction('login'); }}>立即认证</Button>
      <Button className="logout-button" icon={<PlugDisconnectedRegular />} disabled={!desktop.ready || state?.network.state !== 'online'}
        aria-disabled={busy} onClick={() => { if (!busy) void desktop.networkAction('logout'); }}>注销连接</Button>
    </div>
    <Divider />
    <div className="guard-section">
      <div><strong>自动守护</strong><span className="guard-caption">{state?.guarding ? '正在守护网络连接' : '守护已停止'}</span></div>
      <Tooltip content={configured ? '断线时自动重新认证' : '请先填写账号和密码'} relationship="description">
        <Switch aria-label="自动守护" checked={state?.guarding || false} disabled={!desktop.ready || (!configured && !state?.guarding)}
          aria-disabled={desktop.executing} onChange={(_, data) => { if (!desktop.executing) void desktop.guard(data.checked); }} />
      </Tooltip>
    </div>
  </Card>;
}
