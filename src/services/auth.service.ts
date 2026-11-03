import { Page } from 'playwright-core';
import { SteelManager } from '../browser/steel.js';
import { Selectors } from '../browser/selectors.js';
import { config } from '../config.js';
import { AuthStatus } from '../types/index.js';

export class AuthService {
  private steelManager: SteelManager;

  constructor() {
    this.steelManager = SteelManager.getInstance();
  }

  /**
   * Dismisses any popup promotional modal or overlay banner.
   */
  public async dismissModals(page: Page): Promise<void> {
    for (const selector of Selectors.closeModalButton) {
      try {
        const closeBtn = page.locator(selector).first();
        if (await closeBtn.isVisible({ timeout: 1000 })) {
          await closeBtn.click();
          await page.waitForTimeout(500);
        }
      } catch {
        // Continue if no modal
      }
    }
  }

  /**
   * Checks if current page/session is authenticated.
   */
  public async checkAuth(): Promise<AuthStatus> {
    const session = await this.steelManager.getSession();
    const page = session.page;

    try {
      // Check current URL or navigate to landing to inspect state
      const currentUrl = page.url();
      if (currentUrl.includes('/my-water-account') || currentUrl.includes('/dashboard')) {
        return {
          isAuthenticated: true,
          userIdentifier: config.airSelangorId || 'Active User',
          steelSessionId: session.sessionId,
          sessionViewerUrl: session.sessionViewerUrl,
        };
      }

      await page.goto(config.portalUrl, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await this.dismissModals(page);

      // Check if user avatar or account button exists
      const userMenu = page.locator(Selectors.userAvatarOrMenu).first();
      const isUserMenuVisible = await userMenu.isVisible({ timeout: 3000 }).catch(() => false);

      if (isUserMenuVisible) {
        return {
          isAuthenticated: true,
          userIdentifier: config.airSelangorId || 'Active User',
          steelSessionId: session.sessionId,
          sessionViewerUrl: session.sessionViewerUrl,
        };
      }

      return {
        isAuthenticated: false,
        steelSessionId: session.sessionId,
        sessionViewerUrl: session.sessionViewerUrl,
        message: 'Session is not authenticated.',
      };
    } catch (err: any) {
      return {
        isAuthenticated: false,
        steelSessionId: session.sessionId,
        sessionViewerUrl: session.sessionViewerUrl,
        message: `Error checking authentication: ${err.message}`,
      };
    }
  }

  /**
   * Logs into Air Selangor using configured or provided credentials.
   */
  public async login(customId?: string, customPassword?: string): Promise<AuthStatus> {
    const session = await this.steelManager.getSession();
    const page = session.page;
    const loginId = customId || config.airSelangorId;
    const password = customPassword || config.airSelangorPassword;

    if (!loginId || !password) {
      return {
        isAuthenticated: false,
        sessionViewerUrl: session.sessionViewerUrl,
        message: 'Missing Air Selangor login credentials. Please set AIR_SELANGOR_ID and AIR_SELANGOR_PASSWORD in .env or pass them directly.',
      };
    }

    try {
      // Check if already authenticated
      const currentStatus = await this.checkAuth();
      if (currentStatus.isAuthenticated) {
        return currentStatus;
      }

      await page.goto(config.portalUrl, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await this.dismissModals(page);

      // Look for Login / Register button
      const loginNav = page.locator(Selectors.loginRegisterNav).first();
      if (await loginNav.isVisible({ timeout: 5000 })) {
        await loginNav.click();
        await page.waitForTimeout(1000);
      }

      // Check if a secondary "Log in" button appears in the modal/dropdown
      const loginBtn = page.locator(Selectors.loginButtonHeader).first();
      if (await loginBtn.isVisible({ timeout: 3000 })) {
        await loginBtn.click();
        await page.waitForTimeout(1500);
      }

      // Fill in credentials
      const idInput = page.locator(Selectors.idInput).first();
      await idInput.waitFor({ state: 'visible', timeout: 10000 });
      await idInput.fill(loginId);

      const passInput = page.locator(Selectors.passwordInput).first();
      await passInput.waitFor({ state: 'visible', timeout: 5000 });
      await passInput.fill(password);

      // Check for CAPTCHA presence before submitting
      const captcha = page.locator(Selectors.captchaFrame).first();
      const hasCaptcha = await captcha.isVisible({ timeout: 2000 }).catch(() => false);

      if (hasCaptcha) {
        return {
          isAuthenticated: false,
          steelSessionId: session.sessionId,
          sessionViewerUrl: session.sessionViewerUrl,
          message: `CAPTCHA challenge detected. Please open the Live Session Viewer to solve the challenge: ${session.sessionViewerUrl}`,
        };
      }

      // Submit login form
      const submitBtn = page.locator(Selectors.loginSubmitButton).first();
      await submitBtn.click();

      // Wait for navigation or challenge
      await page.waitForTimeout(4000);

      // Check for 2FA / OTP or CAPTCHA after submission
      const otpInput = page.locator(Selectors.otpInput).first();
      const hasOtp = await otpInput.isVisible({ timeout: 3000 }).catch(() => false);
      if (hasOtp) {
        return {
          isAuthenticated: false,
          steelSessionId: session.sessionId,
          sessionViewerUrl: session.sessionViewerUrl,
          message: `2FA/OTP required. Please open the Live Session Viewer to enter the verification code: ${session.sessionViewerUrl}`,
        };
      }

      // Check if logged in successfully
      const checkStatus = await this.checkAuth();
      if (checkStatus.isAuthenticated) {
        return {
          ...checkStatus,
          message: 'Login successful.',
        };
      }

      return {
        isAuthenticated: false,
        steelSessionId: session.sessionId,
        sessionViewerUrl: session.sessionViewerUrl,
        message: `Login pending or requires verification. Inspect session at: ${session.sessionViewerUrl}`,
      };
    } catch (err: any) {
      return {
        isAuthenticated: false,
        steelSessionId: session.sessionId,
        sessionViewerUrl: session.sessionViewerUrl,
        message: `Login failed: ${err.message}. Check session viewer: ${session.sessionViewerUrl}`,
      };
    }
  }
}
