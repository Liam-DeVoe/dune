// OtherComponents/DealComponent.razor: the viewer's deals. Offers made to them (Accept, Decline),
// their own offers with each recipient's answer (Withdraw), and every deal in force. The events
// to submit come built in the view (`deals`).
import "./DealComponent.css";
import { useState } from "react";
import { useGame } from "../state/context";
import { CollapseButton } from "./CollapseButton";
import { answeredDeals } from "./DealToasts";
import { FactionChip } from "./FactionChip";
import { useMenu } from "./menuState";
import { RichText } from "./RichText";
import { SpiceComponent } from "./SpiceComponent";

// `open`: opened up, as when shown in a popover from the menu bar (InPopover).
export function DealComponent({ open = false }: { open?: boolean }) {
  const { view, skin, source } = useGame();
  const m = useMenu();
  const [collapsed, setCollapsed] = useState(!open);
  const d = view.deals;
  if (!d) return null;
  const submit = (e: Record<string, unknown> | null | undefined, answering?: number) => {
    if (!e) return;
    if (answering !== undefined) answeredDeals.add(answering);
    void source.submit(e as { $type: string });
  };
  return (
    <div className="card mb-2" style={open ? undefined : { animation: "fadeIn 1s" }}>
      <div className="card-header" onClick={() => setCollapsed(!collapsed)} style={{ cursor: "pointer" }}>
        Deals{" "}
        {d.acceptable.length > 0 && <><span className="badge bg-warning" title="Deals offered to you">{d.acceptable.length}</span>{" "}</>}
        {d.cancellable.length > 0 && <><span className="badge bg-success" title="Your deal offers">{d.cancellable.length}</span>{" "}</>}
        {d.current.length > 0 && <><span className="badge bg-primary" title="Current deals">{d.current.length}</span>{" "}</>}
        <CollapseButton collapsed={collapsed} />
      </div>
      {!collapsed && (
        <div className="card-body">
          {!view.viewer.isObserver && (d.acceptable.length > 0 || d.cancellable.length > 0) && (
            <>
              <label>Deal offers</label>
              <table className="table table-sm">
                <tbody>
                  {d.acceptable.map((o) => (
                    <tr key={`a${o.id}`} className={`table-dark ${m.markedDeal === o.id ? "deal-marked" : ""}`}>
                      <td><FactionChip faction={o.initiator} /></td>
                      <td><SpiceComponent amount={String(o.price)} /></td>
                      <td className="text-break"><RichText parts={o.description} /></td>
                      <td>
                        <button className="btn btn-primary btn-sm" onClick={() => submit(o.accept, o.id)} disabled={o.acceptError !== null}>Accept</button>{" "}
                        <button className="btn btn-secondary btn-sm ms-1" onClick={() => submit(o.decline, o.id)}>Decline</button>
                      </td>
                    </tr>
                  ))}
                  {d.cancellable.map((o) => (
                    <tr key={`c${o.id}`} className="table-dark">
                      <td><FactionChip faction={o.initiator} /></td>
                      <td><SpiceComponent amount={String(o.price)} /></td>
                      <td className="text-break">
                        <RichText parts={o.description} />
                        <div className="deal-answers">
                          {o.answers.map((a) => {
                            const answer = a.accepted === true ? "accepted" : a.accepted === false ? "declined" : "deciding";
                            return (
                              <span key={a.faction} className={`deal-answer ${answer}`} title={`${skin.faction(a.faction)}: ${answer}`}>
                                <img width="18" height="18" src={skin.factionImage(a.faction)} alt={skin.faction(a.faction)} />{answer}
                              </span>
                            );
                          })}
                          {o.answers.length > 0 && o.answers.every((a) => a.accepted === false) && (
                            <span className="deal-answer-none">Declined by everyone</span>
                          )}
                        </div>
                      </td>
                      <td>
                        <button className="btn btn-secondary btn-sm" onClick={() => submit(o.withdraw)}>Withdraw</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
          {d.current.length > 0 && (
            <>
              <label>Current deals</label>
              <table className="table table-sm">
                <tbody>
                  {d.current.map((deal, i) => (
                    <tr key={i} className="table-dark">
                      <td className="p-0">
                        <FactionChip faction={deal.bound} />{" ⇒ "}<FactionChip faction={deal.consuming} />
                      </td>
                      <td className="text-break"><RichText parts={deal.description} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
        </div>
      )}
    </div>
  );
}
