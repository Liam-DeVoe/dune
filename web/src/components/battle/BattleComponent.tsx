// GameEventComponents/BattleComponent.razor: the battle plan. Forces are a row of tokens per
// force type, each out, at half strength or at full strength; the dial and spice steppers move
// the tokens through the affordable mixes the server lists (BattleOptions.mixes, the C#
// component's Mixes). Leader, weapon and defense choices come from the server for the draft, as
// does the leader's part of the strength.
import { useEffect, useRef, useState, type ReactNode } from "react";
import type { ActionView } from "../../generated/ActionView";
import type { BattleNote } from "../../generated/BattleNote";
import type { BattleSide } from "../../generated/BattleSide";
import type { Faction } from "../../generated/enums";
import { treacheryCardPopup } from "../../popups";
import { useGame } from "../../state/context";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { Image } from "../Image";
import { NumberPicker } from "../NumberPicker";
import { PlaceChip } from "../PlaceChip";
import { RichText } from "../RichText";
import { heroImage, heroPopup, idOrNull, Layout, Radio, SubmitError, useBattleDraft } from "./parts";

type Commit = "out" | "half" | "full";
type Mix = [number, number, number, number, number, number]; // fs, hs, fn, hn, dial, cost

const count = (row: Commit[], c: Commit) => row.filter((x) => x === c).length;

function makeRow(length: number, full: number, half: number): Commit[] {
  return Array.from({ length }, (_, i) => (i < full ? "full" : i < full + half ? "half" : "out"));
}

function resize(row: Commit[], length: number): Commit[] {
  return row.length === length ? row : Array.from({ length }, (_, i) => (i < row.length ? row[i] : "out"));
}

// Change a row to hold these counts, touching only as many tokens as it must. Tokens that join
// the battle are taken from the left, tokens that leave from the right, and the freed places are
// filled full, then half, then out, left to right.
function assign(row: Commit[], full: number, half: number): Commit[] {
  const want: Record<Commit, number> = { full, half, out: row.length - full - half };
  const result = [...row];
  const freed: number[] = [];
  for (const state of ["full", "half", "out"] as Commit[]) {
    const places = row.map((c, i) => (c === state ? i : -1)).filter((i) => i >= 0);
    const surplus = places.length - want[state];
    if (surplus <= 0) continue;
    freed.push(...(state === "out" ? places.slice(0, surplus) : places.slice(places.length - surplus)));
  }
  const fill = (["full", "half", "out"] as Commit[]).flatMap((s) => Array<Commit>(Math.max(0, want[s] - count(row, s))).fill(s));
  freed.sort((a, b) => a - b).forEach((place, i) => { if (i < fill.length) result[place] = fill[i]; });
  return result;
}

const distinctSorted = (xs: number[]) => [...new Set(xs)].sort((a, b) => a - b);

// The first element with the smallest distance (Enumerable.MinBy).
function minBy(xs: number[], target: number): number {
  return xs.reduce((best, x) => (Math.abs(x - target) < Math.abs(best - target) ? x : best), xs[0]);
}

// float.ToString("0.##").
function formatStrength(v: number): string {
  return String(Math.round(v * 100) / 100);
}

function parseStrength(text: string): number | null {
  let t = text.trim().replace("½", ".5");
  if (t.startsWith(".")) t = "0" + t;
  const n = Number.parseFloat(t);
  return Number.isNaN(n) ? null : n;
}

export function BattleComponent({ action, inPopover }: { action: ActionView; inPopover?: boolean }) {
  const { skin, view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "Battle");
  const [normal, setNormal] = useState<Commit[]>([]);
  const [special, setSpecial] = useState<Commit[]>([]);
  const started = useRef(false);

  // The rows start from the draft (the previous plan, when there is one), as C#'s OnInitialized.
  // Later they keep their order across views, like the component's fields, and only follow the
  // draft when the server's answer changed its counts.
  useEffect(() => {
    if (!o) return;
    const counts = [draft.Forces, draft.ForcesAtHalfStrength, draft.SpecialForces, draft.SpecialForcesAtHalfStrength].map((v) => Number(v ?? 0));
    if (!started.current) {
      started.current = true;
      setNormal(makeRow(o.normalTokens, counts[0], counts[1]));
      setSpecial(makeRow(o.specialTokens, counts[2], counts[3]));
      return;
    }
    const follow = (row: Commit[], length: number, full: number, half: number) => {
      const r = resize(row, length);
      return (count(r, "full") === full && count(r, "half") === half) || full + half > length ? null : assign(r, full, half);
    };
    const nr = follow(normal, o.normalTokens, counts[0], counts[1]);
    const sr = follow(special, o.specialTokens, counts[2], counts[3]);
    if (nr) setNormal(nr);
    if (sr) setSpecial(sr);
  });

  if (!o) return null;
  const n = resize(normal, o.normalTokens);
  const s = resize(special, o.specialTokens);
  const mixes = o.mixes as Mix[];
  const spiceAvailable = o.resources + o.allyCap;

  const dialOf = (fs: number, hs: number, fn: number, hn: number) => mixes.find((m) => m[0] === fs && m[1] === hs && m[2] === fn && m[3] === hn)?.[4] ?? 0;
  const costOf = (fs: number, fn: number) => mixes.find((m) => m[0] === fs && m[2] === fn)?.[5] ?? Number.POSITIVE_INFINITY;
  const dialOfRows = (sr: Commit[], nr: Commit[]) => dialOf(count(sr, "full"), count(sr, "half"), count(nr, "full"), count(nr, "half"));
  const costOfRows = (sr: Commit[], nr: Commit[]) => costOf(count(sr, "full"), count(nr, "full"));
  const dial = dialOfRows(s, n);
  const cost = costOfRows(s, n);
  const allDials = distinctSorted(mixes.map((m) => m[4]));
  const allCosts = distinctSorted(mixes.map((m) => m[5]));

  // The spice the ally pays, between what the player can't pay and what the ally can.
  const allyRange = (c: number) => [Math.max(0, c - o.resources), Math.min(c, o.allyCap)] as const;
  const clampAlly = (value: number, c: number) => {
    const [min, max] = allyRange(c);
    return max < min ? max : Math.min(Math.max(value, min), max);
  };

  const setRows = (sr: Commit[], nr: Commit[]) => {
    setSpecial(sr);
    setNormal(nr);
    const c = costOfRows(sr, nr);
    void update({
      Forces: count(nr, "full"), ForcesAtHalfStrength: count(nr, "half"),
      SpecialForces: count(sr, "full"), SpecialForcesAtHalfStrength: count(sr, "half"),
      AllyContributionAmount: clampAlly(draft.AllyContributionAmount as number, Number.isFinite(c) ? c : 0),
    });
  };

  // Reach a dial and cost by changing as few tokens as possible. Ties keep special forces out,
  // then commit fewer forces.
  const moveTo = (d: number, c: number) => {
    const fs = count(s, "full"), hs = count(s, "half"), fn = count(n, "full"), hn = count(n, "half");
    const changes = (len: number, f: number, h: number, f2: number, h2: number) => len - Math.min(f, f2) - Math.min(h, h2) - Math.min(len - f - h, len - f2 - h2);
    const candidates = mixes.filter((m) => m[4] === d && m[5] === c).map((m, i) => ({ m, i,
      key: [changes(s.length, fs, hs, m[0], m[1]) + changes(n.length, fn, hn, m[2], m[3]), m[0] + m[1], m[0] + m[1] + m[2] + m[3]] }));
    candidates.sort((a, b) => a.key[0] - b.key[0] || a.key[1] - b.key[1] || a.key[2] - b.key[2] || a.i - b.i);
    const best = candidates[0]?.m;
    if (best) setRows(assign(s, best[0], best[1]), assign(n, best[2], best[3]));
  };
  const moveToDial = (d: number) => {
    const costs = distinctSorted(mixes.filter((m) => m[4] === d).map((m) => m[5]));
    if (costs.length > 0) moveTo(d, costs.includes(cost) ? cost : minBy(costs, cost));
  };
  const moveToCost = (c: number) => {
    const ds = distinctSorted(mixes.filter((m) => m[5] === c).map((m) => m[4]));
    if (ds.length > 0) moveTo(ds.includes(dial) ? dial : minBy(ds, dial), c);
  };
  const stepDial = (dir: number) => {
    const next = dir > 0 ? allDials.find((d) => d > dial) : allDials.filter((d) => d < dial).pop();
    if (next !== undefined) moveToDial(next);
  };
  const stepCost = (dir: number) => {
    const next = dir > 0 ? allCosts.find((c) => c > cost) : allCosts.filter((c) => c < cost).pop();
    if (next !== undefined) moveToCost(next);
  };
  const typeDial = (text: string) => {
    const wanted = parseStrength(text);
    if (wanted !== null && allDials.length > 0) moveToDial(minBy(allDials, wanted));
  };
  const typeCost = (text: string) => {
    const wanted = parseStrength(text);
    if (wanted !== null && allCosts.length > 0) moveToCost(minBy(allCosts, wanted));
  };
  // A token cycles out, half, full, out. A step that can't be paid for goes back to out.
  const clickToken = (isSpecial: boolean, i: number) => {
    const row = [...(isSpecial ? s : n)];
    const halfAllowed = isSpecial ? o.halfSpecial : o.halfNormal;
    row[i] = row[i] === "out" ? (halfAllowed ? "half" : "full") : row[i] === "half" ? "full" : "out";
    const [sr, nr] = isSpecial ? [row, n] : [s, row];
    if (costOfRows(sr, nr) > spiceAvailable) row[i] = "out";
    setRows(sr, nr);
  };

  const hero = idOrNull(draft._heroId);
  const weapon = idOrNull(draft._weaponCardId);
  const defense = idOrNull(draft._defenseCardId);
  const messiah = draft.Messiah as boolean;
  const pickHero = (id: number) => {
    if (id === hero) {
      if (!o.leaderRequired) void update({ _heroId: -1 });
    } else {
      void update({ _heroId: id });
    }
  };

  const leaderPart = o.leader ? o.leader.value + o.leader.skillBonus + (messiah ? 2 : 0) : 0;
  let sum = `dial ${formatStrength(dial)}, no leader`;
  if (hero !== null && o.leader) {
    sum = `dial ${formatStrength(dial)} + leader ${o.leader.value}`;
    if (o.leader.skillBonus !== 0) sum += ` + ${o.leader.skillLabel} ${o.leader.skillBonus}`;
    if (messiah) sum += ` + ${skin.concept("Messiah")} 2`;
  }
  const [allyMin, allyMax] = allyRange(Number.isFinite(cost) ? cost : 0);
  const harvester = skin.url(skin.str("Harvester_URL"));

  const tokenRow = (row: Commit[], isSpecial: boolean) => {
    if (row.length === 0) return null;
    const name = skin.get(isSpecial ? "SpecialForceName_STR" : "ForceName_STR", o.supplier);
    const label = { full: "full strength", half: "half strength", out: "not dialled" };
    return (
      <div className="bp-pips">
        {row.map((state, i) => (
          <button key={i} type="button" className={`bp-pip ${state}`} title={`${name} · ${label[state]}`} onClick={() => clickToken(isSpecial, i)}>
            <ForceToken faction={o.supplier as Faction} special={isSpecial} size={26} />
          </button>
        ))}
      </div>
    );
  };

  const cards = (title: string, list: number[], chosen: number | null, key: string) => (
    <div>
      <div className="bp-label">{title}</div>
      <div className="bp-choices">
        <button type="button" className={`bp-none ${chosen === null ? "on" : ""}`} onClick={() => void update({ [key]: -1 })}>None</button>
        {list.map((c) => (
          <Image key={c} src={skin.cardImage(c)} alt={skin.card(c)} width={50} pointer highlightOnHover darkened={chosen !== c}
                 popover={treacheryCardPopup(skin, c)} onClick={() => void update({ [key]: c })} />
        ))}
      </div>
    </div>
  );

  return (
    <Layout type={action.eventType} inPopover={inPopover} className="battle-plan prompt-act"
            header={<span className="kicker">Battle · {o.territory >= 0 && <PlaceChip place={{ territory: o.territory }} />}</span>}>
      <div className="bp-matchup">
        <Side side={o.you} name="You" opponent={false} />
        <div className="bp-middle" data-ref-locations={o.territoryLocations.join(",")}>
          <div className="bp-vs">vs</div>
          <div className="bp-where">{o.territoryLabel}</div>
        </div>
        <Side side={o.opponent} name={skin.faction(o.opponent.faction as Faction)} opponent />
      </div>

      {o.notes.map((note, i) => <Note key={i} note={note} />)}

      {n.length + s.length > 0 && (
        <div className="bp-section">
          <div className="bp-label">Forces</div>
          <div className="bp-ctl">
            <span className="bp-what">Dial</span>
            <StepperInput value={formatStrength(dial)} title="The strength of your forces in this battle" mayDown={allDials.some((d) => d < dial)}
                          mayUp={allDials.some((d) => d > dial)} step={stepDial} typed={typeDial} />
          </div>
          {o.mustPay && (
            <>
              <div className="bp-ctl">
                <span className="bp-what">{skin.concept("Resource")}</span>
                <StepperInput value={Number.isFinite(cost) ? String(cost) : "0"} title={`${skin.concept("Resource")} used to support forces`}
                              mayDown={allCosts.some((c) => c < cost)} mayUp={allCosts.some((c) => c > cost)} step={stepCost} typed={typeCost}
                              icon={<span className="ic"><img src={harvester} alt="" /></span>} />
              </div>
              {o.allyCap > 0 && cost > 0 && (
                <div className="bp-ctl">
                  <span className="bp-what">From ally</span>
                  <div className="d-flex">
                    <NumberPicker value={draft.AllyContributionAmount as number} min={allyMin} max={allyMax} spice symbolSrc={skin.factionImage(o.ally as Faction)}
                                  title={`${skin.concept("Resource")} paid by your ally`} onChange={(v) => void update({ AllyContributionAmount: v })} />
                  </div>
                </div>
              )}
            </>
          )}
          {tokenRow(s, true)}
          {tokenRow(n, false)}
        </div>
      )}

      {o.heroes.length > 0 && (
        <div className="bp-section">
          <div className="bp-label">Leader</div>
          <div className="bp-choices">
            {o.heroes.map((h) => (
              <Image key={h.id} src={heroImage(skin, h.hero)} alt={h.label} width={58} pointer highlightOnHover popover={heroPopup(skin, view, h.hero)}
                     darkened={hero !== null && hero !== h.id} onClick={() => pickHero(h.id)} />
            ))}
          </div>
        </div>
      )}

      <div className="bp-section bp-cards">
        {cards("Weapon", o.weapons, weapon, "_weaponCardId")}
        {cards("Defense", o.defenses, defense, "_defenseCardId")}
      </div>

      {o.messiahAvailable && (
        <div className="bp-section">
          <Radio values={[false, true]} labels={["No", "Yes"]} value={messiah} onChange={(v) => void update({ Messiah: v })}
                 label={<>Should <img className="img-fluid" width={32} height={32} src={skin.url(skin.str("Messiah_URL"))} title={skin.concept("Messiah")} alt={skin.concept("Messiah")} /> join the fight?</>} />
        </div>
      )}

      {o.maxBankerBoost > 0 && (
        <div className="bp-section">
          <div className="d-inline-flex align-items-center">
            <label className="me-1">{o.bankerLabel}</label>
            <NumberPicker value={draft.BankerBonus as number} min={0} max={o.maxBankerBoost} spice symbolSrc={harvester}
                          onChange={(v) => void update({ BankerBonus: v })} />
          </div>
        </div>
      )}

      <div className="bp-total">
        <div>
          <div className="bp-label">Strength</div>
          <div className="bp-big">{formatStrength(dial + leaderPart)}</div>
        </div>
        <div className="bp-sum">{sum}</div>
        <button type="button" className="btn btn-primary" disabled={validation !== null} onClick={() => void submit()}>Battle!</button>
      </div>
      {validation && <div className="bp-why-not"><RichText parts={validation} /></div>}
      <SubmitError error={error} />
    </Layout>
  );
}

// The dial and spice steppers: − and + move to the next value a mix allows; a typed value goes
// to the nearest one.
function StepperInput({ value, title, mayDown, mayUp, step, typed, icon }: {
  value: string;
  title: string;
  mayDown: boolean;
  mayUp: boolean;
  step: (dir: number) => void;
  typed: (text: string) => void;
  icon?: ReactNode;
}) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  return (
    <div className="num">
      {icon}
      <button type="button" disabled={!mayDown} onClick={() => step(-1)}>−</button>
      <input value={text} title={title} onChange={(e) => setText(e.target.value)} onBlur={(e) => typed(e.target.value)}
             onKeyDown={(e) => {
               if (e.key === "ArrowUp") step(1);
               else if (e.key === "ArrowDown") step(-1);
               else if (e.key === "Enter") typed((e.target as HTMLInputElement).value);
             }} />
      <button type="button" disabled={!mayUp} onClick={() => step(1)}>+</button>
    </div>
  );
}

// One side of the matchup: sigil, name, and the forces it brings. The aggressor gets an
// "aggressor" label under its name.
function Side({ side, name, opponent }: { side: BattleSide; name: string; opponent: boolean }) {
  const { skin } = useGame();
  const f = side.faction as Faction;
  return (
    <div className={`bp-side ${opponent ? "them" : "you"}`} data-ref-faction={f} data-ref-locations={side.locations.join(",")}
         style={{ ["--fc" as string]: skin.factionColor(f) }}>
      <img className="bp-sigil" src={skin.factionImage(f)} alt="" title={skin.faction(f)} />
      <span className="bp-name">{name}</span>
      {side.aggressor && <span className="bp-agg">aggressor</span>}
      <span className="bp-forces">
        {side.groups.map((g, i) => (
          <span key={i} className="bp-groups">
            {g.forces > 0 && <><ForceToken faction={g.faction as Faction} size={20} title={skin.get("ForceName_STR", g.faction)} /><b>{g.forces}</b></>}
            {g.specialForces > 0 && (
              <>
                <ForceToken faction={g.faction as Faction} special size={20} title={skin.get("SpecialForceName_STR", g.faction)} />
                {!g.hideSpecialCount && <b>{g.specialForces}</b>}
              </>
            )}
          </span>
        ))}
      </span>
    </div>
  );
}

function Note({ note }: { note: BattleNote }) {
  switch (note.t) {
    case "ecazOpponent":
      return <div className="bp-note good"><FactionChip faction="Ecaz" /> forces contribute <strong>{note.amount}</strong> to your opponent's dial.</div>;
    case "opponentNoLeaders":
      return <div className="bp-note bad">Your opponent has no leaders available for this battle!</div>;
    case "voice":
      return note.mine
        ? <div className="bp-note good">Your opponent {note.must ? "must use" : "may not use"} {note.cardType}.</div>
        : <div className="bp-note bad">You {note.must ? "must use" : "may not use"} {note.cardType}.</div>;
    case "seen":
      return (
        <div className="bp-note good">Enemy {note.aspect.toLowerCase()}:{" "}
          {note.dial !== null ? <strong>{note.dial}</strong> : note.shown !== null ? <RichText parts={note.shown} /> : <span>none</span>}.
        </div>
      );
    case "later":
      return note.aspect === null
        ? <div className="bp-note good">Opponent battle plan will be visible as soon as they confirm it.</div>
        : <div className="bp-note good">Opponent {note.aspect.toLowerCase()} will be visible as soon as they confirm their battle plan.</div>;
    case "ecazMine":
      return <div className="bp-note good"><FactionChip faction="Ecaz" /> forces contribute <strong>{note.amount}</strong> to your dial.</div>;
  }
}
