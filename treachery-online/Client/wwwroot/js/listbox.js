// The custom select (SelectComponent.razor). Blazor renders the trigger button
// and the option list; this file does the three things Blazor cannot:
//
//   - show the list in the browser's top layer (the popover attribute) so it
//     escapes panels that clip their overflow, and place it under the trigger
//   - close the list on an outside click, scroll or resize
//   - decide per key whether to swallow the browser default (arrows must not
//     scroll the page, but Tab must still move focus), then tell .NET
//
// .NET owns the open/closed state; aria-expanded on the trigger mirrors it.
window.listbox = (() => {
    const GAP = 4;      // px between trigger and list
    const MARGIN = 8;   // px kept clear of the viewport edge
    const openLists = new Map(); // list element -> cleanup function

    function isOpen(trigger) {
        return trigger.getAttribute("aria-expanded") === "true";
    }

    function place(trigger, list) {
        const r = trigger.getBoundingClientRect();
        const maxFromCss = parseFloat(getComputedStyle(list).getPropertyValue("--listbox-max-height")) || 320;
        const below = window.innerHeight - r.bottom - GAP - MARGIN;
        const above = r.top - GAP - MARGIN;
        const wanted = Math.min(list.scrollHeight, maxFromCss);
        const openAbove = below < wanted && above > below;
        const height = Math.min(wanted, openAbove ? above : below);

        list.style.minWidth = r.width + "px";
        list.style.maxHeight = Math.max(height, 40) + "px";

        // Set the width first so the horizontal clamp sees the real size.
        const width = list.offsetWidth;
        const left = Math.min(r.left, window.innerWidth - MARGIN - width);
        list.style.left = Math.max(MARGIN, left) + "px";

        if (openAbove) {
            list.style.top = "auto";
            list.style.bottom = (window.innerHeight - r.top + GAP) + "px";
        } else {
            list.style.bottom = "auto";
            list.style.top = (r.bottom + GAP) + "px";
        }
    }

    function show(list) {
        if (typeof list.showPopover === "function") {
            if (!list.matches(":popover-open")) list.showPopover();
        } else {
            list.style.display = "block";
        }
    }

    function hide(list) {
        if (typeof list.hidePopover === "function") {
            if (list.matches(":popover-open")) list.hidePopover();
        } else {
            list.style.display = "none";
        }
    }

    // Attaches the keyboard handler to the trigger. Called once per component.
    function init(trigger, dotnet) {
        trigger.addEventListener("keydown", e => {
            const open = isOpen(trigger);
            const navigation = ["ArrowDown", "ArrowUp", "Home", "End", "PageDown", "PageUp"];
            const printable = e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey;
            let handled = false;

            if (navigation.includes(e.key) || printable) {
                handled = true;
            } else if (open && ["Enter", " ", "Escape", "Tab"].includes(e.key)) {
                handled = true;
            }
            // When closed, Enter and Space fall through to the button's click.
            if (!handled) return;

            // Tab keeps its default so focus still moves on.
            if (e.key !== "Tab") e.preventDefault();
            dotnet.invokeMethodAsync("OnKey", e.key);
        });
    }

    function open(trigger, list, dotnet) {
        close(list);
        show(list);
        place(trigger, list);

        const onPointerDown = e => {
            if (trigger.contains(e.target) || list.contains(e.target)) return;
            dotnet.invokeMethodAsync("CloseFromBrowser");
        };
        const onScroll = e => {
            if (list.contains(e.target)) return;
            dotnet.invokeMethodAsync("CloseFromBrowser");
        };
        const onResize = () => dotnet.invokeMethodAsync("CloseFromBrowser");

        document.addEventListener("pointerdown", onPointerDown, true);
        window.addEventListener("scroll", onScroll, true);
        window.addEventListener("resize", onResize);

        openLists.set(list, () => {
            document.removeEventListener("pointerdown", onPointerDown, true);
            window.removeEventListener("scroll", onScroll, true);
            window.removeEventListener("resize", onResize);
        });
    }

    function close(list) {
        const cleanup = openLists.get(list);
        if (cleanup) {
            cleanup();
            openLists.delete(list);
        }
        hide(list);
    }

    // Keeps the highlighted option visible while the user arrows through.
    function reveal(list, optionId) {
        const option = document.getElementById(optionId);
        if (option) option.scrollIntoView({ block: "nearest" });
    }

    return { init, open, close, reveal };
})();
