// A modal <dialog>, opened with openModal(id) by the button that shows it. Escape, the close
// button and a click beside the box close it, with a short fade (components/_modal.scss). The
// browser keeps focus inside while it is open and gives it back to the element that had it when it
// closes.
import { useRef, type ReactNode } from "react";
import { hidePopoversIn } from "../popover";

const FADE = 150; // ms, the closing fade (.modal--closing)

function dialog(id: string): HTMLDialogElement | null {
  const d = document.getElementById(id);
  return d instanceof HTMLDialogElement ? d : null;
}

export function openModal(id: string): void {
  const d = dialog(id);
  if (!d || d.open) return;
  d.showModal();
  // The dialog itself, not its first button, as Bootstrap's modal did.
  d.focus();
}

export function closeModal(id: string): void {
  const d = dialog(id);
  if (d) hide(d);
}

function hide(d: HTMLDialogElement) {
  if (!d.open || d.classList.contains("modal--closing")) return;
  hidePopoversIn(d);
  d.classList.add("modal--closing");
  window.setTimeout(() => d.close(), FADE);
}

export function ModalComponent({ id, title, size, scrolling = false, printable = false, children }: {
  id: string;
  title: string;
  size?: "small" | "large" | "xlarge";
  scrolling?: boolean;
  printable?: boolean;
  children: ReactNode;
}) {
  const labelId = `${id}Label`;
  const bodyId = `${id}Body`;
  // A click closes the modal only when it both starts and ends beside the box.
  const pressedOutside = useRef(false);
  return (
    <dialog className="modal" id={id} role="dialog" tabIndex={-1} aria-labelledby={labelId}
            onCancel={(e) => { e.preventDefault(); hide(e.currentTarget); }}
            onClose={(e) => e.currentTarget.classList.remove("modal--closing")}
            onMouseDown={(e) => { pressedOutside.current = e.target === e.currentTarget; }}
            onClick={(e) => { if (pressedOutside.current && e.target === e.currentTarget) hide(e.currentTarget); }}>
      <div className={size ? `modal__dialog modal__dialog--${size}` : "modal__dialog"}>
        <div className="modal__content">
          <div className="modal__header">
            <h5 className="modal__title" id={labelId}>{title}</h5>
            {"\u00a0"}
            {printable && (
              <button className="button button--link modal-component__print" type="button" onClick={() => print(bodyId)}>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                  <path d="M2.5 8a.5.5 0 1 0 0-1 .5.5 0 0 0 0 1z" />
                  <path d="M5 1a2 2 0 0 0-2 2v2H2a2 2 0 0 0-2 2v3a2 2 0 0 0 2 2h1v1a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2v-1h1a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-1V3a2 2 0 0 0-2-2H5zM4 3a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2H4V3zm1 5a2 2 0 0 0-2 2v1H2a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-1v-1a2 2 0 0 0-2-2H5zm7 2v3a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-3a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1z" />
                </svg>
              </button>
            )}
            <button type="button" className="close-button" aria-label="Close" onClick={() => closeModal(id)} />
          </div>
          <div className={`modal__body ${scrolling ? "modal-component__body--scroll" : ""}`} id={bodyId}>
            {children}
          </div>
        </div>
      </div>
    </dialog>
  );
}

// Printing: a copy of the element in #printSection, which the
// stylesheet's print rules show alone, then the browser's print dialog.
function print(elementId: string) {
  const elem = document.getElementById(elementId);
  if (!elem) return;
  let section = document.getElementById("printSection");
  if (!section) {
    section = document.createElement("div");
    section.id = "printSection";
    document.body.appendChild(section);
  }
  section.innerHTML = "";
  section.appendChild(elem.cloneNode(true));
  window.print();
}
