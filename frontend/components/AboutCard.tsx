import { invoke } from '@tauri-apps/api/core';
import { Badge, Button, Card, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface, DialogTitle } from '@fluentui/react-components';
import { BugRegular, OpenRegular } from '@fluentui/react-icons';
import { useState } from 'react';
import license from '../../LICENSE?raw';
import type { Desktop } from '../useDesktop';

export function AboutCard({ desktop }: { desktop: Desktop }) {
  const [showLicense, setShowLicense] = useState(false);
  const open = (target: 'home' | 'issues') => {
    void desktop.run(() => invoke('open_project_page', { target }));
  };
  return <><Card className="about-card">
    <h3>关于</h3>
    <div className="about-identity"><strong>ZHKU-NetAutoAuth</strong>
      <Badge appearance="tint" color="brand">v{desktop.view?.updater.version || '—'} · {desktop.view?.updater.edition || '—'}</Badge>
    </div>
    <div className="about-links">
      <Button icon={<OpenRegular />} disabled={!desktop.ready || desktop.executing} onClick={() => open('home')}>项目主页</Button>
      <Button icon={<BugRegular />} disabled={!desktop.ready || desktop.executing} onClick={() => open('issues')}>反馈问题</Button>
    </div>
    <Button className="about-license" appearance="transparent" onClick={() => setShowLicense(true)}>许可证：MIT</Button>
  </Card>
    <Dialog open={showLicense} onOpenChange={(_, data) => setShowLicense(data.open)}>
      <DialogSurface><DialogBody>
        <DialogTitle>MIT 许可证</DialogTitle>
        <DialogContent><pre className="license-text">{license}</pre></DialogContent>
        <DialogActions><Button onClick={() => setShowLicense(false)}>关闭</Button></DialogActions>
      </DialogBody></DialogSurface>
    </Dialog>
  </>;
}
