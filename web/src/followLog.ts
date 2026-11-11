// wwwroot/js/follow-log.js: keeps a scrolling list pinned to its newest line, like a chat. While
// the player is at the bottom, new content scrolls into view. Scrolling up stops it, so nobody is
// yanked back while reading; scrolling back down to the bottom starts it again. A change to the
// element's data-step (the report panel's phase) starts it again too, showing that phase's latest
// lines. The C# script finds elements by data-follow-bottom; here a component passes its element.

const SLACK = 6; // px from the bottom that still counts as at the bottom

const followed = new WeakSet<HTMLElement>();

export function followBottom(el: HTMLElement | null): void {
  if (el == null || followed.has(el)) return;
  followed.add(el);
  let stuck = true;
  let lastStep = el.dataset.step;

  // Where this script last scrolled to. A scroll event arrives a frame after the scroll, and the
  // lines may have grown in between; the C# script then reads the gap as the player scrolling up
  // and stops following, so where the C# log ends up depends on timing. A scroll event that finds
  // the panel where this script put it is not the player's.
  let placed = -1;
  const toBottom = () => { el.scrollTop = el.scrollHeight; placed = el.scrollTop; };

  el.addEventListener("scroll", () => {
    stuck = el.scrollTop === placed || el.scrollHeight - el.scrollTop - el.clientHeight <= SLACK;
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

  // So do fonts and anything else that changes a line's size without a DOM change, and the
  // panel itself changes size as the page settles. Blazor's later renders mutate the DOM and so
  // scroll again; React commits once, so the sizes are watched too.
  const resized = new ResizeObserver(() => { if (stuck) toBottom(); });
  const watch = () => {
    resized.disconnect();
    resized.observe(el);
    for (const child of Array.from(el.children)) resized.observe(child);
  };
  new MutationObserver(watch).observe(el, { childList: true });
  watch();

  toBottom();
}
