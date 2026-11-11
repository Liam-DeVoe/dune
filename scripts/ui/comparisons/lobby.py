"""Login, lobby, game setup and seating, against the C# server and the Rust server (package 8)."""


def create_account(u, user: str, player: str) -> None:
    u.button("Create account")
    u.fill("input[placeholder='User name (used to log in)']", user)
    u.fill("input[placeholder='Password']", "secret1")
    u.fill("input[placeholder='Repeat password']", "secret1")
    u.fill("input[placeholder='Player name (visible to other players)']", player)
    u.click_on(".card-body button.btn-primary:has-text('Create account')")
    u.wait_text("Active games" if u.name != "admin" else "Server info")


def run(c):
    with c.lobby() as lobby:
        # The admin page, before any game exists (its tables show times to the second).
        admin = lobby.user("admin", "/admin")
        admin.wait_text("Welcome to treachery.online!")
        create_account(admin, "admin", "Administrator")
        admin.wait_text("Server info")
        admin.shot("admin")
        admin.close()

        alice = lobby.user("alice")
        alice.wait_text("Welcome to treachery.online!")
        alice.shot("login")
        alice.button("Create account")
        alice.shot("login-new-account")
        alice.click_on(".card-body button.btn-primary:has-text('Create account')")
        alice.shot("login-new-account-invalid")
        alice.button("Forgot password?")
        alice.shot("login-forgot-password")
        alice.button("Set new password")
        alice.shot("login-set-password")
        alice.button("Login")
        alice.fill("input[placeholder='User name']", "nobody")
        alice.fill("input[placeholder='Password']", "wrong")
        alice.button("Log in")
        alice.wait_text("Invalid user name or password")
        alice.shot("login-failed")

        create_account(alice, "aliceuser", "Alice")
        # Heartbeats list a user as online from the second one on.
        alice.wait(13_000)
        alice.shot("lobby-empty")
        alice.button("Create or load a game")
        alice.shot("lobby-create")
        alice.button("Schedule a game")
        alice.shot("lobby-schedule")
        alice.button("Change account info")
        alice.shot("lobby-account-edit")
        alice.button("Cancel")

        alice.button("Schedule a game")
        alice.button("Schedule game")
        alice.wait_text("Scheduled games")
        alice.shot("lobby-scheduled")

        alice.button("Create or load a game")
        alice.button("Create new game")
        alice.wait_text("Number of players:")
        alice.shot("establish-players")

        bobby = lobby.user("bobby")
        bobby.wait_text("Welcome to treachery.online!")
        create_account(bobby, "bobbyuser", "Bobby")
        bobby.wait(13_000)
        bobby.shot("lobby-open-game")
        bobby.click_on(".table-scheduled-games .dropdown-toggle")
        bobby.shot("lobby-subscribe-menu")
        bobby.click_on(".table-scheduled-games .dropdown-item:has-text('yes*')")
        bobby.wait_text("yes*")
        bobby.shot("lobby-subscribed")
        bobby.button("Join")
        bobby.wait_text("Welcome to Alice's Game")
        # The host sends its settings every 3.6 s.
        bobby.wait(5_000)
        bobby.shot("awaiting-game")
        alice.shot("establish-players-joined")

        alice.click_on("label:has-text('Change advanced options')")
        alice.shot("establish-players-advanced")
        # AI Players: the one rule of RuleGroup.Bots.
        alice.click_on("label:has-text('Bots may not initiate alliances')")
        alice.shot("establish-players-bot-rule")
        alice.click_on("label:has-text('Change advanced options')")
        # Three players for two users: the third seat becomes a bot when the game starts.
        for _ in range(3):
            alice.click_on("button[aria-label='Decrease'] >> nth=1")
        alice.shot("establish-players-three")
        alice.button("Start")
        # The game screen is other packages' comparison, and its factions come from a random
        # seed (and the bot plays on its own), so only check that both servers started the game
        # for both users.
        for u in (alice, bobby):
            for page in u.pages.values():
                page.locator(".game-screen").first.wait_for(timeout=30_000)

