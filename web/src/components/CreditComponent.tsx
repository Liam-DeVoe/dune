// OtherComponents/CreditComponent.razor: the menu's "Credits" modal.
import { ModalComponent } from "./ModalComponent";

const row = (label: string, value: React.ReactNode) => (
  <tr>
    <td style={{ width: "50%" }} className="p-2 text-end align-text-top">{label}</td>
    <td style={{ width: "50%" }} className="p-2 text-start align-text-top fw-bold">{value}</td>
  </tr>
);

export function CreditComponent({ id }: { id: string }) {
  return (
    <ModalComponent id={id} title="Credits">
      <table className="table-borderless text-center">
        <tbody>
          {row("Game Development", <>Ronald Ossendrijver, <a href="mailto:admin@treachery.online" target="_blank">admin@treachery.online</a></>)}
          {row("Design & Artwork", "Ilya Baranovsky, Reinier de Graaf & Piper")}
          {row("Leader & Treachery Card Artwork", "Avalon Hill")}
          {row("Music", "Stéphane Picq & Philippe Ulrich")}
          {row("Donations", (
            <>
              Andries, Stuart, Eamon, Adriaan, Luca, Valerio, Richard, Peter, Patrick, Vladislav, <strong>Derek</strong>, Darren, Doug, Aaron, Marko, Daniel, Erik, Shea, Justin, Lee, Marko, Aaron W.,{" "}
              Tom, Nathan, Denis, Jason, Stefan, Eliot, Dean, Chris, Sebastian, Brian, Nikita, Andreas, Klaude, Robert, Frederic, Howard, Francis, Marc, André, Kenny, William,{" "}
              Jean-François, Jorge, MelonNinja, Donovan, <strong>Ken-Yu</strong>, Peter, John, Maxwell, Dominic, Conquer Today, Duane, Michael, Rodrigo, Salvatore, André,{" "}
              Petr, Oliver, Sam, Vytautas, Erik, Marcel, Justin, Dimitris, Jerry, Sheehan, Graydon, Jerrod, <strong>Jennifer</strong>, <strong>Thomas</strong>, Robert, Seth, Daniel, Bradley, <strong>Christian</strong>,{" "}
              Nathaniel, Andrew, Thomas, Damien, Finn, <strong>Stanko</strong>, <strong>Jack</strong>, <strong>André</strong>, <strong>Jeffrey</strong>, David, Richard, <strong>Nicolai</strong>, Caspar, Ryan, Stuart,{" "}
              <strong>André</strong>, Rowan, Dirk, Michael, Dale, Ace_xD (Nico), Johaney, Kev, Alexander, Tyler, BigDave, Qaanaaq, Gurr, Damian, Delta & RaiderOfTwix
            </>
          ))}
        </tbody>
      </table>
      <a href="https://ko-fi.com/N4N618W4XV" target="_blank"><img height="36" style={{ border: 0, height: "36px" }} src="https://storage.ko-fi.com/cdn/kofi2.png?v=6" alt="Donate" /></a>
    </ModalComponent>
  );
}
