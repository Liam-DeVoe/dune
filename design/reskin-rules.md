# Reskin rules

Scope for the visual refresh of the Blazor client under `treachery-online/Client`. This is a reskin, not a redesign. The reference for how things should look is `design/design-language.html`. Seven worked examples of the rule applied to real screens are in `design/reskin-scope.html`.

## The rule

Every word, control, and position on screen stays where it is. Only materials, type, and input widgets change.

## Do not change

- Any text: labels, headings, button captions, tooltips, placeholders, log sentences, error messages.
- The order of anything: panels, rows, buttons, checkboxes, dropdown options.
- Which panels are open or collapsed by default.
- Layout: the three-column game screen, the two-column lobby, Bootstrap grid classes (`row`, `col-*`), spacing utilities (`m*-`, `p*-`), flex utilities (`d-flex`, `justify-content-*`).
- Behaviour: click handlers, bindings, timers, hover panels, what is enabled or disabled.
- Icons: keep every icon that exists. Do not add labels beside icons. Do not add names beside faction sigils.
- Anything in `Shared/` or `Server/`.
- The map SVG and the art under `wwwroot/art`.

## Do change

- Colours: replace every Bootstrap Darkly grey and blue with the tokens. Page is ink, panels are ink 2, raised things are ink 3, text is parchment, secondary text is rock, primary buttons are gold.
- Surfaces: solid and opaque. No gradients, no textures behind panels, no translucency except the popover and modal backdrop.
- Borders: 1px white at 9% alpha. Gold only on the act card's left rule, focus rings, and selected states.
- Type: Advokat for panel titles that are already titles, Alegreya Sans for body. Where a player name is set in Orthodox Herbertarian, set it in the body face. The wordmark keeps Herbertarian.
- Buttons: `btn-primary` becomes flat gold with ink text. `btn-secondary` becomes an outline. `btn-danger` stays storm red. `btn-link` becomes gold text. Where two `btn-primary` buttons sit together today, both stay primary.
- Inputs: `form-control` and `form-select` become ink 0 with a neutral hairline and a gold focus ring. `form-check-input` becomes the language checkbox.
- Dropdowns: `SelectComponent` draws its own option list, the guide's "Select, open", because the browser's native menu ignores page styles on macOS. Same parameters and callbacks, so call sites keep working. Where a select has two to four fixed options, `SegmentedComponent` (same parameters) replaces it.
- The number picker: `NumberPickerComponent` becomes the typeable number input from the guide, with the same parameters and callbacks. It keeps the icon cell showing what is being counted: the faction force token for forces, the spice icon for spice, no cell for plain numbers. Every call site keeps working unchanged.
- The beige prompt (`#e8dba6`) becomes the parchment act card.
- Faction-coloured panel backgrounds become ink 2.
- The Blazor error bar keeps its words and gets the language's colours.

## How

- Do it at the CSS layer first. Bootstrap 5.3 exposes `--bs-*` variables, so most of the reskin is overrides in `Client/Styles/bootstrap/`. The SCSS under `Client/Styles/` compiles to `wwwroot/css/treachery.css` on build, loaded after `bootstrap.min.css`.
- Touch a `.razor` file only when the CSS layer cannot reach it: inline `style="..."` with a colour or font, or a Bootstrap class that has no equivalent in the language.
- When you replace an inline style, replace it with a class from `Client/Styles/`. Do not invent class names that are not in the guide unless you add them to `Client/Styles/app/_utilities.scss` with a comment saying what they are for.
- Build after every batch of changes with `dotnet build Client/Treachery.Client.csproj` from `treachery-online/`. Zero new warnings from files you touched.
- Do not commit.
