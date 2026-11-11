// GenericComponents/PickFromImageComponent.razor: an image (or a "?" disc) that opens a modal of
// images to pick from. The first entry of the modal is the "?" disc, which picks nothing.
import { useId, type ReactNode } from "react";
import { supportRound } from "../text";
import { Image } from "./Image";
import { ModalComponent } from "./ModalComponent";

export interface PickItem<K> {
  key: K;
  src: string;
  alt: string;
  popover: string | null;
}

function hideModal(id: string) {
  const el = document.getElementById(id);
  const bs = (window as unknown as { bootstrap?: { Modal: { getInstance(e: Element): { hide(): void } | null } } }).bootstrap;
  if (el && bs) bs.Modal.getInstance(el)?.hide();
}

function Unknown({ width }: { width: number }) {
  const half = supportRound(Math.fround(0.5 * width));
  return (
    <svg width={width} height={width}>
      <circle cx={half} cy={half} r={half} style={{ fill: "var(--parchment)" }} />
      <text textAnchor="middle" alignmentBaseline="middle" fontSize={`${half}px`} fontWeight="bold" x={half} y={supportRound(width / 1.5)} fontStyle="italic">?</text>
    </svg>
  );
}

export function PickFromImageComponent<K>({ value, values, onChanged, title, imageWidth = 60, placeholder }: {
  value: PickItem<K> | null;
  values: PickItem<K>[];
  onChanged: (key: K | null) => void;
  title: string;
  imageWidth?: number;
  placeholder?: ReactNode;
}) {
  const id = "picker" + useId().replace(/[^a-zA-Z0-9]/g, "");
  // Picking the chosen value again clears it (Required is false wherever this is used).
  const click = (k: K | null) => { onChanged(value !== null && k === value.key ? null : k); hideModal(id); };
  return (
    <>
      <ModalComponent id={id} title={title}>
        <div className="row row-cols-3 row-cols-md-6 m-0">
          <div className="card p-1 m-0 border-0">
            <div className="card-img-top" onClick={() => click(null)} style={{ cursor: "pointer" }}>
              {placeholder ?? <Unknown width={imageWidth} />}
            </div>
          </div>
          {values.map((v, i) => (
            <div key={i} className="card p-1 m-0 border-0">
              <div className="card-img-top">
                <Image className="card-img" src={v.src} alt={v.alt} width={0} popover={v.popover} onClick={() => click(v.key)}
                       pointer highlightOnHover darkened={value === null || value.key !== v.key} />
              </div>
            </div>
          ))}
        </div>
      </ModalComponent>
      <a className="btn btn-link p-1" data-bs-toggle="modal" data-bs-target={`#${id}`}>
        {value !== null
          ? <Image src={value.src} alt={value.alt} width={imageWidth} popover={value.popover} pointer />
          : placeholder ?? <Unknown width={imageWidth} />}
      </a>
    </>
  );
}
