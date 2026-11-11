// Support for the /scene page (Pages/Scene.razor), which renders a saved game
// for a screenshot harness. Only does something on that path.
//
// The harness can rely on:
//   <html data-scene="loading|ready|error">   ready once the page has rendered and run its steps
//   <html class="scene-still">                 transitions and animations are switched off
//   treacheryScene.renders                     number of renders of the page so far
//   treacheryScene.locations                   { "Carthag": {id, x, y}, ... } in map coordinates
//   treacheryScene.click(name, {shift, ctrl, right})   click a location; resolves after the redraw
//   treacheryScene.button(label)               click the first visible button with that text; resolves after the redraw
//   treacheryScene.settled(quietMs)            resolves once #app has had no DOM changes for quietMs and no animation runs
(function () {

    const path = location.pathname.replace(/\/+$/, '').toLowerCase();
    const onScene = path.endsWith('/scene');

    if (onScene) {
        document.documentElement.dataset.scene = 'loading';
        document.documentElement.classList.add('scene-still');
    }

    let page = null;

    window.treacheryScene = {

        renders: 0,
        locations: {},

        // Called by Scene.razor.
        attach(dotNetPage) { page = dotNetPage; },
        rendered() { this.renders++; },
        setLocations(locations) { this.locations = locations; },
        setState(state) { document.documentElement.dataset.scene = state; },

        click(name, options) {
            const o = options || {};
            return page.invokeMethodAsync('Click', name, !!o.shift, !!o.ctrl, !!o.right);
        },

        button(label) {
            return page.invokeMethodAsync('PressButton', label);
        },

        // Another faction offers the seated one a deal, or withdraws its newest offer.
        offerDeal(from, price, withdraw) {
            return page.invokeMethodAsync('OfferDeal', from, price || 0, !!withdraw);
        },

        // Clicks the first visible, enabled button whose text is the label.
        // Returns whether one was found.
        pressButton(label) {
            const wanted = label.trim();
            const buttons = Array.from(document.querySelectorAll('button'))
                .filter(b => b.offsetParent !== null && !b.disabled);
            const button = buttons.find(b => b.textContent.trim() === wanted)
                || buttons.find(b => b.textContent.trim().toLowerCase() === wanted.toLowerCase());
            if (button == null) return false;
            button.click();
            return true;
        },

        settled(quietMs) {
            const quiet = quietMs == null ? 100 : quietMs;
            return new Promise(resolve => {
                const app = document.getElementById('app');
                let timer = null;
                const observer = new MutationObserver(schedule);

                function schedule() {
                    clearTimeout(timer);
                    timer = setTimeout(check, quiet);
                }

                function check() {
                    const animating = document.getAnimations().some(a => a.playState === 'running');
                    if (animating) { schedule(); return; }
                    observer.disconnect();
                    resolve();
                }

                observer.observe(app, { subtree: true, childList: true, attributes: true, characterData: true });
                schedule();
            });
        }
    };
})();
