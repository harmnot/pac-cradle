/**
 * Launch prices in USD cents (DEC-02, CLAUDE.md §10).
 * Keep every Paddle checkout at $10 or more (CLAUDE.md §9.1).
 */
export const PRICES = {
  singleExport: 1000,
  fivePack: 3500,
  proMonthly: 1200,
  proYearly: 13900,
} as const;
