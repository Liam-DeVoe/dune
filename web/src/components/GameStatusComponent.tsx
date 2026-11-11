// OtherComponents/GameStatusComponent.razor: the prompt card. How long the game has waited, the
// players it waits for (with the worm around the one whose turn it is, or the bidders with their
// card counts during bidding), what the game is waiting for, and "Continue" when the viewer can
// end the phase (the EndPhase action).
//
// The pause between phases (Client.PhaseControl: "continues in 5s", Hold, Done) comes from the
// server's PhaseControl notification, which the lobby client keeps; a game shown without it (a
// saved view) has no pause.
import { useContext, useState, useSyncExternalStore, type CSSProperties } from "react";
import { LobbyContext } from "../lobby/context";
import type { PhaseControlInfo } from "../lobby/types";
import { Checkbox } from "./Checkbox";
import type { Faction } from "../generated/enums";
import type { GameView } from "../generated/GameView";
import type { SequenceEntry } from "../generated/SequenceEntry";
import { useGame } from "../state/context";
import { NumberComponent } from "./NumberComponent";
import { RichText } from "./RichText";
import { secondsSince, since, useClock } from "./turnClock";
import { withBase } from "../base";

const noLobby = () => () => {};
const noState = () => null;

// The current pause, from the lobby client when the game screen runs inside it.
function usePhaseControl(): PhaseControlInfo | null {
  const client = useContext(LobbyContext);
  const state = useSyncExternalStore(client?.subscribe ?? noLobby, client?.getState ?? noState);
  return state?.game?.phaseControl ?? null;
}

export function GameStatusComponent() {
  const { view, skin, source } = useGame();
  const s = view.status;
  const endPhase = view.actions.find((a) => a.eventType === "EndPhase");
  const [error, setError] = useState<string | null>(null);
  const clock = useClock();
  const control = usePhaseControl();
  const pause = control?.Active ? control : null;
  const bySeat = (seats: number[]) => seats.map((seat) => view.players.find((p) => p.seat === seat)).filter((p) => p !== undefined);
  const mySeat = view.viewer.isObserver ? undefined : view.players.find((p) => p.faction === view.viewer.faction)?.seat;
  // WaitingForPlayers: the pause's when there is one.
  const waitingFor: Faction[] = pause ? bySeat(pause.WaitingFor).map((p) => p.faction) : s.waitingFor;
  const amWaitedFor = mySeat !== undefined && !!pause?.WaitingFor.includes(mySeat);
  const amHolding = mySeat !== undefined && !!pause?.Holding.includes(mySeat);
  const holders = bySeat(pause?.Holding ?? []);
  // OnlyBotsPending: the pause waits only for bots, which end it themselves.
  const onlyBotsPending = !!pause && pause.CountdownEnds === null && pause.Holding.length === 0
    && waitingFor.every((f) => view.players.find((p) => p.faction === f)?.isBot);
  const canHold = mySeat !== undefined && !!pause && pause.CountdownEnds !== null && !amHolding;
  const isBidding = view.mainPhase === "Bidding";
  const showRow = isBidding || waitingFor.length > 0 || s.waitingInSequence.length > 0;
  // StatusStyle: a faint red text border while the viewer is awaited.
  const statusStyle: CSSProperties | undefined = (pause ? amWaitedFor : s.waitingForMe)
    ? { WebkitTextStroke: "0.3px rgba(195,50,50,.09)", textShadow: "0 0 2px rgba(195,50,50,.09)" }
    : undefined;

  const pauseMessage = () => {
    const phase = skin.enumValue("MainPhase", view.mainPhase);
    if (amWaitedFor) return `You may still act before the ${phase} phase continues.`;
    if (onlyBotsPending) return `The ${phase} phase continues...`;
    if (pause!.WaitingFor.length > 0) return `Waiting for players before the ${phase} phase continues...`;
    if (pause!.CountdownEnds !== null) {
      const seconds = Math.max(0, Math.ceil((Date.parse(pause!.CountdownEnds) - clock) / 1000));
      return `The ${phase} phase continues in ${seconds}s.`;
    }
    return pause!.Holding.length > 0 ? `Waiting for the hold before the ${phase} phase continues...` : `The ${phase} phase continues...`;
  };
  const call = (method: string, ...args: unknown[]) => { void source.gameCall?.(method, ...args); };

  return (
    <div className="card p-0 mb-2 prompt-act" style={{ animation: "fadeIn 1s" }}>
      <div className="card-body bg-transparent border-bottom-0">
        {s.timedEventTime !== null && <Timer view={view} from={s.timedEventTime} />}
        {showRow && (
          <div className="row row-cols-6 d-flex justify-content-center mt-2 mb-3">
            <Players view={view} waitingFor={waitingFor} />
          </div>
        )}
        {pause
          ? (
            <>
              <div className="text-dark fw-bold text-center" style={statusStyle}>{pauseMessage()}</div>
              {holders.length > 0 && <div className="text-dark text-center">Held by {holders.map((p) => p.name).join(", ")}</div>}
              <div className="mt-2 text-end">
                {view.viewer.isHost && !onlyBotsPending && (
                  <button className="btn btn-outline-secondary me-1" onClick={() => call("RequestPhaseContinue", view.eventNumber)} title="Continue without waiting">Continue now</button>
                )}
                {canHold && <button className="btn btn-outline-secondary me-1" onClick={() => call("RequestPhaseHold")}>Hold</button>}
                {(amWaitedFor || amHolding) && (
                  <button className="btn btn-primary" onClick={() => call(amWaitedFor ? "RequestPhaseDone" : "RequestPhaseRelease")}>Done</button>
                )}
              </div>
            </>
          )
          : (
            <>
              <div className="text-dark fw-bold text-center" style={statusStyle}>
                <RichText parts={s.message} size={20} />
              </div>
              {endPhase && (
                <div className="mt-2 text-end">
                  <button className="btn btn-primary" disabled={endPhase.validation !== null}
                          title={error ?? undefined}
                          onClick={async () => setError(await source.submit({ $type: "EndPhase" }))}>Continue</button>
                </div>
              )}
            </>
          )}
        {view.viewer.isHost && isBidding && view.players.filter((p) => !p.isBot).length > 1 && <TimeLimit view={view} />}
      </div>
    </div>
  );
}

// The host's bidding time limit (SetTimer). Steps of 5 up to 20 seconds, 10 up to 60, then 30.
function TimeLimit({ view }: { view: GameView }) {
  const { source } = useGame();
  const [limit, setLimit] = useState(20);
  const set = view.timing.timer > 0;
  const update = (value: number) => { void source.gameCall?.("SetTimer", value); };
  const up = limit < 20 ? 5 : limit < 60 ? 10 : 30;
  const down = limit > 60 ? 30 : limit > 20 ? 10 : 5;
  const change = (v: number) => { const c = Math.min(600, Math.max(10, v)); setLimit(c); update(c); };
  return (
    <div className="mt-1">
      <Checkbox value={set} onChange={(v: boolean) => update(v ? limit : -1)}>Time limit (seconds)</Checkbox>
      {set && (
        <span className="num">
          <button onClick={() => change(limit - down)}>−</button>
          <input value={limit} readOnly />
          <button onClick={() => change(limit + up)}>+</button>
        </span>
      )}
    </div>
  );
}

// TimerText: the time since the timed event, once a second has passed, with the bidding time
// limit during bidding.
function Timer({ view, from }: { view: GameView; from: string }) {
  const clock = useClock();
  let text = "";
  if (secondsSince(clock, from) >= 1) {
    text = since(clock, from);
    if (view.phase === "Bidding" && view.timing.timer > 0) text += ` (limit: ${view.timing.timer}s)`;
  }
  const color = view.status.waitingForMe ? "var(--storm)" : "var(--ink-1)";
  return <div style={{ position: "absolute", left: "5px", top: 0, color, fontWeight: "bolder" }}>{text}</div>;
}

function Players({ view, waitingFor }: { view: GameView; waitingFor: Faction[] }) {
  const { skin } = useGame();
  const s = view.status;
  const player = (f: Faction) => view.players.find((p) => p.faction === f);
  const name = (f: Faction) => player(f)?.name ?? "";
  const shadow = "drop-shadow(2px 2px 2px black)";

  if (view.mainPhase === "Bidding" && s.toBid.length > 0) {
    const showTurnInSequence = view.phase === "BlackMarketBidding" || view.phase === "Bidding" || view.phase === "BiddingReport";
    const hasTurn = waitingFor.length === 1 && !showTurnInSequence ? waitingFor[0] : null;
    return s.toBid.map((se: SequenceEntry) => {
      const p = player(se.faction);
      if (!p) return null;
      const big = se.faction === hasTurn || (showTurnInSequence && se.hasTurn);
      return (
        <div key={se.faction} className="col" style={{ filter: "drop-shadow(1px 1px 1px black)" }}>
          <NumberComponent value={p.cardsOwned} size={big ? 38 : 32} symbolSrc={skin.factionImage(se.faction)}
            behindNumberSrc={skin.url(skin.str("CardBack_TreacheryCard_URL"))} title={p.name}
            numberColor={p.hasRoomForCards ? "black" : "red"} numberBorderColor="white" numberBorderWidth={1}
            numberAlignment="BottomRight" numberBackgroundFillColor="white" numberBackgroundBorderColor="black" />
        </div>
      );
    });
  }

  if (waitingFor.length === 1) {
    const f = waitingFor[0];
    return (
      <div style={{ position: "relative", width: "64px", height: "64px" }}>
        <img title={name(f)} src={skin.factionImage(f)} style={{ position: "absolute", left: 0, filter: shadow, width: "64px" }} alt={name(f)} />
        <img src={withBase("art/worm.gif")} style={{ position: "absolute", top: "-14px", left: "-20px", filter: "drop-shadow(1px 1px 1px black)", width: "94px" }} alt="Worm" />
      </div>
    );
  }

  if (waitingFor.length > 1) {
    return waitingFor.map((f) => (
      <div key={f} style={{ position: "relative", width: "48px" }}>
        <img className="p-2" title={name(f)} src={skin.factionImage(f)} style={{ filter: shadow, width: "48px" }} alt={name(f)} />
      </div>
    ));
  }

  return s.waitingInSequence.map((se) => (
    <div key={se.faction} className="align-self-center p-0 m-0" style={{ position: "relative", width: "64px" }}>
      {se.hasTurn
        ? (
          <>
            <img title={name(se.faction)} src={skin.factionImage(se.faction)} style={{ position: "relative", filter: shadow, width: "64px" }} alt={name(se.faction)} />
            <img src={withBase("art/worm.gif")} style={{ position: "absolute", top: "-12px", left: "-18px", filter: "drop-shadow(1px 1px 1px black)", width: "90px" }} alt="Worm" />
          </>
        )
        : <img className="align-self-center p-2" title={name(se.faction)} src={skin.factionImage(se.faction)} width="48" style={{ position: "relative", filter: shadow }} alt={name(se.faction)} />}
    </div>
  ));
}
