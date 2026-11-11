// SimpleActionComponent and YesNoActionComponent (package 7e), with the arguments
// ActionPanel.razor gives them: the server sends the title, labels and alert per event type
// (rust/view/src/actions/other.rs).
import { useEffect, useRef, useState } from "react";
import type { Part } from "../../generated/Part";
import { useGame } from "../../state/context";
import { ButtonRow } from "../ButtonRow";
import { Checkbox } from "../Checkbox";
import { RichText } from "../RichText";
import { useViewChanges } from "../formDraft";
import type { FormProps } from "./ChoiceForms";
import { Layout, SubmitError, useOtherDraft } from "./parts";

// A plain `Title` is text in the header; a `TitleMessage` is drawn by ExpressionComponent (Size 20).
function Title({ parts }: { parts: Part[] }) {
  if (parts.every((p) => p.t === "text")) return <>{parts.map((p) => (p.t === "text" ? p.v : "")).join("")}</>;
  return <RichText parts={parts} size={20} />;
}

// SimpleActionComponent.AutoConfirmSettings: "Next time, auto ... after 3 seconds", per event type
// for the session.
const autoConfirm = new Map<string, boolean>();

export function SimpleForm({ action, inPopover = false }: FormProps) {
  const { source } = useGame();
  const [dismissed, setDismissed] = useState(false);
  const [auto, setAuto] = useState(() => autoConfirm.get(action.eventType) ?? false);
  const [error, setError] = useState<string | null>(null);
  // A new view clears a refusal. Dismissed stays dismissed while the form is on screen, as the C#
  // field does (formDraft.ts).
  useViewChanges(() => setError(null));
  const o = action.options?.form === "Simple" ? action.options : null;
  const confirm = async () => setError(await source.submit({ $type: action.eventType }));
  // OnInitialized: with the box ticked, confirm after 5 seconds if the box is still ticked and the
  // event is valid then (AutoPass).
  const latest = useRef({ action, auto });
  latest.current = { action, auto };
  useEffect(() => {
    if (!o?.allowAutoConfirm || !auto) return;
    const timer = setTimeout(() => { if (latest.current.auto && latest.current.action.validation == null) void confirm(); }, 5000);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  if (!o || dismissed) return null;
  return (
    <Layout type={`Simple:${action.eventType}`} header={o.title ? <Title parts={o.title} /> : null} urgent={o.alert} inPopover={inPopover}>
      {o.dismissLabel != null
        ? <ButtonRow confirm={confirm} confirmText={o.confirmLabel} confirmError={action.validation} other={() => setDismissed(true)} otherText={o.dismissLabel} />
        : <ButtonRow confirm={confirm} confirmText={o.confirmLabel} confirmError={action.validation} />}
      {o.allowAutoConfirm && (
        <div className="mt-0 float-end">
          <Checkbox value={auto} onChange={(v) => { setAuto(v); autoConfirm.set(action.eventType, v); }}>
            Next time, auto {o.confirmLabel} after 3 seconds
          </Checkbox>
        </div>
      )}
      <SubmitError error={error} />
    </Layout>
  );
}

export function YesNoForm({ action, inPopover = false }: FormProps) {
  const { options: o, validation, submit, error } = useOtherDraft(action, "YesNo");
  if (!o) return null;
  return (
    <Layout type={`YesNo:${action.eventType}`} header={<Title parts={o.title} />} urgent={o.alert} inPopover={inPopover}>
      <ButtonRow confirm={() => submit({ Passed: false })} confirmText="Yes" confirmError={validation} pass={() => submit({ Passed: true })} passText="No" />
      <SubmitError error={error} />
    </Layout>
  );
}
