import { Badge, Button, Tooltip } from '@fluentui/react-components';
import { DocumentTextRegular, HomeRegular, SettingsRegular } from '@fluentui/react-icons';

export type Page = 'home' | 'logs' | 'settings';

export function Sidebar({ page, onNavigate, hasUpdate }: {
  page: Page; onNavigate: (page: Page) => void; hasUpdate: boolean;
}) {
  const item = (id: Page, label: string, icon: React.ReactElement) =>
    <Tooltip content={label} relationship="description">
      <Button className={`nav-item ${page === id ? 'selected' : ''}`} appearance="subtle"
        icon={icon} aria-label={label} aria-current={page === id ? 'page' : undefined}
        onClick={() => onNavigate(id)}>
        <span className="nav-label">{label}</span>
        {id === 'settings' && hasUpdate && <Badge className="nav-update" size="tiny" appearance="filled" aria-label="有更新" />}
      </Button>
    </Tooltip>;
  return <nav className="sidebar" aria-label="主导航">
    <div className="sidebar-links">{item('home', '主页', <HomeRegular />)}{item('logs', '运行日志', <DocumentTextRegular />)}</div>
    <div className="sidebar-footer">{item('settings', '设置', <SettingsRegular />)}</div>
  </nav>;
}
