"use client";

import { useId, useState, type InputHTMLAttributes, type Ref } from "react";
import { t } from "@/lib/i18n";
import { Icon } from "./Icon";
import styles from "./TextField.module.css";

/** Props for a labelled text input. */
export interface TextFieldProps
  extends Omit<InputHTMLAttributes<HTMLInputElement>, "className" | "id" | "aria-describedby"> {
  readonly label: string;
  /** Help under the field, read with it by screen readers. */
  readonly hint?: string;
  /** What's wrong with the value; marks the field invalid and gives it a short shake. */
  readonly error?: string;
  /** For `type="password"`: a button that shows the password while typing. */
  readonly revealable?: boolean;
  readonly ref?: Ref<HTMLInputElement>;
}

/**
 * A text field on paper: the label sits above, the border warms to terracotta on focus, and an
 * error appears below with the field marked `aria-invalid`. Uses native validation attributes.
 */
export function TextField({
  label,
  hint,
  error,
  revealable = false,
  type = "text",
  ref,
  ...input
}: TextFieldProps) {
  const id = useId();
  const hintId = useId();
  const errorId = useId();
  const [revealed, setRevealed] = useState(false);
  const describedBy = [hint ? hintId : "", error ? errorId : ""].filter(Boolean).join(" ");
  const inputType = revealable && revealed ? "text" : type;

  return (
    <div className={styles.field} data-invalid={error ? "true" : "false"}>
      <label htmlFor={id} className={styles.label}>
        {label}
      </label>
      <div className={styles.control}>
        <input
          {...input}
          ref={ref}
          id={id}
          type={inputType}
          aria-invalid={error ? true : undefined}
          aria-describedby={describedBy || undefined}
          className={styles.input}
        />
        {revealable && (
          <button
            type="button"
            className={styles.reveal}
            aria-pressed={revealed}
            aria-label={revealed ? t("field.password.hide") : t("field.password.show")}
            onClick={() => setRevealed((shown) => !shown)}
          >
            <Icon name={revealed ? "eyeOff" : "eye"} size={18} />
          </button>
        )}
        <span className={styles.focusRing} aria-hidden="true" />
      </div>
      {hint && (
        <p id={hintId} className={styles.hint}>
          {hint}
        </p>
      )}
      {error && (
        <p id={errorId} className={styles.error} role="alert">
          <Icon name="alert" size={15} />
          {error}
        </p>
      )}
    </div>
  );
}
