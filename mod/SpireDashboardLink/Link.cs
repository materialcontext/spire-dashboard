using System.Text.Json;
using System.Text.Json.Serialization;
using Godot;
using MegaCrit.Sts2.Core.Modding;

namespace SpireDashboardLink;

/// <summary>
/// Tells Spire Dashboard where the run in progress is saved, what the
/// current act's boss is and which room the player is in, by keeping a small
/// JSON "link" file up to date.
///
/// Read-only: it patches nothing and calls nothing that changes the game. It
/// looks at the game's state about twice a second, on the main thread, and
/// writes the file only when something changed. The game's own run save
/// remains the record of the run; the dashboard reads it from the path given
/// here.
///
/// The link lives outside the game's folders, at
/// <c>&lt;local app data&gt;/SpireDashboard/game-link.json</c>
/// (Windows <c>%LOCALAPPDATA%</c>, macOS <c>~/Library/Application Support</c>,
/// Linux <c>~/.local/share</c>).
/// </summary>
[ModInitializer(nameof(Initialize))]
public static class Link
{
    public const string Version = "0.2.0";

    /// <summary>Frames between looks at the game (about half a second).</summary>
    private const int Every = 30;

    private static readonly JsonSerializerOptions Json = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    private static string? _path;
    private static string? _written;
    private static int _frame;
    private static bool _warned;

    public static void Initialize()
    {
        try
        {
            _path = Path.Combine(
                System.Environment.GetFolderPath(System.Environment.SpecialFolder.LocalApplicationData),
                "SpireDashboard",
                "game-link.json");
            var tree = (SceneTree)Engine.GetMainLoop();
            tree.Connect(SceneTree.SignalName.ProcessFrame, Callable.From(Tick));
            GD.Print($"[SpireDashboardLink] {Version}: writing {_path}");
        }
        catch (Exception e)
        {
            GD.PrintErr($"[SpireDashboardLink] not started: {e.Message}");
        }
    }

    private static void Tick()
    {
        if (++_frame % Every != 0 || _path == null)
        {
            return;
        }
        try
        {
            var json = JsonSerializer.Serialize(Game.Snapshot(), Json);
            if (json == _written)
            {
                return;
            }
            Directory.CreateDirectory(Path.GetDirectoryName(_path)!);
            // Write then rename, so the dashboard never reads half a file.
            var tmp = _path + ".tmp";
            File.WriteAllText(tmp, json);
            File.Move(tmp, _path, overwrite: true);
            _written = json;
        }
        catch (Exception e)
        {
            // Never disturb the game: note it once and keep trying quietly.
            if (!_warned)
            {
                GD.PrintErr($"[SpireDashboardLink] {e.GetType().Name}: {e.Message}");
                _warned = true;
            }
        }
    }
}

/// <summary>What the dashboard needs to find and follow the run.</summary>
public sealed record LinkFile(
    int Schema,
    string ModVersion,
    string? GameVersion,
    bool InProgress,
    int? ProfileId,
    string? SaveDir,
    string? RunSave,
    long? RunSaveModified,
    string? HistoryDir,
    int? ActIndex,
    string? Act,
    string? Boss,
    string? SecondBoss,
    RoomLink? Room);

/// <summary>
/// The room being played. <c>Encounter</c> or <c>Monsters</c> name a fight,
/// <c>Event</c> an event; any of them can be missing.
/// </summary>
public sealed record RoomLink(string Kind, string? Type, string? Encounter, string? Event, string[]? Monsters);
