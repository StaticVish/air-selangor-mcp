import path from 'path';
import fs from 'fs/promises';
import { SteelManager } from '../browser/steel.js';
import { Selectors } from '../browser/selectors.js';
import { config } from '../config.js';
import { BillDetails, DownloadResult } from '../types/index.js';

export class BillService {
  private steelManager: SteelManager;

  constructor() {
    this.steelManager = SteelManager.getInstance();
  }

  /**
   * Fetches detailed billing statement information for an account.
   */
  public async getBillDetails(accountNumber: string): Promise<BillDetails> {
    const session = await this.steelManager.getSession();
    const page = session.page;

    // Navigate to payment or water account page
    const targetUrl = `${config.portalUrl}/my-water-account/payment`;
    if (!page.url().includes('/my-water-account')) {
      await page.goto(targetUrl, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await page.waitForTimeout(2000);
    }

    // Attempt to locate account card or row matching accountNumber
    const accountElem = page.locator(`text=${accountNumber}`).first();
    const isVisible = await accountElem.isVisible({ timeout: 5000 }).catch(() => false);

    let currentBalance = 0;
    let dueDate = '';
    let billDate = '';
    let status = 'Unpaid';
    let consumptionM3 = 0;

    if (isVisible) {
      // Find parent container or row
      const parentContainer = accountElem.locator('xpath=./ancestor::div[contains(@class, "card") or contains(@class, "item") or contains(@class, "box") or count(p) > 2]').first();

      const containerText = await parentContainer.innerText().catch(() => '');
      
      // Extract balance (e.g. RM 45.20 or RM45.20)
      const balanceMatch = containerText.match(/RM\s*([\d,]+\.\d{2})/i);
      if (balanceMatch) {
        currentBalance = parseFloat(balanceMatch[1].replace(/,/g, ''));
      }

      // Extract Due Date (e.g. Due Date: 14/11/2026 or 14 Nov 2026)
      const dueDateMatch = containerText.match(/Due\s*(?:Date)?[:\s]+(\d{1,2}[\/\-]\d{1,2}[\/\-]\d{2,4}|\d{1,2}\s+[A-Za-z]+\s+\d{4})/i);
      if (dueDateMatch) {
        dueDate = dueDateMatch[1];
      }

      // Extract Consumption (e.g. 25 m3 or 25.50 m³)
      const consumptionMatch = containerText.match(/([\d.]+)\s*(?:m3|m³)/i);
      if (consumptionMatch) {
        consumptionM3 = parseFloat(consumptionMatch[1]);
      }
    }

    // Calculate days remaining
    let daysRemaining: number | undefined;
    if (dueDate) {
      const parsedDate = new Date(dueDate);
      if (!isNaN(parsedDate.getTime())) {
        const diffMs = parsedDate.getTime() - Date.now();
        daysRemaining = Math.ceil(diffMs / (1000 * 60 * 60 * 24));
        if (daysRemaining < 0) {
          status = 'Overdue';
        } else if (daysRemaining <= 7) {
          status = 'Due Soon';
        } else {
          status = 'Normal';
        }
      }
    }

    return {
      accountNumber,
      billDate: billDate || new Date().toISOString().split('T')[0],
      dueDate: dueDate || 'See statement',
      previousBalance: 0,
      currentCharges: currentBalance,
      totalOutstanding: currentBalance,
      minimumPayable: Math.min(6.0, currentBalance),
      status,
      daysRemaining,
      consumptionM3,
    };
  }

  /**
   * Downloads the latest e-Bill PDF statement.
   */
  public async downloadBillPdf(accountNumber: string, outputDirectory?: string): Promise<DownloadResult> {
    const session = await this.steelManager.getSession();
    const page = session.page;
    const destDir = outputDirectory || config.downloadsDir;

    await fs.mkdir(destDir, { recursive: true });

    try {
      // Find download button
      const downloadBtn = page.locator(Selectors.downloadPdfButton).first();
      const isVisible = await downloadBtn.isVisible({ timeout: 5000 }).catch(() => false);

      if (!isVisible) {
        return {
          success: false,
          accountNumber,
          error: `Download button not found for account ${accountNumber}. Please verify account selection in viewer: ${session.sessionViewerUrl}`,
        };
      }

      // Intercept file download
      const [download] = await Promise.all([
        page.waitForEvent('download', { timeout: 30000 }),
        downloadBtn.click(),
      ]);

      const originalName = download.suggestedFilename() || `AirSelangor_Bill_${accountNumber}.pdf`;
      const filePath = path.join(destDir, originalName);
      await download.saveAs(filePath);

      const stat = await fs.stat(filePath);

      return {
        success: true,
        accountNumber,
        filePath,
        fileSizeBytes: stat.size,
      };
    } catch (err: any) {
      return {
        success: false,
        accountNumber,
        error: `Failed to download PDF: ${err.message}. Inspect session at: ${session.sessionViewerUrl}`,
      };
    }
  }
}
