import { Button, MessageBar, MessageBarActions, MessageBarBody } from '@fluentui/react-components';
import { DismissRegular } from '@fluentui/react-icons';
import { useEffect, useRef, useState } from 'react';
import { useDesktop } from './useDesktop';
import { NetworkCard } from './components/NetworkCard';
import { ControlsCard } from './components/ControlsCard';
import { AccountCard } from './components/AccountCard';
import { LogsCard } from './components/LogsCard';
import { FirstUseDialog, SettingsPage } from './components/SettingsPage';
import { TitleBar } from './components/TitleBar';
import { Sidebar, type Page } from './components/Sidebar';

export function App() {
  const desktop = useDesktop();
  const [page, setPage] = useState<Page>('home');
  const release = desktop.view?.updater.release;
  const rollback = desktop.view?.updater.rollback;
  const rollbackShown = useRef(false);
  useEffect(() => {
    if (rollback && !rollbackShown.current) { rollbackShown.current = true; setPage('settings'); }
  }, [rollback]);
  return <div className="desktop-frame"><TitleBar desktop={desktop} />
    <div className="workspace"><Sidebar page={page} onNavigate={setPage} hasUpdate={!!release} /><main className={`app-shell page-${page}`}>
    {page !== 'home' && <header className="page-header">
      <h1>{page === 'logs' ? '运行日志' : '设置'}</h1>
    </header>}
    {(desktop.error || desktop.persistenceWarning) && <MessageBar intent={desktop.error ? 'error' : 'warning'}><MessageBarBody>{desktop.error || desktop.persistenceWarning}</MessageBarBody><MessageBarActions>
      <Button appearance="transparent" icon={<DismissRegular />} aria-label="关闭提示" onClick={() => {
        if (desktop.error) desktop.setError(''); else desktop.setPersistenceWarning('');
      }} />
    </MessageBarActions></MessageBar>}
    <section className="page-panel home-panel" hidden={page !== 'home'} aria-label="基本连接配置">
      <NetworkCard desktop={desktop} />
      <div className="configuration-grid"><ControlsCard desktop={desktop} /><AccountCard desktop={desktop} /></div>
    </section>
    <section className="page-panel logs-panel" hidden={page !== 'logs'}><LogsCard desktop={desktop} active={page === 'logs'} /></section>
    <section className="page-panel settings-panel" hidden={page !== 'settings'}><SettingsPage desktop={desktop} /></section>
    <FirstUseDialog desktop={desktop} />
  </main></div></div>;
}
