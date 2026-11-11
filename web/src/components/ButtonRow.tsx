// GenericComponents/ButtonRowComponent.razor: Other, Pass and Confirm, in that order. The
// validation message (ConfirmError) disables only the confirm button.
import type { Part } from "../generated/Part";

export function ButtonRow({ confirm, confirmText = "Confirm", confirmError = null, pass, passText = "Pass", other, otherText = "Other",
  alignment = "text-end", disabled = false }: {
  confirm?: () => void;
  confirmText?: string;
  confirmError?: Part[] | null;
  pass?: () => void;
  passText?: string;
  other?: () => void;
  otherText?: string;
  alignment?: string;
  disabled?: boolean;
}) {
  return (
    <div className={`mt-2 ${alignment}`}>
      {other && <button className="btn btn-primary me-1" onClick={other} disabled={disabled}>{otherText}</button>}
      {pass && <button className="btn btn-primary me-1" onClick={pass} disabled={disabled}>{passText}</button>}
      {confirm && <button className="btn btn-primary" onClick={confirm} disabled={disabled || confirmError != null}>{confirmText}</button>}
    </div>
  );
}
