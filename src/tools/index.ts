import { z } from 'zod';
import { AuthService } from '../services/auth.service.js';
import { AccountService } from '../services/account.service.js';
import { BillService } from '../services/bill.service.js';
import { SteelManager } from '../browser/steel.js';

export interface ToolDefinition {
  name: string;
  description: string;
  inputSchema: {
    type: string;
    properties: Record<string, any>;
    required?: string[];
  };
  handler: (args: any) => Promise<any>;
}

export function registerTools(): ToolDefinition[] {
  const authService = new AuthService();
  const accountService = new AccountService();
  const billService = new BillService();
  const steelManager = SteelManager.getInstance();

  return [
    {
      name: 'check_session',
      description: 'Checks whether the current Steel browser session is authenticated on Air Selangor.',
      inputSchema: {
        type: 'object',
        properties: {},
      },
      handler: async () => {
        return await authService.checkAuth();
      },
    },
    {
      name: 'login',
      description: 'Logs into the Air Selangor portal. If visual verification (CAPTCHA) or 2FA is required, returns the Steel Live Viewer URL.',
      inputSchema: {
        type: 'object',
        properties: {
          id: {
            type: 'string',
            description: 'Optional Air Selangor login ID (NRIC, Passport, or SSM). Defaults to AIR_SELANGOR_ID from environment.',
          },
          password: {
            type: 'string',
            description: 'Optional Air Selangor login password. Defaults to AIR_SELANGOR_PASSWORD from environment.',
          },
        },
      },
      handler: async (args: { id?: string; password?: string }) => {
        return await authService.login(args.id, args.password);
      },
    },
    {
      name: 'list_accounts',
      description: 'Retrieves all water accounts registered to the profile along with their current outstanding balances.',
      inputSchema: {
        type: 'object',
        properties: {},
      },
      handler: async () => {
        const accounts = await accountService.listAccounts();
        return {
          count: accounts.length,
          accounts,
          sessionViewerUrl: steelManager.getViewerUrl(),
        };
      },
    },
    {
      name: 'get_bill_details',
      description: 'Fetches detailed billing information for a specific Air Selangor water account.',
      inputSchema: {
        type: 'object',
        properties: {
          accountNumber: {
            type: 'string',
            description: 'The 10-digit Air Selangor water account number (e.g. 1234567890).',
          },
        },
        required: ['accountNumber'],
      },
      handler: async (args: { accountNumber: string }) => {
        const details = await billService.getBillDetails(args.accountNumber);
        return {
          ...details,
          sessionViewerUrl: steelManager.getViewerUrl(),
        };
      },
    },
    {
      name: 'check_due_bills',
      description: 'Scans all water accounts and highlights bills that are overdue or due within a specified number of days.',
      inputSchema: {
        type: 'object',
        properties: {
          dueWithinDays: {
            type: 'number',
            description: 'Filter threshold in days (default: 7).',
          },
        },
      },
      handler: async (args: { dueWithinDays?: number }) => {
        const result = await accountService.checkDueBills(args.dueWithinDays ?? 7);
        return {
          ...result,
          sessionViewerUrl: steelManager.getViewerUrl(),
        };
      },
    },
    {
      name: 'download_bill_pdf',
      description: 'Downloads the official e-Bill PDF statement for a water account and saves it to the local filesystem.',
      inputSchema: {
        type: 'object',
        properties: {
          accountNumber: {
            type: 'string',
            description: 'The 10-digit Air Selangor water account number.',
          },
          outputDirectory: {
            type: 'string',
            description: 'Optional directory path where the PDF will be saved (defaults to ./downloads).',
          },
        },
        required: ['accountNumber'],
      },
      handler: async (args: { accountNumber: string; outputDirectory?: string }) => {
        return await billService.downloadBillPdf(args.accountNumber, args.outputDirectory);
      },
    },
    {
      name: 'get_session_viewer',
      description: 'Returns the Steel Live Interactive Session Viewer URL so you can inspect or interact with the remote browser directly on your Tailscale network.',
      inputSchema: {
        type: 'object',
        properties: {},
      },
      handler: async () => {
        const session = await steelManager.getSession();
        return {
          steelSessionId: session.sessionId,
          sessionViewerUrl: session.sessionViewerUrl,
          instruction: 'Open this URL in any browser on your Tailscale network to view or interact with the browser session.',
        };
      },
    },
  ];
}
