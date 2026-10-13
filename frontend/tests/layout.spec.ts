import { expect, test } from '@playwright/test';

for (const viewport of [ { width: 1000, height: 768 }, { width: 920, height: 728 },
  { width: 520, height: 548 }, { width: 1440, height: 868 } ]) {
  test(`Fluent 布局 ${viewport.width} × ${viewport.height}`, async ({ page }, info) => {
    await page.setViewportSize(viewport);
    await page.goto('/');
    await expect(page.locator('.network-card')).toBeVisible();
    await expect(page.getByRole('heading', { name: '校园网自动认证' })).toHaveCount(0);
    const sidebar = await page.getByRole('navigation', { name: '主导航' }).boundingBox();
    const panel = await page.getByRole('main').boundingBox();
    expect(sidebar!.x + sidebar!.width).toBeLessThan(panel!.x);
    expect(Math.abs(sidebar!.y - panel!.y)).toBeLessThan(1);
    expect(Math.abs(sidebar!.height - panel!.height)).toBeLessThan(1);
    const statusCard = await page.locator('.network-card').boundingBox();
    const refresh = await page.getByRole('button', { name: '刷新状态' }).boundingBox();
    const auth = await page.getByRole('button', { name: '认证页面', exact: true }).boundingBox();
    expect(auth!.x + auth!.width).toBeLessThan(refresh!.x);
    expect(Math.abs(auth!.y - refresh!.y)).toBeLessThan(1);
    expect(Math.abs(auth!.width - refresh!.width)).toBeLessThan(1);
    expect(Math.abs(auth!.height - refresh!.height)).toBeLessThan(1);
    expect(refresh!.x + refresh!.width).toBeGreaterThan(statusCard!.x + statusCard!.width - 45);
    await expect(page.getByText('最近检查', { exact: true })).toBeVisible();
    await expect(page.getByText('VLAN', { exact: true })).toHaveCount(0);
    const boxes = await page.locator('.configuration-grid > .fui-Card').evaluateAll((cards) => cards.map((card) => {
      const r = card.getBoundingClientRect(); return { x: r.x, y: r.y, right: r.right, bottom: r.bottom, height: r.height };
    }));
    const content = await page.locator('.app-shell').evaluate((element) => element.clientWidth -
      parseFloat(getComputedStyle(element).paddingLeft) - parseFloat(getComputedStyle(element).paddingRight));
    if (content > 700) {
      expect(Math.abs(boxes[0].height - boxes[1].height)).toBeLessThan(1);
      expect(boxes[0].right).toBeLessThan(boxes[1].x);
    } else {
      expect(boxes[0].bottom).toBeLessThan(boxes[1].y);
    }
    const inputs = await page.locator('.account-form > .fui-Input').evaluateAll((fields) => fields.map((field) => {
      const r = field.getBoundingClientRect(); return { x: r.x, width: r.width };
    }));
    expect(Math.abs(inputs[0].x - inputs[1].x)).toBeLessThan(1);
    expect(Math.abs(inputs[0].width - inputs[1].width)).toBeLessThan(1);
    await expect(page.getByRole('log')).toBeHidden();
    if (viewport.width === 1000) await page.screenshot({ path: info.outputPath('fluent-home.png') });
    await page.getByRole('button', { name: '运行日志', exact: true }).click();
    await expect(page.locator('.configuration-grid')).toBeHidden();
    await expect(page.getByRole('log')).toBeVisible();
    const pane = await page.locator('.logs-card').boundingBox();
    expect(pane!.height).toBeGreaterThan(250);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
    if (viewport.width === 1000) await page.screenshot({ path: info.outputPath('fluent-layout.png') });
  });
}

test('侧边栏切换页面，设置固定在左下角，返回主页恢复连接配置', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: '主页', exact: true })).toHaveAttribute('aria-current', 'page');
  const settings = await page.getByRole('button', { name: '设置', exact: true }).boundingBox();
  expect(settings!.y).toBeGreaterThan(600);
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await expect(page.getByRole('heading', { name: '设置', exact: true })).toBeVisible();
  await expect(page.getByText('软件更新', { exact: true })).toBeVisible();
  await expect(page.locator('.network-card')).toBeHidden();
  await page.getByRole('button', { name: '主页', exact: true }).click();
  await expect(page.locator('.network-card')).toBeVisible();
  await expect(page.getByRole('log')).toBeHidden();
});
