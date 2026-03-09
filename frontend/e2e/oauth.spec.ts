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
    // The page will try to validate via /api/auth/me which will fail
    await expect(page.getByText('Authentication failed')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('SSO Buttons on Login Page', () => {
  test('login page renders without SSO buttons when no providers configured', async ({ page }) => {
    // Mock the providers endpoint to return no providers
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: false, google: false }),
      }),
    );

    await page.goto('/login');
    await expect(page.getByRole('heading', { name: 'Login' })).toBeVisible();
    // SSO buttons should not be present
    await expect(page.getByText('Continue with GitHub')).not.toBeVisible();
    await expect(page.getByText('Continue with Google')).not.toBeVisible();
    // Regular form should still work
    await expect(page.locator('label:has-text("Username")')).toBeVisible();
  });

  test('login page shows GitHub button when GitHub is configured', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: true, google: false }),
      }),
    );

    await page.goto('/login');
    await expect(page.getByText('Continue with GitHub')).toBeVisible();
    await expect(page.getByText('Continue with Google')).not.toBeVisible();
    // "or" divider should be present
    await expect(page.getByText('or', { exact: true })).toBeVisible();
  });

  test('login page shows both SSO buttons when both providers configured', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: true, google: true }),
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
        body: JSON.stringify({ github: true, google: false }),
      }),
    );

    // Mock the GitHub auth start to return a URL (don't actually redirect)
    await page.route('**/api/auth/oauth/github', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ url: 'https://github.com/login/oauth/authorize?test=1' }),
      }),
    );

    await page.goto('/login');
    await expect(page.getByText('Continue with GitHub')).toBeVisible();

    // Intercept navigation to GitHub (prevent actual redirect)
    const [request] = await Promise.all([
      page.waitForRequest(req => req.url().includes('/api/auth/oauth/github')),
      page.getByText('Continue with GitHub').click(),
    ]);

    expect(request.url()).toContain('/api/auth/oauth/github');
  });
});

test.describe('SSO Buttons on Register Page', () => {
  test('register page shows SSO buttons when providers configured', async ({ page }) => {
    await page.route('**/api/auth/providers', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ github: true, google: true }),
      }),
    );

    await page.goto('/register');
    await expect(page.getByRole('heading', { name: 'Register' })).toBeVisible();
    await expect(page.getByText('Continue with GitHub')).toBeVisible();
    await expect(page.getByText('Continue with Google')).toBeVisible();
    // Regular form should still be below
    await expect(page.locator('label:has-text("Username")')).toBeVisible();
  });
});
