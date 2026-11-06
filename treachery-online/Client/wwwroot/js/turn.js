// The red dot on the tab's icon while it's the player's turn, so a player in
// another tab can see they're holding up the game. Client.cs calls
// SetTurnDot after every update.
(function () {
    const PLAIN = "favicon.ico";
    let dotted = null; // the icon with the dot, drawn once
    let on = false;

    function link() {
        let l = document.querySelector("link[rel='icon']");
        if (!l) {
            l = document.createElement("link");
            l.rel = "icon";
            l.href = PLAIN;
            document.head.appendChild(l);
        }
        return l;
    }

    function draw() {
        return new Promise(resolve => {
            const img = new Image();
            img.onload = () => {
                const size = 64;
                const c = document.createElement("canvas");
                c.width = c.height = size;
                const g = c.getContext("2d");
                g.drawImage(img, 0, 0, size, size);
                const r = size * 0.22;
                g.beginPath();
                g.arc(size - r - 1, r + 1, r, 0, 2 * Math.PI);
                g.fillStyle = "#e0312b";
                g.fill();
                g.lineWidth = size * 0.05;
                g.strokeStyle = "#ffffff";
                g.stroke();
                resolve(c.toDataURL("image/png"));
            };
            img.onerror = () => resolve(null);
            img.src = PLAIN;
        });
    }

    window.SetTurnDot = async function (show) {
        if (show === on) return;
        on = show;
        if (show && dotted === null) dotted = await draw();
        if (on !== show) return; // changed while drawing
        link().href = show && dotted ? dotted : PLAIN;
    };
})();
