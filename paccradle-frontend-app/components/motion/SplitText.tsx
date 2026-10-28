import { Fragment, type CSSProperties } from "react";
import styles from "./SplitText.module.css";

/** Props for word-by-word animated text. */
export interface SplitTextProps {
  readonly text: string;
  /** Position of the first word, to continue one stagger across several SplitTexts. */
  readonly startIndex?: number;
  /** Also render the plain text for screen readers; turn off when the parent provides it. */
  readonly withLabel?: boolean;
}

/**
 * Text whose words rise into view one after another. Plays on page load, or — inside a
 * `[data-reveal="words"]` element — when that element scrolls into view.
 */
export function SplitText({ text, startIndex = 0, withLabel = true }: SplitTextProps) {
  const words = text.split(" ").filter((word) => word !== "");
  const lastIndex = words.length - 1;
  return (
    <>
      {withLabel && <span className="visually-hidden">{text}</span>}
      <span aria-hidden="true">
        {words.map((word, index) => (
          <Fragment key={`${index}-${word}`}>
            <span className={styles.word}>
              <span
                className={styles.inner}
                style={{ "--word-index": startIndex + index } as CSSProperties}
              >
                {word}
              </span>
            </span>
            {index < lastIndex ? " " : ""}
          </Fragment>
        ))}
      </span>
    </>
  );
}
