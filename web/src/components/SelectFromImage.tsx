// GenericComponents/SelectFromImageComponent.razor, single choice: a row of images, the chosen
// one lit, the rest darkened. With `required`, a value not among the choices becomes the first
// choice; without, clicking the chosen image clears it.
import { useEffect, type ReactNode } from "react";
import { Image } from "./Image";

export interface ImageChoice<T> {
  value: T;
  src: string;
  alt: string;
  popover: string | null;
  // Image.razor's tooltip (title) for values without a rich popover, such as a faction.
  tooltip?: string | null;
}

export function SelectFromImage<T>({ values, value, onChange, required = true, imageWidth = 60, label, disabled = false }: {
  values: ImageChoice<T>[];
  value: T | null;
  onChange: (v: T | null) => void;
  required?: boolean;
  imageWidth?: number;
  label?: ReactNode;
  disabled?: boolean;
}) {
  useEffect(() => {
    if (required && !values.some((c) => c.value === value)) onChange(values[0]?.value ?? null);
  });
  const click = (item: T) => {
    if (disabled) return;
    if (item === value) {
      if (!required) onChange(null);
    } else {
      onChange(item);
    }
  };
  return (
    <>
      {label != null && <label className="d-block mt-1 mb-0">{label}</label>}
      <div className="img-choices">
        {values.map((c) => (
          <Image key={String(c.value)} src={c.src} alt={c.alt} width={imageWidth} popover={c.popover} tooltip={c.tooltip ?? null} pointer highlightOnHover
                 darkened={c.value !== value} onClick={() => click(c.value)} />
        ))}
      </div>
    </>
  );
}
