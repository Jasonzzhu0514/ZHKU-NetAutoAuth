import { expect, test } from '@playwright/test';
import { desktopCalls, mockDesktop } from './desktopMock';

test('打开认证页面期间不显示刷新动画，也不发送状态检查', async ({ page }) => {
  await mockDesktop(page, 'startup-race', [], true);
  await page.goto('/');
  await expect.poll(() => desktopCalls(page)).toContain('ready');
  const lastCheck = page.locator('.detail-item').filter({ has: page.getByText('最近检查', { exact: true }) }).locator('strong');
  const before = await lastCheck.textContent();
  await page.getByRole('button', { name: '认证页面', exact: true }).click();
  await expect.poll(() => desktopCalls(page)).toContain('open-auth-page');
  await expect(page.getByRole('button', { name: '认证页面', exact: true })).toBeDisabled();
  await expect(page.locator('.refresh-button .fui-Spinner')).toHaveCount(0);
  expect(await desktopCalls(page)).not.toContain('check');
  await expect(lastCheck).toHaveText(before!);
  await page.evaluate(() => (window as unknown as { desktopTest: { finishAuthPage: () => void } }).desktopTest.finishAuthPage());
  await expect(page.getByRole('button', { name: '认证页面', exact: true })).toBeEnabled();
  await page.getByRole('button', { name: '刷新状态', exact: true }).click();
  await expect.poll(() => desktopCalls(page)).toContain('check');
});

test('初始化期间收到完成事件，不被旧 busy 快照覆盖', async ({ page }) => {
  await mockDesktop(page, 'startup-race');
  await page.goto('/');
  await expect.poll(() => desktopCalls(page)).toContain('ready');
  await expect(page.getByRole('heading', { name: '已通过认证' })).toBeVisible();
  await expect(page.getByRole('button', { name: '刷新状态' })).toHaveAttribute('aria-disabled', 'false');
  const lastCheck = page.locator('.detail-item').filter({ has: page.getByText('最近检查', { exact: true }) });
  await expect(lastCheck.locator('strong')).toHaveText(/^\d{2}:\d{2}:\d{2}$/);
  await page.getByRole('button', { name: '认证页面', exact: true }).click();
  await expect.poll(() => desktopCalls(page)).toContain('open-auth-page');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await expect(page.getByText('检查已完成', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: '主页', exact: true }).click();
  await page.getByRole('button', { name: '刷新状态' }).click();
  await expect.poll(() => desktopCalls(page)).toContain('check');
  await expect(page.getByRole('button', { name: '刷新状态' })).toHaveAttribute('aria-disabled', 'false');
});

test('更新重启前保存失败，保留运行中的客户端', async ({ page }) => {
  await mockDesktop(page, 'save-failure');
  await page.goto('/');
  await expect.poll(() => desktopCalls(page)).toContain('ready');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await page.getByRole('button', { name: '安装并重启', exact: true }).click();
  await page.getByRole('button', { name: '确认安装', exact: true }).click();
  await expect(page.getByText('Error: 模拟：凭据目录不可写', { exact: true })).toBeVisible();
  expect(await desktopCalls(page)).not.toContain('install');
  expect(await desktopCalls(page)).not.toContain('close');
});

test('自定义守护间隔校验、保存，并在页面切换后保留', async ({ page }) => {
  await mockDesktop(page, 'startup-race');
  await page.goto('/');
  await expect.poll(() => desktopCalls(page)).toContain('ready');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  const interval = page.getByRole('spinbutton', { name: '守护检查间隔' });
  await expect(interval).toHaveValue('30');
  await interval.fill('29');
  await expect(page.getByRole('button', { name: '保存间隔' })).toBeDisabled();
  await expect(page.getByText('请输入 30～3600 的整数')).toBeVisible();
  await interval.fill('90');
  await page.getByRole('button', { name: '保存间隔' }).click();
  await expect.poll(() => desktopCalls(page)).toContain('interval:90');
  await expect(page.getByRole('button', { name: '保存间隔' })).toBeDisabled();
  await page.getByRole('button', { name: '主页', exact: true }).click();
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await expect(interval).toHaveValue('90');
});

test('保存失败提示保留，会话仍能认证、守护、刷新、注销和关闭', async ({ page }) => {
  await mockDesktop(page, 'save-failure');
  await page.goto('/');
  await expect.poll(() => desktopCalls(page)).toContain('ready');
  await page.getByLabel('账号', { exact: true }).fill('test-session-account');
  await page.getByLabel('密码', { exact: true }).fill('test-only-password');
  await page.getByRole('button', { name: '立即认证' }).click();
  await expect.poll(() => desktopCalls(page)).toContain('login');
  await expect(page.getByText(/本次仍可认证/)).toBeVisible();
  await page.getByRole('switch', { name: '自动守护' }).check();
  await expect.poll(() => desktopCalls(page)).toContain('guard-on');
  await page.getByRole('switch', { name: '自动守护' }).uncheck();
  await expect.poll(() => desktopCalls(page)).toContain('guard-off');
  await page.getByRole('button', { name: '刷新状态' }).click();
  await expect.poll(() => desktopCalls(page)).toContain('check');
  await page.getByRole('button', { name: '注销连接' }).click();
  await expect.poll(() => desktopCalls(page)).toContain('logout');
  await page.getByLabel('账号', { exact: true }).fill('test-unsaved-edit');
  await page.getByRole('button', { name: '关闭窗口', exact: true }).click();
  await expect.poll(() => desktopCalls(page)).toContain('close');
});
