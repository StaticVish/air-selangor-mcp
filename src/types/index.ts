export interface WaterAccount {
  accountNumber: string;
  premiseAddress: string;
  status: string;
  currentBalance: number;
  isOverdue: boolean;
  dueDate?: string;
}

export interface BillDetails {
  accountNumber: string;
  registeredName?: string;
  billDate?: string;
  dueDate?: string;
  previousBalance: number;
  currentCharges: number;
  totalOutstanding: number;
  minimumPayable: number;
  status: string;
  daysRemaining?: number;
  consumptionM3?: number;
}

export interface DueBillSummary {
  accountNumber: string;
  premiseAddress?: string;
  amount: number;
  dueDate: string;
  urgency: 'Overdue' | 'Due Soon' | 'Normal';
  daysRemaining: number;
}

export interface AuthStatus {
  isAuthenticated: boolean;
  userIdentifier?: string;
  sessionViewerUrl?: string;
  steelSessionId?: string;
  message?: string;
}

export interface DownloadResult {
  success: boolean;
  accountNumber: string;
  filePath?: string;
  fileSizeBytes?: number;
  error?: string;
}
