// The game's small tooltip, in place of the browser's own. Any element with a
// title attribute gets it: on hover the title moves to data-tip, so the
// browser's tooltip never shows, and after a delay one shared box (#tip,
// styled in _tip.scss) shows the same text above the element, or below it
// when there is no room. The text and the call sites are unchanged.
//
// Blazor sets title again whenever its value changes, and the next hover then
// picks up the new text.
(() => {
    // Chrome on macOS shows a title through AppKit's tooltips (see Chromium's
    // ui/base/cocoa/tool_tip_base_view.mm), so the delay is the system's. This
    // is an estimate of that delay, not a measured value.
    const DELAY = 1000; // ms from entering the element to showing the tooltip
    const GAP = 8;      // px between the element and the tooltip's caret
    const MARGIN = 8;   // px kept clear of the window edge

    let tip = null;
    let current = null;
    let timer = null;

    function box() {
        if (!tip) {
            tip = document.createElement("div");
            tip.id = "tip";
            tip.setAttribute("role", "tooltip");
            document.body.appendChild(tip);
        }
        return tip;
    }

    // Moves a title to data-tip, keeping it for screen readers as a description.
    function adopt(el) {
        const title = el.getAttribute("title");
        if (title !== null) {
            el.removeAttribute("title");
            if (title !== "") {
                el.dataset.tip = title;
                if (!el.hasAttribute("aria-description")) el.setAttribute("aria-description", title);
            } else {
                delete el.dataset.tip;
            }
        }
        return el.dataset.tip;
    }

    function show(el) {
        const text = el.dataset.tip;
        if (!text || !el.isConnected) return;

        const t = box();
        t.textContent = text;
        t.classList.remove("on", "below");
        t.style.left = "0px";
        t.style.top = "0px";

        const r = el.getBoundingClientRect();
        const w = t.offsetWidth;
        const h = t.offsetHeight;
        const cx = r.left + r.width / 2;
        const left = Math.min(Math.max(MARGIN, cx - w / 2), innerWidth - w - MARGIN);
        let top = r.top - h - GAP;
        const below = top < MARGIN;
        if (below) top = r.bottom + GAP;

        t.style.left = `${Math.round(left)}px`;
        t.style.top = `${Math.round(top)}px`;
        t.style.setProperty("--caret-x", `${Math.round(cx - left)}px`);
        t.classList.toggle("below", below);
        t.classList.add("on");
    }

    function hide() {
        clearTimeout(timer);
        timer = null;
        current = null;
        if (tip) tip.classList.remove("on");
    }

    document.addEventListener("mouseover", e => {
        const el = e.target.closest?.("[title], [data-tip]");
        if (el === current) return;
        hide();
        if (!el || !adopt(el)) return;
        current = el;
        timer = setTimeout(() => show(el), DELAY);
    });

    document.addEventListener("mouseout", e => {
        if (current && !current.contains(e.relatedTarget)) hide();
    });

    // A click, key, scroll or leaving the window ends the hover, as with the
    // browser's tooltip.
    document.addEventListener("mousedown", hide, true);
    document.addEventListener("keydown", hide, true);
    document.addEventListener("scroll", hide, true);
    window.addEventListener("blur", hide);
})();
