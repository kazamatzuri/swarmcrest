import { test, expect } from '@playwright/test';

test.describe('OAuth Callback Page', () => {
  test('/auth/callback with error param shows error message', async ({ page }) => {
    await page.goto('/auth/callback?error=internal');
    await expect(page.getByRole('heading', { name: 'Authentication Error' })).toBeVisible();
    await expect(page.getByText('Authentication failed')).toBeVisible();
    await expect(page.getByRole('link', { name: 'Back to Login' })).toBeVisible();
  });

  test('/auth/callback with custom error param shows that error', async ({ page }) => {
    await page.goto('/auth/callback?error=provider_denied');
    await expect(page.getByText('provider_denied')).toBeVisible();
  });

  test('/auth/callback with no params shows missing token error', async ({ page }) => {
    await page.goto('/auth/callback');
    await expect(page.getByText('No authentication token received')).toBeVisible();
  });

  test('/auth/callback with invalid token shows error after validation', async ({ page }) => {
    await page.goto('/auth/callback?token=invalid-token-abc');
    await expect(page.getByText('Authentication failed')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('SSO Buttons on Login Page', () => {
  test('login page hides password form when password auth is disabled', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: true, google: false, password: false }),
      }),
    );

    await page.goto('/login');
    await expect(page.getByRole('heading', { name: 'Login' })).toBeVisible();
    await expect(page.getByText('Continue with GitHub')).toBeVisible();
    // Password form should NOT be shown
    await expect(page.locator('label:has-text("Username")')).not.toBeVisible();
    await expect(page.locator('label:has-text("Password")')).not.toBeVisible();
    // "or" divider should NOT be shown (no password form below)
    await expect(page.getByText('or', { exact: true })).not.toBeVisible();
  });

  test('login page shows password form and "or" divider when password auth is enabled', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: true, google: false, password: true }),
      }),
    );

    await page.goto('/login');
    await expect(page.getByText('Continue with GitHub')).toBeVisible();
    await expect(page.getByText('or', { exact: true })).toBeVisible();
    await expect(page.locator('label:has-text("Username")')).toBeVisible();
    await expect(page.locator('label:has-text("Password")')).toBeVisible();
  });

  test('login page renders without SSO buttons when no SSO providers configured', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: false, google: false, password: true }),
      }),
    );

    await page.goto('/login');
    await expect(page.getByText('Continue with GitHub')).not.toBeVisible();
    await expect(page.getByText('Continue with Google')).not.toBeVisible();
    // Password form should still work
    await expect(page.locator('label:has-text("Username")')).toBeVisible();
  });

  test('login page shows both SSO buttons when both providers configured', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: true, google: true, password: false }),
      }),
    );

    await page.goto('/login');
    await expect(page.getByText('Continue with GitHub')).toBeVisible();
    await expect(page.getByText('Continue with Google')).toBeVisible();
  });

  test('clicking GitHub SSO button fetches auth URL', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: true, google: false, password: false }),
      }),
    );

    await page.route('**/api/auth/oauth/github', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ url: 'https://github.com/login/oauth/authorize?test=1' }),
      }),
    );

    await page.goto('/login');
    await expect(page.getByText('Continue with GitHub')).toBeVisible();

    const [request] = await Promise.all([
      page.waitForRequest(req => req.url().includes('/api/auth/oauth/github')),
      page.getByText('Continue with GitHub').click(),
    ]);

    expect(request.url()).toContain('/api/auth/oauth/github');
  });
});

test.describe('SSO Buttons on Register Page', () => {
  test('register page shows SSO buttons and hides form when password disabled', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: true, google: true, password: false }),
      }),
    );

    await page.goto('/register');
    await expect(page.getByRole('heading', { name: 'Register' })).toBeVisible();
    await expect(page.getByText('Continue with GitHub')).toBeVisible();
    await expect(page.getByText('Continue with Google')).toBeVisible();
    // Registration form should be hidden
    await expect(page.locator('label:has-text("Username")')).not.toBeVisible();
  });

  test('register page shows both SSO and form when password enabled', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: true, google: true, password: true }),
      }),
    );

    await page.goto('/register');
    await expect(page.getByText('Continue with GitHub')).toBeVisible();
    await expect(page.locator('label:has-text("Username")')).toBeVisible();
  });
});
