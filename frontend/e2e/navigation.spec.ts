import { test, expect } from '@playwright/test';

test.describe('Navigation & Layout', () => {
  test('navbar renders with public links for unauthenticated user', async ({ page }) => {
    await page.goto('/');
    const nav = page.locator('nav.app-nav');
    await expect(nav).toBeVisible();
    await expect(nav.locator('h1')).toHaveText('SwarmCrest');

    // Public nav links visible
    await expect(nav.getByRole('link', { name: 'Leaderboard' })).toBeVisible();
    await expect(nav.getByRole('link', { name: 'Tournaments' })).toBeVisible();
    await expect(nav.getByRole('link', { name: 'Games' })).toBeVisible();
    await expect(nav.getByRole('link', { name: 'Docs' })).toBeVisible();
    await expect(nav.getByRole('link', { name: 'About' })).toBeVisible();

    // Auth-only nav links NOT visible
    await expect(nav.getByRole('link', { name: 'Bot Library' })).not.toBeVisible();
  });

  test('unauthenticated user sees Login/Register links', async ({ page }) => {
    await page.goto('/');
    const nav = page.locator('nav.app-nav');

    await expect(nav.getByRole('link', { name: 'Login' })).toBeVisible();
    await expect(nav.getByRole('link', { name: 'Register' })).toBeVisible();

    // Authenticated-only links should NOT be visible
    await expect(nav.getByRole('link', { name: 'Challenge' })).not.toBeVisible();
    await expect(nav.getByRole('link', { name: 'My Matches' })).not.toBeVisible();
    await expect(nav.getByRole('link', { name: 'Teams' })).not.toBeVisible();
    await expect(nav.getByRole('link', { name: 'API Keys' })).not.toBeVisible();
  });

  test('landing page shows for unauthenticated user', async ({ page }) => {
    await page.goto('/');
    const main = page.locator('main');
    await expect(main.getByRole('heading', { name: 'SwarmCrest' })).toBeVisible();
    await expect(main.getByText('Program your bots')).toBeVisible();
    await expect(main.getByRole('link', { name: 'Start Competing' })).toBeVisible();
  });

  test('clicking nav links navigates to correct pages', async ({ page }) => {
    await page.goto('/');
    const nav = page.locator('nav.app-nav');

    await nav.getByRole('link', { name: 'Leaderboard' }).click();
    await expect(page).toHaveURL('/leaderboard');
    await expect(page.getByRole('heading', { name: 'Leaderboards' })).toBeVisible();

    await nav.getByRole('link', { name: 'Tournaments' }).click();
    await expect(page).toHaveURL('/tournaments');
    await expect(page.getByRole('heading', { name: 'Tournaments' })).toBeVisible();

    await nav.getByRole('link', { name: 'Docs' }).click();
    await expect(page).toHaveURL('/docs');
    await expect(page.getByRole('heading', { name: 'Getting Started' })).toBeVisible();
  });

  test('protected routes redirect to login when unauthenticated', async ({ page }) => {
    // Test each protected route in separate navigations
    await page.goto('/bots');
    await expect(page).toHaveURL('/login', { timeout: 5000 });

    await page.goto('/editor');
    await expect(page).toHaveURL('/login', { timeout: 5000 });

    await page.goto('/game');
    await expect(page).toHaveURL('/login', { timeout: 5000 });

    await page.goto('/challenge');
    await expect(page).toHaveURL('/login', { timeout: 5000 });

    await page.goto('/my-matches');
    await expect(page).toHaveURL('/login', { timeout: 5000 });

    await page.goto('/teams');
    await expect(page).toHaveURL('/login', { timeout: 5000 });
  });

  test('/api-keys redirects to login when unauthenticated', async ({ page }) => {
    await page.goto('/api-keys');
    await expect(page).toHaveURL('/login', { timeout: 5000 });
  });
});
