export const Selectors = {
  // Global & Modals
  closeModalButton: [
    'button[aria-label="Close"]',
    'button[aria-label="close"]',
    '.modal-close',
    'button:has-text("Close")',
    'button:has-text("Tutup")',
  ],

  // Header & Navigation
  loginRegisterNav: 'button:has-text("Login / Register"), a:has-text("Login / Register")',
  loginButtonHeader: 'button:has-text("Log in"), a:has-text("Log in"), button:has-text("Log In")',
  userAvatarOrMenu: '[data-testid="user-profile"], .user-avatar, button:has-text("My Account"), a[href*="/my-water-account"]',

  // Login Form
  idInput: 'input[name="id"], input[name="loginId"], input[placeholder*="ID"], input[type="text"]',
  passwordInput: 'input[name="password"], input[type="password"]',
  loginSubmitButton: 'button[type="submit"], button:has-text("Log In"), button:has-text("Login")',

  // CAPTCHA / 2FA Checks
  captchaFrame: 'iframe[src*="recaptcha"], iframe[src*="challenges.cloudflare.com"], iframe[src*="hcaptcha"]',
  otpInput: 'input[name="otp"], input[placeholder*="OTP"], input[placeholder*="TAC"]',

  // Accounts List & Dashboard
  accountCard: '.account-card, [data-testid="account-item"], div:has-text("Account No")',
  accountNumberText: 'p:has-text("Account No"), span:has-text("Account No"), div:has-text("Account No")',
  accountBalanceText: 'p:has-text("RM"), span:has-text("RM"), div:has-text("RM")',
  
  // Bill Details
  viewBillLink: 'button:has-text("View Bill"), a:has-text("View Bill"), button:has-text("Bill Details")',
  downloadPdfButton: 'button:has-text("Download e-Bill"), a:has-text("Download e-Bill"), button:has-text("Download"), a:has-text("Download")',
};
