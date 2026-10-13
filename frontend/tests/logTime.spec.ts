import { expect, test } from '@playwright/test';
import { desktopCalls, mockDesktop } from './desktopMock';

test.use({ timezoneId: 'Asia/Shanghai' });

test('日志显示和复制使用记录时刻的本机日期时间，跨日正确', async ({ page }) => {
  await mockDesktop(page, 'startup-race', [{ id: 1, timestampMs: Date.parse('2026-10-13T23:59:58.000Z'),
    level: 'INFO', message: 'raw test response\nsecond line' }]);
  await page.goto('/');
  await expect.poll(() => desktopCalls(page)).toContain('ready');
  await page.getByRole('button', { name: '运行日志', exact: true }).click();
  const time = page.getByRole('log').locator('time').first();
  await expect(time).toHaveAttribute('datetime', '2026-10-13T23:59:58.000Z');
  await expect(time).toHaveAttribute('title', '2026-10-14 07:59:58');
  await expect(time.locator('span').nth(0)).toHaveText('2026-10-14');
  await expect(time.locator('span').nth(1)).toHaveText('07:59:58');
  await page.getByRole('button', { name: '复制当前日志', exact: true }).click();
  await expect.poll(() => page.evaluate(() =>
    (window as unknown as { desktopTest: { clipboard: string[] } }).desktopTest.clipboard))
    .toEqual(['2026-10-14 07:59:58 INFO raw test response\nsecond line']);
});
