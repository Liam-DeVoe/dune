// GenericComponents/ModalComponent.razor: a Bootstrap modal, opened by a button with
// data-bs-toggle="modal" and data-bs-target="#<id>". Bootstrap's own script (index.html) shows
// and hides it, as in C#; React never changes the classes it sets, because the props stay the same.
import type { ReactNode } from "react";

export function ModalComponent({ id, title, size = "", scrolling = false, printable = false, children }: {
  id: string;
  title: string;
  size?: string; // modal-sm, modal-lg, modal-xl
  scrolling?: boolean;
  printable?: boolean;
  children: ReactNode;
}) {
  const labelId = `${id}Label`;
  const bodyId = `${id}Body`;
  return (
    <div className="modal fade" id={id} tabIndex={-1} role="dialog" aria-labelledby={labelId} aria-hidden="true">
      <div className={`modal-dialog ${size}`} role="document">
        <div className="modal-content">
          <div className="modal-header">
            <h5 className="modal-title" id={labelId}>{title}</h5>
            {" "}
            {printable && (
              <button className="btn btn-link p-0" type="button" onClick={() => print(bodyId)}>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" className="bi bi-printer" viewBox="0 0 16 16">
                  <path d="M2.5 8a.5.5 0 1 0 0-1 .5.5 0 0 0 0 1z" />
                  <path d="M5 1a2 2 0 0 0-2 2v2H2a2 2 0 0 0-2 2v3a2 2 0 0 0 2 2h1v1a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2v-1h1a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-1V3a2 2 0 0 0-2-2H5zM4 3a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2H4V3zm1 5a2 2 0 0 0-2 2v1H2a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-1v-1a2 2 0 0 0-2-2H5zm7 2v3a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-3a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1z" />
                </svg>
              </button>
            )}
            <button type="button" className="btn-close" data-bs-dismiss="modal" aria-label="Close" />
          </div>
          <div className={`modal-body ${scrolling ? "overflow-auto" : ""}`} id={bodyId}>
            {children}
          </div>
        </div>
      </div>
    </div>
  );
}

// Browser.Print (wwwroot/index.html's Print): a copy of the element in #printSection, which the
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
