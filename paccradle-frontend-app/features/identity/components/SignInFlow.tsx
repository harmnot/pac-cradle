"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { startTransition, useEffect, useReducer, useRef, useState, type FormEvent } from "react";
import { StepTransition, TransitionType } from "@/components/motion/PageTransition";
import { Button, ButtonSize, ButtonVariant } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Notice, NoticeTone } from "@/components/ui/Notice";
import { TextField } from "@/components/ui/TextField";
import { describeError } from "@/lib/errors";
import { t } from "@/lib/i18n";
import { none, some, type Maybe } from "@/lib/maybe";
import * as identity from "../api";
import {
  autofillSupported,
  canMakePasskeyHere,
  getPasskey,
  Mediation,
  PasskeyFailure,
  passkeysSupported,
  type AuthenticationResponse,
} from "../passkey";
import { AppRoute, NEXT_PARAM, routeAfterSignIn, safeNextPath, withNext } from "../routes";
import { isAccount, useSession } from "../session";
import type { Session } from "../types";
import { AuthCard } from "./AuthShell";
import { InboxSent } from "./InboxSent";
import styles from "./SignInFlow.module.css";

/** After this many failed passkey tries, the password form and the email link take over. */
const PASSKEY_TRIES = 2;
const ERROR_INVALID_CREDENTIALS = "AUTH_INVALID_CREDENTIALS";
const ERROR_PASSKEY_REJECTED = "AUTH_PASSKEY_REJECTED";

/** The three screens of a sign-in. */
const View = {
  Passkey: "passkey",
  Password: "password",
  Sent: "sent",
} as const;

type View = (typeof View)[keyof typeof View];

/** What is running. */
type Busy = "none" | "passkey" | "password" | "link";

interface Message {
  readonly tone: NoticeTone;
  readonly text: string;
}

interface State {
  readonly view: View;
  readonly failures: number;
  readonly busy: Busy;
  readonly notice: Maybe<Message>;
  readonly passwordError: Maybe<string>;
}

type Action =
  | { readonly type: "view"; readonly view: View; readonly notice?: Message }
  | { readonly type: "busy"; readonly busy: Busy }
  | { readonly type: "notice"; readonly notice: Message }
  | { readonly type: "passkey_failed" }
  | { readonly type: "wrong_password" };

const INITIAL: State = {
  view: View.Passkey,
  failures: 0,
  busy: "none",
  notice: none,
  passwordError: none,
};

function reduce(state: State, action: Action): State {
  switch (action.type) {
    case "view":
      return {
        ...state,
        view: action.view,
        busy: "none",
        notice: action.notice ? some(action.notice) : none,
        passwordError: none,
      };
    case "busy":
      return { ...state, busy: action.busy, notice: action.busy === "none" ? state.notice : none };
    case "notice":
      return { ...state, busy: "none", notice: some(action.notice) };
    case "passkey_failed": {
      const failures = state.failures + 1;
      if (failures >= PASSKEY_TRIES) {
        return {
          ...state,
          failures,
          view: View.Password,
          busy: "none",
          notice: some({ tone: NoticeTone.Info, text: t("signIn.passkey.fallback") }),
        };
      }
      return {
        ...state,
        failures,
        busy: "none",
        notice: some({ tone: NoticeTone.Error, text: t("signIn.passkey.failed") }),
      };
    }
    case "wrong_password":
      return { ...state, busy: "none", passwordError: some(t("error.AUTH_INVALID_CREDENTIALS")) };
  }
}

/**
 * Sign-in, passkey first (MILESTONES M1-E1, DEC-07): one tap with the passkey on this device,
 * or the email field's autofill. After two failed passkey tries, or when the person asks,
 * the password form and "email me a sign-in link" (also "forgot password") take over.
 */
export function SignInFlow() {
  const router = useRouter();
  const params = useSearchParams();
  const next = safeNextPath(params.get(NEXT_PARAM));
  const { state: session, signedIn } = useSession();
  const [state, dispatch] = useReducer(reduce, INITIAL);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [armed, rearm] = useReducer((count: number) => count + 1, 0);
  const autofill = useRef<Maybe<AbortController>>(none);
  const finishing = useRef(false);
  const emailField = useRef<HTMLInputElement>(null);

  const show = (view: View, notice?: Message) =>
    startTransition(() => dispatch({ type: "view", view, notice }));

  // Someone already signed in has nothing to do here.
  useEffect(() => {
    if (isAccount(session) && !finishing.current) {
      router.replace(next);
    }
  }, [session, next, router]);

  // Browsers without passkeys go straight to the password.
  useEffect(() => {
    if (!passkeysSupported()) {
      startTransition(() =>
        dispatch({
          type: "view",
          view: View.Password,
          notice: { tone: NoticeTone.Info, text: t("signIn.passkey.unsupported") },
        }),
      );
    }
  }, []);

  const finish = async (signIn: Session) => {
    finishing.current = true;
    signedIn(signIn);
    const canMakePasskey = await canMakePasskeyHere();
    router.replace(routeAfterSignIn({ session: signIn, next, canMakePasskey }), {
      transitionTypes: [TransitionType.Forward],
    });
  };

  const completePasskey = async ({
    ceremonyId,
    credential,
  }: {
    ceremonyId: string;
    credential: AuthenticationResponse;
  }) => {
    dispatch({ type: "busy", busy: "passkey" });
    const result = await identity.signInWithPasskey({ ceremonyId, credential });
    if (result.kind === "ok") {
      await finish(result.value);
      return;
    }
    if (result.error.code === ERROR_PASSKEY_REJECTED) {
      startTransition(() => dispatch({ type: "passkey_failed" }));
      rearm();
      return;
    }
    dispatch({ type: "notice", notice: { tone: NoticeTone.Error, text: describeError(result.error) } });
    rearm();
  };

  // Offer the device's passkeys in the email field's autofill while the passkey screen shows.
  useEffect(() => {
    if (state.view !== View.Passkey) {
      return;
    }
    const controller = new AbortController();
    autofill.current = some(controller);
    const arm = async () => {
      if (!(await autofillSupported()) || controller.signal.aborted) {
        return;
      }
      const options = await identity.passkeySignInOptions(controller.signal);
      if (options.kind === "error") {
        return;
      }
      const outcome = await getPasskey({
        options: options.value.public_key,
        mediation: Mediation.Autofill,
        signal: controller.signal,
      });
      if (outcome.kind === "ok") {
        await completePasskey({ ceremonyId: options.value.ceremony_id, credential: outcome.credential });
      }
    };
    void arm();
    return () => controller.abort();
    // completePasskey is rebuilt every render; re-arming on it would restart the ceremony.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.view, armed]);

  const signInWithPasskey = async () => {
    if (autofill.current.kind === "some") {
      autofill.current.value.abort();
    }
    dispatch({ type: "busy", busy: "passkey" });
    const options = await identity.passkeySignInOptions();
    if (options.kind === "error") {
      dispatch({ type: "notice", notice: { tone: NoticeTone.Error, text: describeError(options.error) } });
      rearm();
      return;
    }
    const outcome = await getPasskey({
      options: options.value.public_key,
      mediation: Mediation.Modal,
      signal: new AbortController().signal,
    });
    if (outcome.kind === "ok") {
      await completePasskey({ ceremonyId: options.value.ceremony_id, credential: outcome.credential });
      return;
    }
    if (outcome.failure === PasskeyFailure.Unsupported) {
      show(View.Password, { tone: NoticeTone.Info, text: t("signIn.passkey.unsupported") });
      return;
    }
    startTransition(() => dispatch({ type: "passkey_failed" }));
    rearm();
  };

  const continueWithPassword = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (autofill.current.kind === "some") {
      autofill.current.value.abort();
    }
    show(View.Password);
  };

  const signInWithPassword = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    dispatch({ type: "busy", busy: "password" });
    const result = await identity.signInWithPassword({ email: email.trim(), password });
    if (result.kind === "ok") {
      await finish(result.value);
      return;
    }
    if (result.error.code === ERROR_INVALID_CREDENTIALS) {
      dispatch({ type: "wrong_password" });
      return;
    }
    dispatch({ type: "notice", notice: { tone: NoticeTone.Error, text: describeError(result.error) } });
  };

  /** Sends a sign-in link; "" when it went, else what went wrong. */
  const sendLink = async (): Promise<string> => {
    const result = await identity.requestEmailLink(email.trim());
    return result.kind === "ok" ? "" : describeError(result.error);
  };

  const emailMeALink = async () => {
    if (emailField.current && !emailField.current.reportValidity()) {
      return;
    }
    dispatch({ type: "busy", busy: "link" });
    const error = await sendLink();
    if (error === "") {
      show(View.Sent);
      return;
    }
    dispatch({ type: "notice", notice: { tone: NoticeTone.Error, text: error } });
  };

  const notice = state.notice.kind === "some" && (
    <Notice tone={state.notice.value.tone}>{state.notice.value.text}</Notice>
  );

  return (
    <StepTransition step={state.view}>
      {state.view === View.Passkey && (
        <AuthCard eyebrow={t("signIn.eyebrow")} title={t("signIn.title")} lead={t("signIn.passkey.lead")}>
          {notice}
          <button
            type="button"
            className={styles.passkey}
            onClick={signInWithPasskey}
            disabled={state.busy !== "none"}
            aria-busy={state.busy === "passkey"}
          >
            <span className={styles.passkeyIcon} aria-hidden="true">
              <Icon name="passkey" size={26} />
            </span>
            <span className={styles.passkeyText}>
              <strong>{state.busy === "passkey" ? t("signIn.passkey.waiting") : t("signIn.passkey.button")}</strong>
              <span>{t("signIn.passkey.detail")}</span>
            </span>
            <Icon name="arrow" size={18} className={styles.passkeyArrow} />
          </button>
          <p className={styles.divider}>
            <span>{t("signIn.or")}</span>
          </p>
          <form className={styles.form} onSubmit={continueWithPassword}>
            <TextField
              ref={emailField}
              label={t("field.email.label")}
              type="email"
              name="email"
              autoComplete="username webauthn"
              required
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder={t("field.email.placeholder")}
              hint={t("signIn.email.hint")}
            />
            <Button type="submit" variant={ButtonVariant.Secondary} size={ButtonSize.Large} fullWidth withArrow>
              {t("signIn.continueWithPassword")}
            </Button>
          </form>
          <p className={styles.switch}>
            {t("signIn.newHere")}{" "}
            <Link href={withNext({ route: AppRoute.SignUp, next })} transitionTypes={[TransitionType.Forward]}>
              {t("signIn.createAccount")}
            </Link>
          </p>
        </AuthCard>
      )}

      {state.view === View.Password && (
        <AuthCard eyebrow={t("signIn.eyebrow")} title={t("signIn.password.title")} lead={t("signIn.password.lead")}>
          {notice}
          <form className={styles.form} onSubmit={signInWithPassword}>
            <TextField
              ref={emailField}
              label={t("field.email.label")}
              type="email"
              name="email"
              autoComplete="username"
              required
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder={t("field.email.placeholder")}
            />
            <TextField
              label={t("field.password.label")}
              type="password"
              name="password"
              autoComplete="current-password"
              required
              revealable
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              error={state.passwordError.kind === "some" ? state.passwordError.value : undefined}
            />
            <Button
              type="submit"
              size={ButtonSize.Large}
              fullWidth
              withArrow
              busy={state.busy === "password"}
              disabled={state.busy !== "none"}
            >
              {t("signIn.password.submit")}
            </Button>
          </form>
          <div className={styles.alternatives}>
            <Button variant={ButtonVariant.Quiet} onClick={emailMeALink} busy={state.busy === "link"}>
              <Icon name="mail" size={16} />
              {t("signIn.emailLink")}
            </Button>
            {passkeysSupported() && (
              <Button variant={ButtonVariant.Quiet} onClick={() => show(View.Passkey)}>
                <Icon name="passkey" size={16} />
                {t("signIn.tryPasskey")}
              </Button>
            )}
          </div>
        </AuthCard>
      )}

      {state.view === View.Sent && (
        <AuthCard eyebrow={t("signIn.eyebrow")} title={t("inbox.title")}>
          <InboxSent email={email.trim()} onResend={sendLink} onChangeEmail={() => show(View.Password)} />
        </AuthCard>
      )}
    </StepTransition>
  );
}
