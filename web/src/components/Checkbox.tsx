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
    <div className={`check ${className}`}>
      <input className="check__input" style={{ cursor: "pointer" }} type="checkbox" id={id} checked={value} onChange={() => onChange(!value)} />
      <label className="check__label" style={{ cursor: "pointer" }} htmlFor={id}>{children}</label>
    </div>
  );
}
