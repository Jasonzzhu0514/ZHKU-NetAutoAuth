import { Button, Card, Checkbox, Input, Label, Menu, MenuItem, MenuList, MenuPopover, MenuTrigger, Tooltip } from '@fluentui/react-components';
import { ChevronDownRegular, DismissRegular, EyeOffRegular, EyeRegular } from '@fluentui/react-icons';
import type { Account } from '../types';
import type { Desktop } from '../useDesktop';

export function AccountCard({ desktop }: { desktop: Desktop }) {
  const { account, setAccount } = desktop;
  const state = desktop.view?.client;
  const locked = !desktop.ready || !!state?.busy || !!state?.guarding || desktop.executing;
  const change = (patch: Partial<Account>) => setAccount((current) => ({ ...current, ...patch }));
  const configured = !!account.username.trim() && !!account.password;
  const saved = state?.savedUsername;
  return <Card className="account-card">
    <h3>账号配置</h3>
    <div className="account-form">
      <Label htmlFor="account-input">账号</Label>
      <Input id="account-input" value={account.username} placeholder="校园网账号" disabled={locked} maxLength={256}
        autoComplete="off" onChange={(_, data) => change({ username: data.value })}
        contentAfter={<Menu><MenuTrigger disableButtonEnhancement>
          <Button className="input-icon" appearance="transparent" icon={<ChevronDownRegular />} aria-label="已保存账号" disabled={locked} />
        </MenuTrigger><MenuPopover><MenuList>
          {saved ? <div className="saved-account-row"><MenuItem onClick={() => { void desktop.restore(); }}>{saved}</MenuItem>
            <Tooltip content="删除保存的账号" relationship="label"><Button appearance="subtle" icon={<DismissRegular />}
              aria-label="删除保存的账号" onClick={() => { void desktop.forget(); }} /></Tooltip></div>
            : <MenuItem disabled>暂无保存的账号</MenuItem>}
        </MenuList></MenuPopover></Menu>} />
      <Label htmlFor="password-input">密码</Label>
      <Input id="password-input" type={desktop.showPassword ? 'text' : 'password'} value={account.password}
        placeholder="校园网密码" disabled={locked} maxLength={1024} autoComplete="off"
        onChange={(_, data) => change({ password: data.value })}
        contentAfter={<Tooltip content={desktop.showPassword ? '隐藏密码' : '显示密码'} relationship="label">
          <Button className="input-icon" appearance="transparent" icon={desktop.showPassword ? <EyeOffRegular /> : <EyeRegular />}
            aria-label={desktop.showPassword ? '隐藏密码' : '显示密码'} disabled={locked}
            onClick={() => desktop.setShowPassword(!desktop.showPassword)} />
        </Tooltip>} />
      <div className="account-options">
        <Checkbox label="记住密码" checked={account.rememberPassword} disabled={locked}
          onChange={(_, data) => change({ rememberPassword: !!data.checked, autoLogin: data.checked ? account.autoLogin : false })} />
        <Tooltip content="下次启动自动认证并开启守护" relationship="description">
          <Checkbox label="自动登录" checked={account.autoLogin} disabled={locked || (!configured && !account.autoLogin)}
            onChange={(_, data) => change({ autoLogin: !!data.checked, rememberPassword: data.checked ? true : account.rememberPassword })} />
        </Tooltip>
      </div>
    </div>
  </Card>;
}
