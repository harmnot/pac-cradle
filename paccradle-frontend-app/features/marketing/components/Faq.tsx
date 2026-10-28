import Link from "next/link";
import { revealDelay } from "@/components/motion/reveal";
import { Icon } from "@/components/ui/Icon";
import { t } from "@/lib/i18n";
import styles from "./Faq.module.css";

/** Pricing questions as native disclosure widgets; the plus turns into a cross when open. */
export function Faq() {
  const items = [
    { id: "export", question: t("faq.export.q"), answer: t("faq.export.a") },
    { id: "edit", question: t("faq.edit.q"), answer: t("faq.edit.a") },
    { id: "paddle", question: t("faq.paddle.q"), answer: t("faq.paddle.a") },
  ] as const;
  const refundPosition = items.length;

  return (
    <ul className={styles.list}>
      {items.map((item, index) => (
        <li key={item.id} data-reveal style={revealDelay(index)}>
          <details className={styles.item}>
            <summary className={styles.question}>
              {item.question}
              <Icon name="plus" size={18} className={styles.toggle} />
            </summary>
            <p className={styles.answer}>{item.answer}</p>
          </details>
        </li>
      ))}
      <li data-reveal style={revealDelay(refundPosition)}>
        <details className={styles.item}>
          <summary className={styles.question}>
            {t("faq.refund.q")}
            <Icon name="plus" size={18} className={styles.toggle} />
          </summary>
          <p className={styles.answer}>
            {t("faq.refund.a")} <Link href="/refund">{t("faq.refund.link")}</Link>
          </p>
        </details>
      </li>
    </ul>
  );
}
