"use client";

import { useId, useRef, useState, type ChangeEvent, type DragEvent } from "react";
import { Button, ButtonSize, ButtonVariant } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { t } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import { Sketch } from "./Sketch";
import styles from "./PhotoStep.module.css";

/** What the photo step is doing. */
export type PhotoBusy = "none" | "preparing" | "uploading";

/** Props for the photo step. */
export interface PhotoStepProps {
  readonly busy: PhotoBusy;
  /** The photo being prepared or sent, as an object URL. */
  readonly preview: Maybe<string>;
  /** What went wrong with the last photo. */
  readonly problem: Maybe<string>;
  readonly onFile: (file: File) => void;
  readonly onBack: () => void;
}

/** The first file in a drop or a file input. */
function firstFile(files: FileList | null): Maybe<File> {
  const file = files === null ? undefined : Array.from(files)[0];
  return file === undefined ? none : some(file);
}

/**
 * Step 3: the photo. Drop it, choose it, or (on a phone) take it with the back camera. While
 * it is prepared and sent, the photo itself shows with a scanning beam.
 */
export function PhotoStep({ busy, preview, problem, onFile, onBack }: PhotoStepProps) {
  const cameraInput = useRef<HTMLInputElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);
  const hintId = useId();
  const working = busy !== "none";

  const take = (files: FileList | null) => {
    const file = firstFile(files);
    if (file.kind === "some") {
      onFile(file.value);
    }
  };

  const onChange = (event: ChangeEvent<HTMLInputElement>) => {
    take(event.target.files);
    event.target.value = "";
  };

  const onDrop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setDragging(false);
    if (!working) {
      take(event.dataTransfer.files);
    }
  };

  return (
    <section className={styles.step} aria-labelledby="photo-title" aria-busy={working}>
      <header className={styles.head}>
        <h1 id="photo-title" className={styles.title}>
          {t("capture.photo.title")}
        </h1>
        <p className={styles.lead}>{t("capture.photo.lead")}</p>
      </header>

      {problem.kind === "some" && <Notice tone={NoticeTone.Error}>{problem.value}</Notice>}

      <div
        className={styles.drop}
        data-dragging={dragging}
        data-working={working}
        onDragOver={(event) => {
          event.preventDefault();
          setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={onDrop}
      >
        {preview.kind === "some" ? (
          <figure className={styles.preview}>
            {/* An object URL of the person's own photo: next/image can't optimise it. */}
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src={preview.value} alt={t("capture.photo.previewAlt")} className={styles.previewImage} />
            {working && <span className={styles.beam} aria-hidden="true" />}
            <figcaption className={styles.status} role="status" aria-live="polite">
              <span className={styles.statusDot} aria-hidden="true" />
              {busy === "uploading" ? t("capture.photo.uploading") : t("capture.photo.preparing")}
            </figcaption>
          </figure>
        ) : (
          <div className={styles.empty}>
            <Sketch name="photo" className={styles.emptyArt} />
            <p className={styles.dropText}>{t("capture.photo.drop")}</p>
            <p id={hintId} className={styles.hint}>
              {t("capture.photo.formats")}
            </p>
          </div>
        )}
        <div className={styles.buttons}>
          <Button
            size={ButtonSize.Large}
            onClick={() => cameraInput.current?.click()}
            disabled={working}
            aria-describedby={hintId}
          >
            <Icon name="camera" size={18} />
            {t("capture.photo.take")}
          </Button>
          <Button
            size={ButtonSize.Large}
            variant={ButtonVariant.Secondary}
            onClick={() => fileInput.current?.click()}
            disabled={working}
            aria-describedby={hintId}
          >
            <Icon name="file" size={18} />
            {t("capture.photo.choose")}
          </Button>
        </div>
        <input
          ref={cameraInput}
          type="file"
          accept="image/*"
          capture="environment"
          className="visually-hidden"
          tabIndex={-1}
          onChange={onChange}
        />
        <input
          ref={fileInput}
          type="file"
          accept="image/jpeg,image/png,image/webp,image/heic,image/heif,.heic,.heif"
          className="visually-hidden"
          tabIndex={-1}
          onChange={onChange}
        />
      </div>

      <Button variant={ButtonVariant.Quiet} onClick={onBack} disabled={working}>
        {t("capture.back")}
      </Button>
    </section>
  );
}
