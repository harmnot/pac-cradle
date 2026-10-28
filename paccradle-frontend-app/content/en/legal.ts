/**
 * Legal pages (English). DRAFT text written for Paddle domain approval (CLAUDE.md §9.1):
 * have it reviewed before launch. It is not legal advice.
 */

/** One titled part of a legal document. */
export interface LegalSection {
  readonly heading: string;
  readonly paragraphs: readonly string[];
}

/** A full legal document. */
export interface LegalDocument {
  readonly title: string;
  readonly sections: readonly LegalSection[];
}

/** Operator details filled into every document. */
export interface LegalContext {
  readonly siteName: string;
  readonly legalName: string;
  readonly contactEmail: string;
}

/** Date shown as "Last updated" on all three documents. */
export const LEGAL_LAST_UPDATED = "2026-10-27";

/** Paddle's standard merchant-of-record wording. */
const PADDLE_MOR =
  "Our order process is conducted by our online reseller Paddle.com. Paddle.com is the Merchant of Record for all our orders. Paddle provides all customer service inquiries and handles returns.";

/** Terms & Conditions. */
export function termsDocument({ siteName, legalName, contactEmail }: LegalContext): LegalDocument {
  return {
    title: "Terms & Conditions",
    sections: [
      {
        heading: "Who we are",
        paragraphs: [
          `${siteName} is operated by ${legalName}, a sole proprietor ("we", "us"). You can reach us at ${contactEmail}.`,
          `By using ${siteName} you agree to these terms.`,
        ],
      },
      {
        heading: "The service",
        paragraphs: [
          `${siteName} measures products from photos or sizes you enter and generates design files (dielines) for box inserts and boxes. Designing and previewing is free. Production files are paid.`,
        ],
      },
      {
        heading: "Orders and payment",
        paragraphs: [
          PADDLE_MOR,
          "Prices are shown in US dollars on our pricing page. Paddle adds sales tax or VAT where it applies.",
          "Pro subscriptions renew automatically until you cancel. You can cancel at any time; you keep Pro until the end of the period you paid for.",
          "Export credits never expire. A design you unlock can be downloaded again at any time and edited for 30 days.",
        ],
      },
      {
        heading: "Design files are design aids",
        paragraphs: [
          "Files are generated from your photos and settings. Measurements taken from photos can be wrong.",
          "Always print and test a sample before you order production, and confirm materials and tolerances with your printer.",
          "We do not claim that any insert or box protects products in shipping. You are responsible for checking that a design fits and suits your product.",
        ],
      },
      {
        heading: "Your content",
        paragraphs: [
          "You keep all rights to the photos and designs you create. You allow us to store and process them only to provide the service.",
          "Files you buy are licensed to you for making packaging. You may not resell or redistribute the files themselves as templates.",
        ],
      },
      {
        heading: "Acceptable use",
        paragraphs: [
          "Don't upload content you have no right to use, try to break or overload the service, or access other people's data.",
          "We may suspend accounts that break these terms.",
        ],
      },
      {
        heading: "Liability",
        paragraphs: [
          "The service is provided as is. To the extent the law allows, we are not liable for indirect losses, and our total liability is limited to the amount you paid us in the 12 months before the claim.",
        ],
      },
      {
        heading: "Changes",
        paragraphs: [
          "We may update these terms. If a change is important, we will tell you by email or on the site before it takes effect.",
        ],
      },
    ],
  };
}

/** Refund Policy. */
export function refundDocument({ siteName, contactEmail }: LegalContext): LegalDocument {
  return {
    title: "Refund Policy",
    sections: [
      {
        heading: "Who handles refunds",
        paragraphs: [PADDLE_MOR],
      },
      {
        heading: "Exports and credit packs",
        paragraphs: [
          `If files you bought from ${siteName} don't work for you, ask for a refund within 14 days of purchase and you will get your money back in full.`,
          "For a 5-pack, ask within 14 days of purchase.",
        ],
      },
      {
        heading: "Pro subscriptions",
        paragraphs: [
          "You can cancel at any time and keep Pro until the end of the paid period.",
          "If you ask within 14 days of a charge, we refund that charge.",
        ],
      },
      {
        heading: "How to ask",
        paragraphs: [
          `Reply to your Paddle receipt, visit paddle.net, or email us at ${contactEmail} with your order number.`,
        ],
      },
    ],
  };
}

/** Privacy Policy. */
export function privacyDocument({ siteName, legalName, contactEmail }: LegalContext): LegalDocument {
  return {
    title: "Privacy Policy",
    sections: [
      {
        heading: "Who is responsible",
        paragraphs: [
          `${siteName} is operated by ${legalName}. For any privacy question, email ${contactEmail}.`,
        ],
      },
      {
        heading: "What we collect",
        paragraphs: [
          "Your email address, when you join the waitlist or sign in.",
          "Photos you upload and their technical data (EXIF). We remove GPS location data from photos.",
          "Product outlines, sizes and the designs you make.",
          "Payment details are collected by Paddle, our merchant of record. We never see your card number.",
          "Visit statistics from Plausible Analytics, which uses no cookies and collects no personal data.",
        ],
      },
      {
        heading: "Why we use it",
        paragraphs: [
          "To measure your products, generate your designs and files, sign you in, send beta invites and service emails, and keep the service secure.",
        ],
      },
      {
        heading: "Photos",
        paragraphs: [
          "Photos are deleted automatically after 30 days. We keep only the outlines and sizes measured from them.",
          "When our software can't read a photo clearly, the part showing the paper may be sent to an AI provider to help find the product: Hugging Face and its inference partner, Google (Gemini API) or fal.ai. You can opt out in your settings.",
          "Google may use what it receives through its free Gemini service to improve its products, and its reviewers may read it. We never use that free service for visitors in the European Economic Area, the United Kingdom or Switzerland.",
        ],
      },
      {
        heading: "Who we share data with",
        paragraphs: [
          "Only service providers that help us run the service: hosting and file storage, email delivery, Paddle for payments, Plausible for visit statistics, and AI providers (Hugging Face, Google, fal.ai) for unclear photos unless you opt out.",
          "We don't sell your data.",
        ],
      },
      {
        heading: "Cookies",
        paragraphs: [
          "We use no tracking cookies. When you sign in, we set one essential cookie to keep you signed in.",
        ],
      },
      {
        heading: "Your rights",
        paragraphs: [
          `You can ask us to show, correct, export or delete your data at any time by emailing ${contactEmail}.`,
        ],
      },
      {
        heading: "Changes",
        paragraphs: ["If we change this policy in an important way, we will tell you before it applies."],
      },
    ],
  };
}
