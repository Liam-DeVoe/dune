// Keeps a scrolling list pinned to its newest line, like a chat. Any element
// with data-follow-bottom follows: while the player is at the bottom, new
// content scrolls into view. Scrolling up stops it, so nobody is yanked back
// while reading; scrolling back down to the bottom starts it again. A change
// to the element's data-step (the report panel's phase) starts it again too,
// showing that phase's latest lines.
(function () {
    const SLACK = 6; // px from the bottom that still counts as at the bottom

    function follow(el) {
        if (el._follow) return;
        el._follow = true;
        let stuck = true;
        let lastStep = el.dataset.step;

        const toBottom = () => { el.scrollTop = el.scrollHeight; };

        el.addEventListener("scroll", () => {
            stuck = el.scrollHeight - el.scrollTop - el.clientHeight <= SLACK;
        }, { passive: true });

        new MutationObserver(() => {
            if (el.dataset.step !== lastStep) {
                lastStep = el.dataset.step;
                stuck = true;
            }
            if (stuck) toBottom();
        }).observe(el, { childList: true, subtree: true, characterData: true, attributes: true, attributeFilter: ["data-step"] });

        // Images (sigils, cards) that load after a line is added make it taller.
        el.addEventListener("load", () => { if (stuck) toBottom(); }, true);

        toBottom();
    }

    function scan(root) {
        if (root.matches?.("[data-follow-bottom]")) follow(root);
        root.querySelectorAll?.("[data-follow-bottom]").forEach(follow);
    }

    new MutationObserver(records => {
        for (const r of records) r.addedNodes.forEach(n => { if (n.nodeType === 1) scan(n); });
    }).observe(document.documentElement, { childList: true, subtree: true });

    document.addEventListener("DOMContentLoaded", () => scan(document));
})();
