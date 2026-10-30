import type { Metadata } from "next";
import type { ReactNode } from "react";
import { SessionProvider } from "@/features/identity/session";

/** App pages are personal: keep them out of search results. */
export const metadata: Metadata = {
  robots: { index: false, follow: false },
};

/**
 * The app (sign-in, workspace, account): every page shares one session. The session lives in
 * the browser (the refresh cookie belongs to the API's host), so the shell prerenders and the
 * session starts on the client.
 */
export default function AppLayout({ children }: Readonly<{ children: ReactNode }>) {
  return <SessionProvider>{children}</SessionProvider>;
}
