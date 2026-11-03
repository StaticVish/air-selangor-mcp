import dotenv from 'dotenv';
import path from 'path';

dotenv.config();

export interface AppConfig {
  steelBaseUrl: string;
  steelApiKey?: string;
  steelSessionTimeoutMs: number;
  airSelangorId?: string;
  airSelangorPassword?: string;
  airSelangorIdType: string;
  downloadsDir: string;
  portalUrl: string;
  logLevel: string;
}

export const config: AppConfig = {
  steelBaseUrl: process.env.STEEL_BASE_URL || 'https://steel.kingfisher-noodlefish.ts.net',
  steelApiKey: process.env.STEEL_API_KEY || undefined,
  steelSessionTimeoutMs: Number(process.env.STEEL_SESSION_TIMEOUT_MS) || 15 * 60 * 1000,
  airSelangorId: process.env.AIR_SELANGOR_ID || '',
  airSelangorPassword: process.env.AIR_SELANGOR_PASSWORD || '',
  airSelangorIdType: (process.env.AIR_SELANGOR_ID_TYPE || 'nric').toLowerCase(),
  downloadsDir: path.resolve(process.cwd(), process.env.DOWNLOADS_DIR || 'downloads'),
  portalUrl: 'https://www.airselangor.com',
  logLevel: process.env.LOG_LEVEL || 'info',
};
