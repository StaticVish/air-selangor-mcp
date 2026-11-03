import { Steel } from 'steel-sdk';
import { chromium, Browser, BrowserContext, Page } from 'playwright-core';
import dns from 'dns/promises';
import { config } from '../config.js';

export interface SteelBrowserSession {
  sessionId: string;
  websocketUrl: string;
  sessionViewerUrl: string;
  browser: Browser;
  context: BrowserContext;
  page: Page;
}

export class SteelManager {
  private static instance: SteelManager;
  private client: Steel;
  private activeSession: SteelBrowserSession | null = null;

  private constructor() {
    this.client = new Steel({
      baseURL: config.steelBaseUrl,
      steelAPIKey: config.steelApiKey,
    });
  }

  public static getInstance(): SteelManager {
    if (!SteelManager.instance) {
      SteelManager.instance = new SteelManager();
    }
    return SteelManager.instance;
  }

  /**
   * Resolves hostname to IP to circumvent Chromium DevTools Host-header security restriction.
   */
  private async getCdpWebsocketUrl(rawUrl: string, sessionId: string): Promise<string> {
    try {
      const urlObj = new URL(rawUrl.startsWith('http') || rawUrl.startsWith('ws') ? rawUrl : `ws://${rawUrl}`);
      const hostname = urlObj.hostname;

      // If it's already an IP or localhost, just attach query param
      if (/^(\d{1,3}\.){3}\d{1,3}$/.test(hostname) || hostname === 'localhost') {
        const port = urlObj.port || '3000';
        return `ws://${hostname}:${port}/?sessionId=${sessionId}`;
      }

      // Resolve DNS to IPv4
      const lookupResult = await dns.lookup(hostname, { family: 4 });
      const ip = lookupResult.address;
      const port = urlObj.port || '3000';
      return `ws://${ip}:${port}/?sessionId=${sessionId}`;
    } catch (err) {
      // Fallback: Default to user's known Tailscale IP 100.77.67.114:3000
      return `ws://100.77.67.114:3000/?sessionId=${sessionId}`;
    }
  }

  /**
   * Gets or initializes an active browser session with Steel.
   */
  public async getSession(options: { forceNew?: boolean } = {}): Promise<SteelBrowserSession> {
    if (this.activeSession && !options.forceNew) {
      try {
        // Quick health ping on the browser context
        if (this.activeSession.browser.isConnected()) {
          return this.activeSession;
        }
      } catch {
        this.activeSession = null;
      }
    }

    // Create session in Steel
    const session = await this.client.sessions.create({
      timeout: config.steelSessionTimeoutMs,
      dimensions: { width: 1920, height: 1080 },
    });

    const cdpUrl = await this.getCdpWebsocketUrl(session.websocketUrl || config.steelBaseUrl, session.id);

    // Connect Playwright over CDP
    const browser = await chromium.connectOverCDP(cdpUrl);
    const context = browser.contexts()[0] || (await browser.newContext());
    const page = context.pages()[0] || (await context.newPage());

    // Fix viewer URL to ensure it points to accessible hostname/port
    let viewerUrl = session.sessionViewerUrl;
    if (!viewerUrl || viewerUrl === 'http://steel.kingfisher-noodlefish.ts.net/') {
      viewerUrl = `${config.steelBaseUrl}/v1/sessions/${session.id}/viewer`;
    }

    this.activeSession = {
      sessionId: session.id,
      websocketUrl: cdpUrl,
      sessionViewerUrl: viewerUrl,
      browser,
      context,
      page,
    };

    return this.activeSession;
  }

  public getViewerUrl(): string | undefined {
    return this.activeSession?.sessionViewerUrl;
  }

  public async closeSession(): Promise<void> {
    if (this.activeSession) {
      try {
        await this.activeSession.browser.close();
      } catch {
        // Ignore close error
      }
      try {
        await this.client.sessions.release(this.activeSession.sessionId);
      } catch {
        // Ignore release error
      }
      this.activeSession = null;
    }
  }
}
