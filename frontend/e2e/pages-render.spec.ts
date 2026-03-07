import { test, expect } from '@playwright/test';

// Smoke tests: every route renders without crashing (no white screen).

test.describe('Page Rendering - Public Routes', () => {
  test('/ (Home) renders landing page for unauthenticated user', async ({ page }) => {
    await page.goto('/');
    const main = page.locator('main');
    await expect(main.getByRole('heading', { name: 'SwarmCrest' })).toBeVisible();
    await expect(main.getByText('Program your bots')).toBeVisible();
  });

  test('/leaderboard renders with tabs', async ({ page }) => {
    await page.goto('/leaderboard');
    await expect(page.getByRole('heading', { name: 'Leaderboards' })).toBeVisible();
    await expect(page.getByRole('button', { name: '1v1 Elo' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'FFA' })).toBeVisible();
    await expect(page.getByRole('button', { name: '2v2 Teams' })).toBeVisible();
  });

  test('/tournaments renders tournament list', async ({ page }) => {
    await page.goto('/tournaments');
    await expect(page.getByRole('heading', { name: 'Tournaments' })).toBeVisible();
  });

  test('/docs renders documentation page', async ({ page }) => {
    await page.goto('/docs');
    await expect(page.getByRole('heading', { name: 'Documentation' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Getting Started' })).toBeVisible();
  });

  test('/about renders about page', async ({ page }) => {
    await page.goto('/about');
    await expect(page.getByRole('heading', { name: 'About SwarmCrest' })).toBeVisible();
    await expect(page.getByText('Florian Wesch')).toBeVisible();
  });

  test('/login renders login form', async ({ page }) => {
    await page.goto('/login');
    await expect(page.getByRole('heading', { name: 'Login' })).toBeVisible();
    await expect(page.locator('label:has-text("Username")')).toBeVisible();
    await expect(page.locator('label:has-text("Password")')).toBeVisible();
    await expect(page.getByRole('button', { name: /Login/ })).toBeVisible();
    await expect(page.locator('main').getByRole('link', { name: 'Register' })).toBeVisible();
  });

  test('/register renders registration form', async ({ page }) => {
    await page.goto('/register');
    await expect(page.getByRole('heading', { name: 'Register' })).toBeVisible();
    await expect(page.locator('label:has-text("Email")')).toBeVisible();
    await expect(page.locator('label:has-text("Password")')).toBeVisible();
    await expect(page.getByRole('button', { name: /Register/ })).toBeVisible();
    await expect(page.locator('main').getByRole('link', { name: 'Login' })).toBeVisible();
  });
});

test.describe('Page Rendering - Auth-Gated Routes', () => {
  test('protected routes redirect to login', async ({ page }) => {
    for (const path of ['/bots', '/editor', '/game', '/challenge', '/teams']) {
      await page.goto(path);
      await expect(page).toHaveURL('/login', { timeout: 5000 });
      await expect(page.locator('nav.app-nav')).toBeVisible();
    }
  });

  test('/api-keys redirects to login when unauthenticated', async ({ page }) => {
    await page.goto('/api-keys');
    await expect(page).toHaveURL('/login', { timeout: 5000 });
    await expect(page.locator('nav.app-nav')).toBeVisible();
  });

  test('/matches/999 renders without crash for nonexistent match', async ({ page }) => {
    await page.goto('/matches/999');
    await page.waitForTimeout(500);
    await expect(page.locator('nav.app-nav')).toBeVisible();
  });

  test('/tournaments/999 renders without crash for nonexistent tournament', async ({ page }) => {
    await page.goto('/tournaments/999');
    await page.waitForTimeout(500);
    await expect(page.locator('nav.app-nav')).toBeVisible();
  });
});
