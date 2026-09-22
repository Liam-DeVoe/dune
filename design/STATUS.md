# Status

Last updated 2026-09-18. Nothing below is committed; check `git status`.

## What exists

- **A self-hosted copy of treachery.online** at https://treachery.exe.xyz/, running the upstream code as of commit `07e27e1`. Admin account `admin`, password in `deploy/.admin-password`. `just deploy` pushes the working tree to the box and restarts it in about 40 seconds. `just provision` rebuilds the box from nothing.
- **A design language** in `design-language.html`. Tokens sampled from the map and the skin file, three shipped display fonts plus Alegreya Sans for body, flat opaque surfaces, neutral hairlines, gold reserved for the primary action. The surface rules were decided with `loud-vs-quiet.html`: gradients, texture bleed, and gold borders off; display-font headers and the full set of text tones on.
- **A reskin of the client** under `treachery-online/Client`, applied under the rule in `reskin-rules.md`: every word, control, and position stays, only materials, type, and input widgets change. `reskin-scope.html` shows the rule on seven real screens. `reskin-review.html` shows real before and after screenshots from the built client.

## How the reskin was done

- `Client/Styles/` (SCSS, compiled to `wwwroot/css/treachery.css` on build) overrides Bootstrap's variables and the selectors Bootswatch Darkly hard-codes. This alone restyled most of the app.
- Three passes over `Pages`, `OtherComponents`, `GameEventComponents`, `GenericComponents`, `MapComponents`, and `Icons` removed inline colours, faction-coloured panel backgrounds, the beige prompt and hover strips, and player names set in the logo font. 41 files changed, net minus 168 lines.
- `NumberPickerComponent` was rewritten as a typeable input with plus and minus buttons and an icon cell showing what is being counted. Every parameter and callback was kept, so all 46 call sites are unchanged.
- Fonts are vendored under `wwwroot/fonts`. No runtime dependency on Google.
- Build verified clean. The client was driven locally through registration, game creation, host setup, advanced rules, game start, and traitor selection with no page errors. Force placement and bidding pickers were verified by markup, not screenshot.

## Judgement calls to review

- Panel headers with a collapse chevron use the body font; titled headers use Advokat.
- The "urgent" prompt style is an ink panel with a stronger hairline, not a second parchment card.
- The faction watermark behind the player shield sits under a translucent ink veil, the one translucency kept.
- Your own name in the seat list is gold.
- The player name on the map ring is still in the logo font because it comes from `Shared/DefaultSkin.cs`, which was out of scope.

## Not done, on purpose

- No UX changes. Faction names beside sigils, session persistence, timer bars, toasts, re-syncing the bid picker, collapsing panels by default, grouping advanced rules: all proposed in `design-language.html`, none applied.
- The reskin has not been deployed to the box. The box still runs the original look.

## Local tooling

- .NET 10 SDK at `~/dotnet10`. `just build` compiles the client in about 25 seconds. `just run` serves it at http://127.0.0.1:5050.
- Screenshots of the live site before the change are in `before/`. Screenshots of the reskinned client are in `after/`.
