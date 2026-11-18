// GenericComponents/ButtonRowComponent.razor: Other, Pass and Confirm, in that order. The
// validation message (ConfirmError) disables only the confirm button. The row sits at the right of
// the form unless `alignEnd` is false.
import type { Part } from "../generated/Part";

export function ButtonRow({ confirm, confirmText = "Confirm", confirmError = null, pass, passText = "Pass", other, otherText = "Other",
  alignEnd = true, disabled = false }: {
  confirm?: () => void;
  confirmText?: string;
  confirmError?: Part[] | null;
  pass?: () => void;
  passText?: string;
  other?: () => void;
  otherText?: string;
  alignEnd?: boolean;
  disabled?: boolean;
}) {
  return (
    <div className={alignEnd ? "button-row button-row--end" : "button-row"}>
      {other && <button className="btn btn-primary button-row__button" onClick={other} disabled={disabled}>{otherText}</button>}
      {pass && <button className="btn btn-primary button-row__button" onClick={pass} disabled={disabled}>{passText}</button>}
      {confirm && <button className="btn btn-primary" onClick={confirm} disabled={disabled || confirmError != null}>{confirmText}</button>}
    </div>
  );
}
