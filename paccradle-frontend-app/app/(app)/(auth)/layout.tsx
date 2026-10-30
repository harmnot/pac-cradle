import type { ReactNode } from "react";
import { AuthShell } from "@/features/identity/components/AuthShell";

/** Sign-in screens share the shell; its art panel stays put while they slide. */
export default function AuthLayout({ children }: Readonly<{ children: ReactNode }>) {
  return <AuthShell>{children}</AuthShell>;
}
