using System.Diagnostics;
using System.Text;

// Minimal C#/.NET 10 baseline for exp_102.
// Usage: LegionBench [hello|idle <seconds>|bench <frames> <changes> <write:0|1>]
var argv = Environment.GetCommandLineArgs().Skip(1).ToArray();
var mode = argv.Length > 0 ? argv[0] : "hello";
if (mode == "idle")
{
    Thread.Sleep(int.Parse(argv[1]) * 1000);
    return;
}
if (mode == "bench")
{
    const int W = 120, H = 50, N = W * H;
    int frames = int.Parse(argv[1]), changes = int.Parse(argv[2]);
    bool write = argv.Length > 3 && argv[3] == "1";
    var front = new Cell[N];
    var back = new Cell[N];
    var sb = new StringBuilder(N * 24);
    ulong state = 0x9e3779b97f4a7c15UL;
    long totalNs = 0, totalBytes = 0;
    var stdout = Console.OpenStandardOutput();
    for (int f = 0; f < frames; f++)
    {
        for (int k = 0; k < changes; k++)
        {
            state ^= state << 13; state ^= state >> 7; state ^= state << 17;
            int i = (int)(state % (ulong)N);
            back[i] = new Cell((char)('A' + (f + k) % 26), (byte)((state >> 11) % 16));
        }
        var sw = Stopwatch.StartNew();
        sb.Clear();
        sb.Append("\x1b[?2026h");
        byte lastFg = 255;
        for (int y = 0; y < H; y++)
        {
            bool inRun = false;
            for (int x = 0; x < W; x++)
            {
                int i = y * W + x;
                if (front[i].Equals(back[i])) { inRun = false; continue; }
                if (!inRun) { sb.Append("\x1b[").Append(y + 1).Append(';').Append(x + 1).Append('H'); inRun = true; lastFg = 255; }
                var c = back[i];
                if (c.Fg != lastFg) { sb.Append("\x1b[38;5;").Append(c.Fg).Append('m'); lastFg = c.Fg; }
                sb.Append(c.Glyph);
            }
        }
        sb.Append("\x1b[0m\x1b[?2026l");
        if (write)
        {
            var bytes = Encoding.UTF8.GetBytes(sb.ToString());
            stdout.Write(bytes, 0, bytes.Length);
            totalBytes += bytes.Length;
        }
        else totalBytes += sb.Length;
        totalNs += sw.ElapsedTicks * (1_000_000_000L / Stopwatch.Frequency);
        front = (Cell[])back.Clone();
    }
    Console.Error.WriteLine($"frames={frames} changes/frame={changes} write={(write ? 1 : 0)} mean_ms={totalNs / (double)frames / 1e6:F4} total_bytes={totalBytes} bytes/frame={totalBytes / frames:F0}");
    return;
}
Console.WriteLine("hello");

readonly record struct Cell(char Glyph, byte Fg);
