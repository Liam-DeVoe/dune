namespace Treachery.Server;

/// <summary>
/// Hooks for the server equivalence harness (treachery-online/ServerOracle), which runs this hub and the Rust
/// port on the same virtual clock and compares them. Every hook is null by default, and then the server runs
/// exactly the code it ran before the hook existed. Listed in rust/docs/csharp-hooks.md.
/// </summary>
public static class TestHooks
{
    /// <summary>Replaces <c>DateTimeOffset.Now</c> and <c>DateTime.Now</c>.</summary>
    public static Func<DateTimeOffset>? Clock { get; set; }

    /// <summary>Replaces the random token of <c>GenerateToken</c>.</summary>
    public static Func<string>? NewToken { get; set; }

    /// <summary>Replaces <c>Guid.NewGuid()</c>.</summary>
    public static Func<Guid>? NewGuid { get; set; }

    /// <summary>
    /// Replaces <c>_ = Task.Delay(milliseconds).ContinueWith(_ => action())</c>. The string names the delayed call,
    /// such as <c>"SendAsyncPlayMessagesIfApplicable &lt;gameId&gt;"</c>.
    /// </summary>
    public static Action<int, string, Func<Task>>? Delay { get; set; }

    /// <summary>
    /// Replaces the unseeded <c>new LoggedRandom()</c> of the bots: the bot order in <c>PerformBotEvent</c>
    /// (<c>Deck&lt;Player&gt;.Randomize</c>) and each bot's own random in <c>GetOrInitializeBot</c>. The string names the
    /// use: <c>"BotOrder &lt;gameId&gt;"</c> or <c>"Bot &lt;gameId&gt; &lt;faction&gt;"</c>.
    /// </summary>
    public static Func<string, LoggedRandom>? NewRandom { get; set; }

    /// <summary>Receives the mail that <c>SendMail</c> would send over SMTP (after its user name check).</summary>
    public static Func<MailMessage, Task>? MailSink { get; set; }

    internal static DateTimeOffset Now() => Clock != null ? Clock() : DateTimeOffset.Now;

    internal static DateTime LocalNow() => Clock != null ? Clock().LocalDateTime : DateTime.Now;

    internal static string NewGuidString() => NewGuid != null ? NewGuid().ToString() : Guid.NewGuid().ToString();
}
