// GenericComponents/CheckboxComponent.razor.
import { useId, type ReactNode } from "react";

export function Checkbox({ value, onChange, className = "", children }: {
  value: boolean;
  onChange: (v: boolean) => void;
  className?: string;
  children: ReactNode;
}) {
  const id = useId();
  return (
    <div className={`form-check ${className}`}>
      <input className="form-check-input" style={{ cursor: "pointer" }} type="checkbox" id={id} checked={value} onChange={() => onChange(!value)} />
      <label className="form-check-label" style={{ cursor: "pointer" }} htmlFor={id}>{children}</label>
    </div>
  );
}
