using System.IO.Compression;

public static class Output
{
    /// <summary>A UTF-8 writer with \n line ends, gzipped when the path ends in .gz.</summary>
    public static StreamWriter Open(string path)
    {
        Stream s = File.Create(path);
        if (path.EndsWith(".gz")) s = new GZipStream(s, CompressionLevel.SmallestSize);
        return new StreamWriter(s, new System.Text.UTF8Encoding(false)) { NewLine = "\n" };
    }
}
