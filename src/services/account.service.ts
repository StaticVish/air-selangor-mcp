import { SteelManager } from '../browser/steel.js';
import { AuthService } from './auth.service.js';
import { BillService } from './bill.service.js';
import { config } from '../config.js';
import { WaterAccount, DueBillSummary } from '../types/index.js';

export class AccountService {
  private steelManager: SteelManager;
  private authService: AuthService;
  private billService: BillService;

  constructor() {
    this.steelManager = SteelManager.getInstance();
    this.authService = new AuthService();
    this.billService = new BillService();
  }

  /**
   * Lists all water accounts registered to the profile.
   */
  public async listAccounts(): Promise<WaterAccount[]> {
    const session = await this.steelManager.getSession();
    const page = session.page;

    // Navigate to accounts page
    const targetUrl = `${config.portalUrl}/my-water-account/payment`;
    if (!page.url().includes('/my-water-account')) {
      await page.goto(targetUrl, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await this.authService.dismissModals(page);
      await page.waitForTimeout(2000);
    }

    // Evaluate account cards on the page
    const accountsData = await page.evaluate(() => {
      const results: {
        accountNumber: string;
        premiseAddress: string;
        status: string;
        currentBalance: number;
        isOverdue: boolean;
      }[] = [];

      // Look for elements containing 10-digit account numbers or "Account No"
      const bodyText = document.body.innerText;
      const accountMatches = bodyText.matchAll(/(?:Account\s*No\.?|Akaun)\s*[:\s]*(\d{10})/gi);
      const seenAccounts = new Set<string>();

      for (const match of accountMatches) {
        const accNum = match[1];
        if (!seenAccounts.has(accNum)) {
          seenAccounts.add(accNum);

          // Find surrounding text to parse balance
          const balanceMatch = bodyText.match(new RegExp(`${accNum}[\\s\\S]{1,300}?RM\\s*([\\d,]+\\.\\d{2})`, 'i'));
          let balance = 0;
          if (balanceMatch) {
            balance = parseFloat(balanceMatch[1].replace(/,/g, ''));
          }

          const isOverdue = /Overdue|Tertunggak/i.test(bodyText);

          results.push({
            accountNumber: accNum,
            premiseAddress: 'Registered Premise',
            status: 'Active',
            currentBalance: balance,
            isOverdue,
          });
        }
      }

      return results;
    });

    return accountsData;
  }

  /**
   * Checks all accounts for due or overdue water bills.
   */
  public async checkDueBills(dueWithinDays: number = 7): Promise<{ summary: string; dueBills: DueBillSummary[] }> {
    const accounts = await this.listAccounts();
    const dueBills: DueBillSummary[] = [];
    let totalDue = 0;

    for (const acc of accounts) {
      if (acc.currentBalance > 0) {
        const details = await this.billService.getBillDetails(acc.accountNumber);
        const daysRemaining = details.daysRemaining ?? 0;

        let urgency: 'Overdue' | 'Due Soon' | 'Normal' = 'Normal';
        if (daysRemaining < 0 || acc.isOverdue) {
          urgency = 'Overdue';
        } else if (daysRemaining <= dueWithinDays) {
          urgency = 'Due Soon';
        }

        if (urgency === 'Overdue' || urgency === 'Due Soon' || daysRemaining <= dueWithinDays) {
          dueBills.push({
            accountNumber: acc.accountNumber,
            premiseAddress: acc.premiseAddress,
            amount: acc.currentBalance,
            dueDate: details.dueDate || 'Immediate',
            urgency,
            daysRemaining,
          });
          totalDue += acc.currentBalance;
        }
      }
    }

    const summary =
      dueBills.length > 0
        ? `Found ${dueBills.length} account(s) with bills due within ${dueWithinDays} days. Total amount due: RM ${totalDue.toFixed(2)}.`
        : `All water accounts are up to date. No bills due within ${dueWithinDays} days.`;

    return { summary, dueBills };
  }
}
