import { invoke } from '@tauri-apps/api/core';
import { Button, Card, Checkbox, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface, DialogTitle, Divider, ProgressBar, Radio, RadioGroup } from '@fluentui/react-components';
import { ArrowDownloadRegular, ArrowSyncRegular, OpenRegular } from '@fluentui/react-icons';
import { useState } from 'react';
import type { Desktop } from '../useDesktop';

export function SettingsPage({ desktop }: { desktop: Desktop }) {
  const [confirm, setConfirm] = useState(false);
  const preferences = desktop.view?.client.preferences;
  const update = desktop.view?.updater;
  return <>
    <Card className="settings-card"><div className="settings-content">
          <section><h4>关闭窗口时</h4><RadioGroup value={preferences?.closeBehavior || 'exit'}
            onChange={(_, data) => { void desktop.preferences({ closeBehavior: data.value as 'tray' | 'exit' }); }}>
            <Radio value="tray" label="驻留系统托盘，继续运行" disabled={!desktop.view?.trayReady} />
            <Radio value="exit" label="退出程序，停止守护" />
          </RadioGroup></section>
          <Divider />
          <section className="updates-section"><div className="section-heading"><h4>软件更新</h4><span className="version-tag">v{update?.version || '—'} · {update?.edition || '—'}</span></div>
            <Checkbox label="认证联网后自动检查更新" checked={preferences?.autoCheckUpdates || false}
              onChange={(_, data) => { void desktop.preferences({ autoCheckUpdates: !!data.checked }); }} />
            <p className={`update-message ${update?.failed ? 'error-text' : ''}`}>{update?.message || '正在读取更新状态'}</p>
            {update?.downloading && <div className="download-progress"><ProgressBar value={update.bytes} max={update.release?.size || 1} />
              <span>{Math.min(100, Math.floor(update.bytes / (update.release?.size || 1) * 100))}%</span></div>}
            <div className="update-actions"><Button icon={<ArrowSyncRegular />} disabled={!desktop.ready || update?.busy || desktop.executing}
              onClick={() => { void desktop.run(() => invoke('check_updates')); }}>检查更新</Button>
              {update?.downloading ? <Button onClick={() => { void desktop.run(() => invoke('cancel_update')); }}>取消下载</Button>
                : update?.release && (update.prepared
                  ? <Button appearance="primary" onClick={() => setConfirm(true)}>安装并重启</Button>
                  : <Button appearance="primary" icon={<ArrowDownloadRegular />} disabled={update.busy}
                    onClick={() => { void desktop.run(() => invoke('download_update')); }}>下载更新</Button>)}
              <Button appearance="subtle" icon={<OpenRegular />} onClick={() => { void desktop.run(() => invoke('open_release_page')); }}>发布页面</Button>
            </div>
            {update?.release && <div className="release-notes"><strong>v{update.release.version} 更新说明</strong><p>{update.release.notes}</p></div>}
          </section>
    </div></Card>
    <Dialog open={confirm} onOpenChange={(_, data) => setConfirm(data.open)}><DialogSurface><DialogBody>
      <DialogTitle>安装更新并重启？</DialogTitle><DialogContent>升级期间会暂时停止守护，不会注销校园网。现有配置会保留，未记住的密码需要重新输入。</DialogContent>
      <DialogActions><Button onClick={() => setConfirm(false)}>取消</Button><Button appearance="primary" onClick={() => { setConfirm(false); void desktop.install(); }}>确认安装</Button></DialogActions>
    </DialogBody></DialogSurface></Dialog>
  </>;
}

export function FirstUseDialog({ desktop }: { desktop: Desktop }) {
  return <Dialog open={desktop.notice}><DialogSurface><DialogBody>
    <DialogTitle>后台运行与关闭窗口</DialogTitle>
    <DialogContent><p>关闭窗口时，可以驻留系统托盘，已开启的自动守护会继续运行。托盘菜单中的「退出程序」会停止守护并退出客户端。</p>
      <RadioGroup value={desktop.view?.client.preferences.closeBehavior || 'exit'}
        onChange={(_, data) => { void desktop.preferences({ closeBehavior: data.value as 'tray' | 'exit' }); }}>
        <Radio value="tray" label="驻留系统托盘，继续运行" disabled={!desktop.view?.trayReady} />
        <Radio value="exit" label="退出程序，停止守护" />
      </RadioGroup>
    </DialogContent>
    <DialogActions><Button appearance="primary" disabled={desktop.executing} onClick={() => { void desktop.acknowledge(); }}>知道了</Button></DialogActions>
  </DialogBody></DialogSurface></Dialog>;
}
