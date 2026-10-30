import type { Metadata } from "next";
import { Fraunces, Geist, Geist_Mono } from "next/font/google";
import Script from "next/script";
import { publicConfig } from "@/lib/config";
import { t } from "@/lib/i18n";
import "../styles/tokens.css";
import "../styles/base.css";
import "../styles/motion.css";
import "../styles/transitions.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

/** Display serif for headings: soft, organic shapes that suit kraft-paper packaging. */
const fraunces = Fraunces({
  variable: "--font-fraunces",
  subsets: ["latin"],
  style: ["normal", "italic"],
  axes: ["SOFT", "WONK", "opsz"],
});

/** If the motion controller hasn't started by then, show everything without animation. */
const MOTION_FALLBACK_MS = 4000;

/**
 * Runs before first paint: lets scroll reveals hide content until it animates in
 * (`motion-ready`, see styles/motion.css), and undoes that when JavaScript is slow or fails.
 */
const MOTION_BOOT_SCRIPT = `(function(){var d=document.documentElement;d.classList.add("motion-ready");setTimeout(function(){if(!d.dataset.motionLive){d.classList.remove("motion-ready")}},${MOTION_FALLBACK_MS})})();`;

export const metadata: Metadata = {
  title: { default: t("site.title"), template: `%s · ${t("site.name")}` },
  description: t("site.description"),
};

/** Root layout: fonts, global styles (tokens, base, motion) and Plausible when configured. */
export default function RootLayout({ children }: LayoutProps<"/">) {
  const { plausibleDomain, plausibleScriptSrc } = publicConfig;
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} ${fraunces.variable}`}
      data-scroll-behavior="smooth"
      suppressHydrationWarning
    >
      <head>
        <script dangerouslySetInnerHTML={{ __html: MOTION_BOOT_SCRIPT }} />
      </head>
      <body>
        <a href="#main" className="skip-link">
          {t("nav.skip")}
        </a>
        {children}
        {plausibleDomain.kind === "some" && (
          <Script
            src={plausibleScriptSrc}
            data-domain={plausibleDomain.value}
            strategy="afterInteractive"
          />
        )}
      </body>
    </html>
  );
}
