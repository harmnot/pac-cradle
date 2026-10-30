"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { startTransition, useEffect, useReducer } from "react";
import { StepTransition, TransitionType } from "@/components/motion/PageTransition";
import { Paper } from "@/features/identity/types";
import { useSession } from "@/features/identity/session";
import { describeError } from "@/lib/errors";
import { t, type MessageKey } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import { uploadCapture } from "../api";
import { rememberPaper, useDefaultPaper } from "../paper";
import { holdPhoto } from "../photo-memory";
import { preparePhoto, PrepareFailure } from "../prepare-photo";
import { CaptureMode, DEFAULT_PEN_WIDTH_MM } from "../types";
import { FlowMode, ModeStep } from "./ModeStep";
import { PhotoStep, type PhotoBusy } from "./PhotoStep";
import { SetupStep } from "./SetupStep";
import { StepRail } from "./StepRail";
import { TypedStep } from "./TypedStep";
import styles from "./CaptureFlow.module.css";

/** The query parameter that starts the flow in a mode (`retake`, "try Trace mode"). */
export const MODE_PARAM = "mode";

/** The flow's screens. */
type Step = "mode" | "setup" | "photo" | "typed";

interface State {
  readonly step: Step;
  readonly mode: CaptureMode;
  /** The paper the person picked here; until then, this browser's default. */
  readonly paper: Maybe<Paper>;
  readonly penWidthMm: number;
  readonly busy: PhotoBusy;
  readonly preview: Maybe<string>;
  readonly problem: Maybe<string>;
}

type Action =
  | { readonly type: "choose"; readonly mode: FlowMode }
  | { readonly type: "go"; readonly step: Step }
  | { readonly type: "paper"; readonly paper: Paper }
  | { readonly type: "pen"; readonly width: number }
  | { readonly type: "busy"; readonly busy: PhotoBusy; readonly preview: Maybe<string> }
  | { readonly type: "problem"; readonly problem: string };

function reduce(state: State, action: Action): State {
  switch (action.type) {
    case "choose":
      return action.mode === FlowMode.Typed
        ? { ...state, step: "typed" }
        : { ...state, step: "setup", mode: action.mode, problem: none };
    case "go":
      return { ...state, step: action.step, problem: none };
    case "paper":
      return { ...state, paper: some(action.paper) };
    case "pen":
      return { ...state, penWidthMm: action.width };
    case "busy":
      return { ...state, busy: action.busy, preview: action.preview, problem: none };
    case "problem":
      return { ...state, busy: "none", preview: none, problem: some(action.problem) };
  }
}

const PREPARE_MESSAGES: Readonly<Record<PrepareFailure, MessageKey>> = {
  [PrepareFailure.HeicUnsupported]: "capture.photo.heic",
  [PrepareFailure.Unreadable]: "capture.photo.unreadable",
  [PrepareFailure.TooSmall]: "capture.photo.tooSmall",
};

/** The first state, from `?mode=` (a retake or "try Trace mode" starts at the setup). */
function initial(mode: string | null): State {
  const known = mode === FlowMode.Photo || mode === FlowMode.Trace || mode === FlowMode.Typed;
  const step: Step = !known ? "mode" : mode === FlowMode.Typed ? "typed" : "setup";
  return {
    step,
    mode: mode === FlowMode.Trace ? CaptureMode.Trace : CaptureMode.Photo,
    paper: none,
    penWidthMm: DEFAULT_PEN_WIDTH_MM,
    busy: "none",
    preview: none,
    problem: none,
  };
}

const RAIL_STEPS: Readonly<Record<Step, number>> = { mode: 0, setup: 1, photo: 2, typed: 1 };

/**
 * A new capture (CLAUDE.md §3 steps 1–3): how to measure, the paper and how to take the
 * photo, then the photo, which is read, re-encoded and sent before the capture page takes
 * over to watch it being measured. A visitor gets a guest session at the upload.
 */
export function CaptureFlow() {
  const router = useRouter();
  const { ensureSession } = useSession();
  const [state, dispatch] = useReducer(reduce, useSearchParams().get(MODE_PARAM), initial);
  const browserPaper = useDefaultPaper();
  const paper = state.paper.kind === "some" ? state.paper.value : browserPaper;

  // The preview is an object URL of the person's photo: let it go when it changes.
  useEffect(() => {
    const preview = state.preview;
    return () => {
      if (preview.kind === "some") {
        URL.revokeObjectURL(preview.value);
      }
    };
  }, [state.preview]);

  const move = (action: Action) => startTransition(() => dispatch(action));

  const upload = async (file: File) => {
    dispatch({ type: "busy", busy: "preparing", preview: some(URL.createObjectURL(file)) });
    const prepared = await preparePhoto(file);
    if (prepared.kind === "failed") {
      dispatch({ type: "problem", problem: t(PREPARE_MESSAGES[prepared.failure]) });
      return;
    }
    const photo = prepared.photo;
    dispatch({ type: "busy", busy: "uploading", preview: some(URL.createObjectURL(photo.blob)) });
    const session = await ensureSession();
    if (session.kind === "error") {
      dispatch({ type: "problem", problem: describeError(session.error) });
      return;
    }
    const result = await uploadCapture({
      photo: photo.blob,
      settings: {
        paper,
        mode: state.mode,
        penWidthMm: state.mode === CaptureMode.Trace ? some(state.penWidthMm) : none,
        exif: photo.exif,
      },
    });
    if (result.kind === "error") {
      dispatch({ type: "problem", problem: describeError(result.error) });
      return;
    }
    const captureId = result.value.capture_id;
    holdPhoto({ captureId, blob: photo.blob, width: photo.width, height: photo.height });
    router.push(`/capture/${captureId}`, { transitionTypes: [TransitionType.Forward] });
  };

  const rail =
    state.step === "typed"
      ? [t("capture.rail.mode"), t("capture.rail.sizes")]
      : [t("capture.rail.mode"), t("capture.rail.setup"), t("capture.rail.photo"), t("capture.rail.measure")];

  return (
    <div className={styles.flow}>
      <StepRail steps={rail} current={RAIL_STEPS[state.step]} />
      <StepTransition step={state.step}>
        <div className={styles.screen}>
          {state.step === "mode" && <ModeStep onChoose={(mode) => move({ type: "choose", mode })} />}
          {state.step === "setup" && (
            <SetupStep
              mode={state.mode}
              paper={paper}
              penWidthMm={state.penWidthMm}
              onPaper={(paper) => {
                rememberPaper(paper);
                dispatch({ type: "paper", paper });
              }}
              onPenWidth={(width) => dispatch({ type: "pen", width })}
              onReady={() => move({ type: "go", step: "photo" })}
              onBack={() => move({ type: "go", step: "mode" })}
            />
          )}
          {state.step === "photo" && (
            <PhotoStep
              busy={state.busy}
              preview={state.preview}
              problem={state.problem}
              onFile={(file) => void upload(file)}
              onBack={() => move({ type: "go", step: "setup" })}
            />
          )}
          {state.step === "typed" && <TypedStep onBack={() => move({ type: "go", step: "mode" })} />}
        </div>
      </StepTransition>
    </div>
  );
}
