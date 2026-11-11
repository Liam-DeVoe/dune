using System.Globalization;
using Treachery.Oracle.Canon;

namespace Treachery.Oracle;

/// <summary>
/// Writes, for every enum in the Shared assembly, the text the state dump uses for each defined value
/// (.NET Enum.ToString()). Lets the Rust side reproduce enum names, including which name .NET picks for
/// values that have several names.
/// </summary>
public static class EnumTable
{
    public static int Write(string path)
    {
        var o = new JsonOut();
        o.ObjStart();
        var first = true;
        foreach (var t in typeof(Game).Assembly.GetTypes().Where(t => t.IsEnum).OrderBy(t => t.FullName, StringComparer.Ordinal))
        {
            if (!first) o.Comma();
            first = false;
            o.Key(t.FullName!);
            o.ObjStart();
            o.Key("flags"); o.Bool(t.IsDefined(typeof(FlagsAttribute), false));
            o.Comma(); o.Key("values");
            o.ArrStart();
            var firstValue = true;
            var seen = new HashSet<long>();
            foreach (var v in Enum.GetValues(t))
            {
                var raw = Convert.ToInt64(v, CultureInfo.InvariantCulture);
                if (!seen.Add(raw)) continue;
                if (!firstValue) o.Comma();
                firstValue = false;
                o.ArrStart();
                o.Int(raw);
                o.Comma();
                o.WriteString(v.ToString());
                o.Comma();
                o.ArrStart();
                var firstName = true;
                foreach (var name in Enum.GetNames(t).Where(n => Convert.ToInt64(Enum.Parse(t, n), CultureInfo.InvariantCulture) == raw))
                {
                    if (!firstName) o.Comma();
                    firstName = false;
                    o.WriteString(name);
                }
                o.ArrEnd();
                o.ArrEnd();
            }
            o.ArrEnd();
            o.ObjEnd();
        }
        o.ObjEnd();
        o.Raw((byte)'\n');
        File.WriteAllBytes(path, o.WrittenSpan.ToArray());
        return 0;
    }
}
