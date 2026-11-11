// Package H1's differential probe. Modes:
//
//   mail   stdin: one JSON string per line. stdout: one JSON object per line, the outcome of
//          `new MailAddress(s)` (ok with User, Host, DisplayName, ToString(); or the exception).
//   chars  string.ToLower() (invariant culture, as the server runs) for every code point whose
//          lower case differs, and every UTF-16 unit for which char.IsWhiteSpace holds.
//   ef     the SQL EF Core generates for the authentication queries (GameHub_Authentication.cs),
//          and which rows they match on non-ASCII data.
//   dates  stdin: one JSON string per line. Each is stored as the text of a Users.LastLogin
//          cell and read back through EF Core (a DateTimeOffset property). stdout: one JSON
//          object per line with the local time zone and the value read (Ticks, Offset in
//          minutes) or the exception. Run it under several TZ values to see that the local
//          time zone plays no part. Used by rust/server/tests/db_dates.rs.
//
// Used by rust/server/tests/auth_dotnet.rs.

using System.Globalization;
using System.Net.Mail;
using System.Text;
using System.Text.Json;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Diagnostics;

CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
CultureInfo.CurrentUICulture = CultureInfo.InvariantCulture;
Console.OutputEncoding = new UTF8Encoding(false);
var stdout = new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false)) { AutoFlush = false };

switch (args.Length > 0 ? args[0] : "")
{
    case "mail":
        Mail(stdout);
        break;
    case "chars":
        Chars(stdout);
        break;
    case "ef":
        await Ef(stdout);
        break;
    case "dates":
        await Dates(stdout);
        break;
    default:
        Console.Error.WriteLine("usage: H1Probe mail|chars|ef|dates");
        return 2;
}
stdout.Flush();
return 0;

static void Mail(StreamWriter stdout)
{
    var stdin = new StreamReader(Console.OpenStandardInput(), new UTF8Encoding(false));
    string? line;
    while ((line = stdin.ReadLine()) != null)
    {
        var address = JsonSerializer.Deserialize<string>(line)!;
        object result;
        try
        {
            var m = new MailAddress(address);
            result = new { ok = true, user = Lossy(m.User), host = Lossy(m.Host), display = Lossy(m.DisplayName), text = Lossy(m.ToString()) };
        }
        catch (Exception e)
        {
            result = new { ok = false, type = e.GetType().Name, msg = Lossy(e.Message) };
        }
        stdout.WriteLine(JsonSerializer.Serialize(result));
    }
}

// Lone surrogates (a message can quote half of a pair) become U+FFFD, as Rust's
// String::from_utf16_lossy does.
static string Lossy(string s)
{
    var b = new StringBuilder(s.Length);
    for (var i = 0; i < s.Length; i++)
    {
        if (char.IsHighSurrogate(s[i]) && i + 1 < s.Length && char.IsLowSurrogate(s[i + 1]))
        {
            b.Append(s[i]).Append(s[i + 1]);
            i++;
        }
        else if (char.IsSurrogate(s[i])) b.Append('\uFFFD');
        else b.Append(s[i]);
    }
    return b.ToString();
}

static void Chars(StreamWriter stdout)
{
    for (var cp = 0; cp <= 0x10FFFF; cp++)
    {
        if (cp is >= 0xD800 and <= 0xDFFF) continue;
        var s = char.ConvertFromUtf32(cp);
        var lower = s.ToLower();
        if (lower != s)
        {
            var lowerCps = new List<string>();
            for (var i = 0; i < lower.Length; i += char.IsSurrogatePair(lower, i) ? 2 : 1)
                lowerCps.Add(char.ConvertToUtf32(lower, i).ToString("X"));
            stdout.WriteLine($"lower {cp:X} {string.Join(",", lowerCps)}");
        }
    }
    for (var c = 0; c <= 0xFFFF; c++)
    {
        if (char.IsWhiteSpace((char)c)) stdout.WriteLine($"ws {c:X}");
    }
    // Trim() and ToLower() of whole strings, to catch context-dependent rules (final sigma).
    foreach (var s in new[] { "ΟΔΟΣ", "ΣΑΣ Σ", "İSTANBUL", "ǅ", "ẞ", "\u0085a ", "a​b​", "᠎a᠎", "﻿a﻿" })
    {
        stdout.WriteLine("str " + JsonSerializer.Serialize(new[] { s, s.ToLower(), s.Trim(), s.Length.ToString() }));
    }
}

static async Task Ef(StreamWriter stdout)
{
    var path = Path.Combine(Path.GetTempPath(), $"h1probe-{Environment.ProcessId}.db");
    File.Delete(path);
    var sql = new List<string>();
    var options = new DbContextOptionsBuilder<ProbeContext>()
        .UseSqlite($"Data Source={path};Pooling=False")
        .LogTo(s => sql.Add(s), new[] { RelationalEventId.CommandExecuted })
        .Options;
    var now = new DateTimeOffset(2026, 11, 8, 12, 0, 0, TimeSpan.FromHours(-5));

    void Dump(string label)
    {
        foreach (var s in sql)
        {
            // Keep only the command text: drop the "Executed DbCommand (..ms) [Parameters=...]" header.
            var start = s.IndexOf('\n');
            var text = start < 0 ? s : s[(start + 1)..];
            stdout.WriteLine(JsonSerializer.Serialize(new { label, sql = text.Trim() }));
        }
        sql.Clear();
    }

    await using (var db = new ProbeContext(options))
    {
        await db.Database.EnsureCreatedAsync();
    }
    sql.Clear();

    var names = new[] { "abc", " abc ", "ABC", "\tabc", " abc", "äbc", "ÄBC", "straße", "STRASSE", "σς", "ΣΣ", "ǅx", "İx", "ix", "ab c" };
    await using (var db = new ProbeContext(options))
    {
        var i = 0;
        foreach (var n in names)
        {
            db.Add(new User { Name = n, Email = names[(i + 3) % names.Length].ToUpperInvariant(), HashedPassword = "h", PlayerName = "p", LastLogin = now });
            i++;
        }
        await db.SaveChangesAsync();
        foreach (var u in db.Users.OrderBy(x => x.Id).ToList())
            stdout.WriteLine(JsonSerializer.Serialize(new { label = "data", id = u.Id, name = u.Name, email = u.Email }));
    }
    sql.Clear();

    await using (var db = new ProbeContext(options))
    {
        var cleanedUsername = "abc";
        var hashedPassword = "h";
        _ = db.Users.Any(x => Equals(x.Name, cleanedUsername));
        Dump("users_any_with_name");
        _ = await db.Users.FirstOrDefaultAsync(x => Equals(x.Name, cleanedUsername) && x.HashedPassword == hashedPassword);
        Dump("users_first_by_name_and_password");
        var userName = " ABC ";
        _ = await db.Users.FirstOrDefaultAsync(x => x.Name.Trim().ToLower().Equals(userName.Trim().ToLower()) && x.HashedPassword == hashedPassword);
        Dump("users_first_for_login");
        var usernameOrEmail = "abc";
        _ = db.Users.Where(x => x.Email.Trim().ToLower().Equals(usernameOrEmail.Trim().ToLower()) || x.Name.Trim().ToLower().Equals(usernameOrEmail.Trim().ToLower())).ToList();
        Dump("users_where_email_or_name");
        _ = await db.Users.FirstOrDefaultAsync(x => x.Name.Trim().ToLower().Equals(userName.Trim().ToLower()));
        Dump("users_first_by_name");
    }

    // Tracked updates: the columns SaveChanges writes.
    await using (var db = new ProbeContext(options))
    {
        var u = await db.Users.FirstAsync(x => x.Id == 1);
        sql.Clear();
        u.LastLogin = now.AddMinutes(1);
        await db.SaveChangesAsync();
        Dump("tracked_last_login");
        u.LastLogin = now.AddMinutes(1).ToOffset(TimeSpan.Zero);
        await db.SaveChangesAsync();
        Dump("tracked_same_instant_other_offset");
        u.PasswordResetToken = "T";
        u.PasswordResetTokenCreated = now;
        await db.SaveChangesAsync();
        Dump("tracked_reset");
        u.HashedPassword = "h";
        u.PasswordResetToken = string.Empty;
        u.PasswordResetTokenCreated = default;
        await db.SaveChangesAsync();
        Dump("tracked_set_password_same_hash");
    }
    await using (var db = new ProbeContext(options))
    {
        var u = new User { Id = 2, Name = "x", Email = "e", HashedPassword = "h", PlayerName = "p", LastLogin = now };
        db.Users.Update(u);
        await db.SaveChangesAsync();
        Dump("update_all");
    }
    await using (var db = new ProbeContext(options))
    {
        var u = new User { Id = 999, Name = "x", Email = "e", HashedPassword = "h", PlayerName = "p", LastLogin = now };
        db.Users.Update(u);
        try
        {
            await db.SaveChangesAsync();
            stdout.WriteLine(JsonSerializer.Serialize(new { label = "update_all_missing", result = "ok" }));
        }
        catch (Exception e)
        {
            stdout.WriteLine(JsonSerializer.Serialize(new { label = "update_all_missing", result = e.GetType().Name }));
        }
        sql.Clear();
    }
    await using (var db = new ProbeContext(options))
    {
        var u = await db.Users.FirstAsync(x => x.Id == 3);
        await db.Users.Where(x => x.Id == 3).ExecuteDeleteAsync();
        u.LastLogin = now.AddDays(1);
        try
        {
            await db.SaveChangesAsync();
            stdout.WriteLine(JsonSerializer.Serialize(new { label = "tracked_missing", result = "ok" }));
        }
        catch (Exception e)
        {
            stdout.WriteLine(JsonSerializer.Serialize(new { label = "tracked_missing", result = e.GetType().Name }));
        }
        sql.Clear();
    }

    // Matching on non-ASCII data: which ids each query returns.
    var inputs = new[] { "abc", "ABC", " abc ", "äbc", "ÄBC", "STRAßE", "strasse", "ΣΣ", "σσ", "ǅX", "İX", "IX", "AB C", " abc" };
    await using (var db = new ProbeContext(options))
    {
        foreach (var input in inputs)
        {
            var login = await db.Users.Where(x => x.Name.Trim().ToLower().Equals(input.Trim().ToLower()) && x.HashedPassword == "h").Select(x => x.Id).ToListAsync();
            var reset = db.Users.Where(x => x.Email.Trim().ToLower().Equals(input.Trim().ToLower()) || x.Name.Trim().ToLower().Equals(input.Trim().ToLower())).Select(x => x.Id).ToList();
            var exact = db.Users.Where(x => Equals(x.Name, input)).Select(x => x.Id).ToList();
            stdout.WriteLine(JsonSerializer.Serialize(new { label = "match", input, login, reset, exact }));
        }
        sql.Clear();
    }
    File.Delete(path);
}

static async Task Dates(StreamWriter stdout)
{
    var inputs = new List<string>();
    var stdin = new StreamReader(Console.OpenStandardInput(), new UTF8Encoding(false));
    string? line;
    while ((line = stdin.ReadLine()) != null)
        if (line.Length > 0) inputs.Add(JsonSerializer.Deserialize<string>(line)!);

    var path = Path.Combine(Path.GetTempPath(), $"h1probe-dates-{Environment.ProcessId}.db");
    File.Delete(path);
    var options = new DbContextOptionsBuilder<ProbeContext>().UseSqlite($"Data Source={path};Pooling=False").Options;
    await using (var db = new ProbeContext(options))
    {
        await db.Database.EnsureCreatedAsync();
        var conn = db.Database.GetDbConnection();
        await conn.OpenAsync();
        // Raw inserts, as sqlite3 or another tool would write the row.
        foreach (var input in inputs)
        {
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "INSERT INTO Users (Name, Email, HashedPassword, PlayerName, PasswordResetTokenCreated, LastLogin) VALUES ('n', 'e', 'h', 'p', '0001-01-01 00:00:00+00:00', $v)";
            var p = cmd.CreateParameter();
            p.ParameterName = "$v";
            p.Value = input;
            cmd.Parameters.Add(p);
            await cmd.ExecuteNonQueryAsync();
        }
    }
    var tz = TimeZoneInfo.Local.Id;
    for (var i = 0; i < inputs.Count; i++)
    {
        var id = i + 1;
        object result;
        await using var db = new ProbeContext(options);
        try
        {
            // The server's own read path: a tracked entity, as in GameHub_Authentication.
            var u = await db.Users.FirstAsync(x => x.Id == id);
            var v = u.LastLogin;
            result = new { tz, input = inputs[i], ok = true, ticks = v.Ticks, offset = (int)v.Offset.TotalMinutes };
        }
        catch (Exception e)
        {
            var inner = e;
            while (inner.InnerException != null) inner = inner.InnerException;
            result = new { tz, input = inputs[i], ok = false, type = inner.GetType().Name };
        }
        stdout.WriteLine(JsonSerializer.Serialize(result));
    }
    File.Delete(path);
}

public class User
{
    public int Id { get; init; }
    [System.ComponentModel.DataAnnotations.MaxLength(4000)] public string Name { get; set; } = string.Empty;
    [System.ComponentModel.DataAnnotations.MaxLength(4000)] public string Email { get; set; } = string.Empty;
    [System.ComponentModel.DataAnnotations.MaxLength(4000)] public string HashedPassword { get; set; } = string.Empty;
    [System.ComponentModel.DataAnnotations.MaxLength(4000)] public string? PasswordResetToken { get; set; }
    public DateTimeOffset PasswordResetTokenCreated { get; set; }
    [System.ComponentModel.DataAnnotations.MaxLength(4000)] public string PlayerName { get; set; } = string.Empty;
    public DateTimeOffset LastLogin { get; set; }
}

public class ProbeContext(DbContextOptions<ProbeContext> options) : DbContext(options)
{
    public DbSet<User> Users { get; set; } = null!;
}
