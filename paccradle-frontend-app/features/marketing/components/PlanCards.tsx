import { revealDelay } from "@/components/motion/reveal";
import { Icon } from "@/components/ui/Icon";
import { formatUsd, t } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import { PRICES } from "../pricing";
import styles from "./PlanCards.module.css";

/** One plan as shown on a card. */
interface PlanView {
  readonly id: string;
  readonly name: string;
  readonly price: string;
  readonly unit: string;
  readonly body: string;
  readonly features: readonly string[];
  readonly note: Maybe<string>;
  /** Small tag in the corner; only the highlighted plan has one. */
  readonly tag: Maybe<string>;
  readonly featured: boolean;
}

const FREE_CENTS = 0;

/** The four launch plans (CLAUDE.md §10). Studio comes in v2 and isn't shown. */
function plans(): readonly PlanView[] {
  return [
    {
      id: "free",
      name: t("pricing.free.name"),
      price: formatUsd(FREE_CENTS),
      unit: t("pricing.free.unit"),
      body: t("pricing.free.body"),
      features: [t("pricing.free.f1"), t("pricing.free.f2"), t("pricing.free.f3")],
      note: none,
      tag: none,
      featured: false,
    },
    {
      id: "single",
      name: t("pricing.single.name"),
      price: formatUsd(PRICES.singleExport),
      unit: t("pricing.single.unit"),
      body: t("pricing.single.body"),
      features: [t("pricing.single.f1"), t("pricing.single.f2"), t("pricing.single.f3")],
      note: none,
      tag: none,
      featured: false,
    },
    {
      id: "pack",
      name: t("pricing.pack.name"),
      price: formatUsd(PRICES.fivePack),
      unit: t("pricing.pack.unit"),
      body: t("pricing.pack.body"),
      features: [t("pricing.pack.f1"), t("pricing.pack.f2")],
      note: none,
      tag: none,
      featured: false,
    },
    {
      id: "pro",
      name: t("pricing.pro.name"),
      price: formatUsd(PRICES.proMonthly),
      unit: t("pricing.pro.unit"),
      body: t("pricing.pro.body"),
      features: [t("pricing.pro.f1"), t("pricing.pro.f2"), t("pricing.pro.f3")],
      note: some(t("pricing.pro.yearly", { price: formatUsd(PRICES.proYearly) })),
      tag: some(t("pricing.pro.tag")),
      featured: true,
    },
  ];
}

/** Plan cards for the landing and pricing pages. Pro gets a slowly turning gradient border. */
export function PlanCards() {
  return (
    <ul className={styles.grid}>
      {plans().map((plan, index) => (
        <li key={plan.id} className={styles.cell} data-reveal style={revealDelay(index)}>
          <article
            aria-labelledby={`plan-${plan.id}`}
            className={`${styles.card} ${plan.featured ? styles.featured : ""}`}
          >
            {plan.tag.kind === "some" && <p className={styles.tag}>{plan.tag.value}</p>}
            <h3 id={`plan-${plan.id}`} className={styles.name}>
              {plan.name}
            </h3>
            <p className={styles.price}>
              <strong>{plan.price}</strong> <span className={styles.unit}>{plan.unit}</span>
            </p>
            {plan.note.kind === "some" && <p className={styles.unit}>{plan.note.value}</p>}
            <p className={styles.body}>{plan.body}</p>
            <ul className={styles.features}>
              {plan.features.map((feature) => (
                <li key={feature}>
                  <Icon name="check" size={16} className={styles.check} />
                  {feature}
                </li>
              ))}
            </ul>
          </article>
        </li>
      ))}
    </ul>
  );
}
