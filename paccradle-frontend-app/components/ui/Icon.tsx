import styles from "./Icon.module.css";

/** Line icons on a 24 × 24 grid, drawn with `currentColor`. */
const ICON_PATHS = {
  arrow: ["M5 12h14", "M13 6l6 6-6 6"],
  camera: [
    "M4.5 8h3l1.6-2.5h5.8L16.5 8h3A1.5 1.5 0 0 1 21 9.5v8a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5v-8A1.5 1.5 0 0 1 4.5 8Z",
    "M15.3 13.2a3.3 3.3 0 1 1-6.6 0 3.3 3.3 0 0 1 6.6 0Z",
  ],
  insert: ["M9 4h6v5h5v6h-5v5H9v-5H4V9h5Z", "M13.6 12a1.6 1.6 0 1 1-3.2 0 1.6 1.6 0 0 1 3.2 0Z"],
  cube: ["M12 3 20 7.5v9L12 21l-8-4.5v-9Z", "M4 7.5 12 12l8-4.5", "M12 12v9"],
  layers: ["m12 4 8.5 4.5L12 13 3.5 8.5Z", "m3.5 12.5 8.5 4.5 8.5-4.5", "m3.5 16.5 8.5 4.5 8.5-4.5"],
  printer: [
    "M7 9V4h10v5",
    "M7 17H5a1.5 1.5 0 0 1-1.5-1.5v-5A1.5 1.5 0 0 1 5 9h14a1.5 1.5 0 0 1 1.5 1.5v5A1.5 1.5 0 0 1 19 17h-2",
    "M7 14h10v6H7Z",
  ],
  file: [
    "M14 3.5H7.5A1.5 1.5 0 0 0 6 5v14a1.5 1.5 0 0 0 1.5 1.5h9A1.5 1.5 0 0 0 18 19V7.5Z",
    "M14 3.5v4h4",
    "M9 12.5h6",
    "M9 16h4",
  ],
  shield: ["M12 3.5 19 6v5.5c0 4.2-2.9 7.4-7 9-4.1-1.6-7-4.8-7-9V6Z", "m9 12 2.2 2.2L15.5 10"],
  leaf: ["M5.5 18.5C5 11 10 5.5 18.5 5c.6 8-4.6 13.4-13 13.5Z", "M5.5 18.5 14 10"],
  check: ["m5 12.5 4.5 4.5L19 7.5"],
  bottle: [
    "M10 3h4v3.2l1.8 2.3c.5.6.7 1.3.7 2V19a2 2 0 0 1-2 2h-5a2 2 0 0 1-2-2v-8.5c0-.7.2-1.4.7-2L10 6.2Z",
    "M7.5 13h9",
  ],
  pen: ["M4 20l1-4.5L15.5 5a2.1 2.1 0 0 1 3 3L8 18.5Z", "M13.5 7l3 3"],
  scissors: [
    "M8.4 8.6 20 17",
    "M8.4 15.4 20 7",
    "M9 6.5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0Z",
    "M9 17.5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0Z",
  ],
  play: ["M9 6.5v11l9-5.5Z"],
  plus: ["M12 5v14", "M5 12h14"],
  close: ["M6 6l12 12", "M18 6 6 18"],
  sparkle: [
    "M12 3.5c.6 4.2 2.4 6.3 6.5 7-4.1.7-5.9 2.8-6.5 7-.6-4.2-2.4-6.3-6.5-7 4.1-.7 5.9-2.8 6.5-7Z",
  ],
} as const;

/** Names of the available icons. */
export type IconName = keyof typeof ICON_PATHS;

/** Props for an icon. */
export interface IconProps {
  readonly name: IconName;
  /** Size in pixels (square). */
  readonly size?: number;
  readonly className?: string;
}

const DEFAULT_SIZE = 20;

/** A decorative line icon; hidden from screen readers (put the meaning in nearby text). */
export function Icon({ name, size = DEFAULT_SIZE, className = "" }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      aria-hidden="true"
      className={`${styles.icon} ${className}`}
    >
      {ICON_PATHS[name].map((path) => (
        <path key={path} d={path} />
      ))}
    </svg>
  );
}
