"use client";

import { useId, useRef } from "react";
import { Button, ButtonSize, ButtonVariant } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { PRICES } from "@/features/marketing/pricing";
import { AnalyticsEvent, trackEvent } from "@/lib/analytics";
import { formatUsd, t } from "@/lib/i18n";
import { WaitlistSource } from "../types";
import { WaitlistForm } from "./WaitlistForm";
import styles from "./FakeDoorButton.module.css";

/**
 * The fake-door test (CLAUDE.md §12, M0-04): a "Download files – $10" button that
 * records the click, then explains that exports open with the beta and offers the waitlist.
 * The dialog scales in over a blurred backdrop.
 */
export function FakeDoorButton() {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  const open = () => {
    trackEvent(AnalyticsEvent.FakeDoorClick);
    dialogRef.current?.showModal();
  };

  return (
    <>
      <Button variant={ButtonVariant.Secondary} size={ButtonSize.Large} onClick={open}>
        <Icon name="file" size={18} />
        {t("fakeDoor.button", { price: formatUsd(PRICES.singleExport) })}
      </Button>
      <dialog ref={dialogRef} aria-labelledby={titleId} className={styles.dialog}>
        <span className={styles.badge} aria-hidden="true">
          <Icon name="sparkle" size={22} />
        </span>
        <h2 id={titleId} className={styles.title}>
          {t("fakeDoor.title")}
        </h2>
        <p className={styles.body}>{t("fakeDoor.body")}</p>
        <WaitlistForm source={WaitlistSource.FakeDoor} />
        {/* Last in the DOM so opening the dialog focuses the email field first. */}
        <form method="dialog" className={styles.close}>
          <button type="submit" className={styles.closeButton} aria-label={t("fakeDoor.close")}>
            <Icon name="close" size={18} />
          </button>
        </form>
      </dialog>
    </>
  );
}
