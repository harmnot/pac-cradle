"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useId, useRef, useState } from "react";
import { TransitionType } from "@/components/motion/PageTransition";
import { Button, ButtonVariant, LinkButton } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { LogoMark } from "@/features/marketing/components/LogoMark";
import { t } from "@/lib/i18n";
import { AppRoute, withNext } from "../routes";
import { isAccount, useSession } from "../session";
import { AccountRole, type Account } from "../types";
import styles from "./AppHeader.module.css";

/** The first letter of the account's email, for its avatar. */
export function initialOf(account: Account): string {
  return account.email.kind === "some" ? account.email.value.charAt(0).toUpperCase() : "·";
}

/** The account's avatar button and its menu (email, account, sign out). */
function UserMenu({ account }: { readonly account: Account }) {
  const router = useRouter();
  const { signOut } = useSession();
  const [open, setOpen] = useState(false);
  const [leaving, setLeaving] = useState(false);
  const menuId = useId();
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    const close = (event: Event) => {
      const outside = event instanceof MouseEvent && !root.current?.contains(event.target as Node);
      const escape = event instanceof KeyboardEvent && event.key === "Escape";
      if (outside || escape) {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", close);
    return () => {
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", close);
    };
  }, [open]);

  const leave = async () => {
    setLeaving(true);
    await signOut();
    router.replace(AppRoute.SignIn, { transitionTypes: [TransitionType.Back] });
  };

  return (
    <div className={styles.menuRoot} ref={root}>
      <button
        type="button"
        className={styles.avatar}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        aria-label={t("app.menu.open")}
        onClick={() => setOpen((shown) => !shown)}
      >
        {initialOf(account)}
      </button>
      <div id={menuId} role="menu" className={styles.menu} data-open={open} hidden={!open}>
        <p className={styles.menuWho}>
          <span>{t("app.menu.signedInAs")}</span>
          <strong>{account.email.kind === "some" ? account.email.value : ""}</strong>
          {account.role === AccountRole.Admin && <span className={styles.badge}>{t("account.role.admin")}</span>}
        </p>
        <Link role="menuitem" href={AppRoute.Account} className={styles.menuItem} onClick={() => setOpen(false)}>
          <Icon name="user" size={18} />
          {t("app.nav.account")}
        </Link>
        <button role="menuitem" type="button" className={styles.menuItem} onClick={leave} disabled={leaving}>
          <Icon name="logOut" size={18} />
          {leaving ? t("app.menu.signingOut") : t("app.menu.signOut")}
        </button>
      </div>
    </div>
  );
}

/**
 * The header's frame while the live one waits for the page's address (a capture's id is only
 * known in the browser): the mark and an empty place for the person's menu.
 */
export function AppHeaderFallback() {
  return (
    <header className={styles.header}>
      <span className={styles.brand}>
        <LogoMark />
        <span className={styles.wordmark}>{t("site.name")}</span>
      </span>
      <div className={styles.end}>
        <span className={styles.avatarPlaceholder} aria-hidden="true" />
      </div>
    </header>
  );
}

/**
 * The app's header: the mark, the workspace and account links, and the person's menu (or
 * sign-in buttons for a visitor or a guest). It holds still while pages slide
 * (`view-transition-name: app-header`). It reads the address, so the layout puts it inside
 * `<Suspense>` (routes like `/capture/[captureId]` know theirs only in the browser).
 */
export function AppHeader() {
  const { state } = useSession();
  const pathname = usePathname();
  const links = [
    { href: AppRoute.Workspace, label: t("app.nav.workspace") },
    { href: AppRoute.Capture, label: t("app.nav.capture") },
    { href: AppRoute.Account, label: t("app.nav.account") },
  ];

  return (
    <header className={styles.header}>
      <Link href={AppRoute.Workspace} className={styles.brand} aria-label={t("app.nav.workspace")}>
        <LogoMark />
        <span className={styles.wordmark}>{t("site.name")}</span>
      </Link>
      <nav aria-label={t("app.nav.label")}>
        <ul className={styles.links}>
          {links.map((link) => (
            <li key={link.href}>
              <Link
                href={link.href}
                className={styles.link}
                aria-current={pathname === link.href || pathname.startsWith(`${link.href}/`) ? "page" : undefined}
              >
                {link.label}
              </Link>
            </li>
          ))}
        </ul>
      </nav>
      <div className={styles.end}>
        {isAccount(state) && <UserMenu account={state.account} />}
        {(state.status === "none" || (state.status === "active" && !isAccount(state))) && (
          <>
            <LinkButton href={withNext({ route: AppRoute.SignIn, next: pathname })} variant={ButtonVariant.Quiet}>
              {t("app.signIn")}
            </LinkButton>
            <LinkButton href={withNext({ route: AppRoute.SignUp, next: pathname })} withArrow>
              {t("app.signUp")}
            </LinkButton>
          </>
        )}
        {state.status === "starting" && <span className={styles.avatarPlaceholder} aria-hidden="true" />}
        {state.status === "unreachable" && (
          <Button variant={ButtonVariant.Secondary} onClick={() => window.location.reload()}>
            <Icon name="refresh" size={16} />
            {t("session.retry")}
          </Button>
        )}
      </div>
    </header>
  );
}
