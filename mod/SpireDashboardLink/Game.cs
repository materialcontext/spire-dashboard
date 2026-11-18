using System.Reflection;
using Godot;

namespace SpireDashboardLink;

/// <summary>
/// Reads the few things the link needs from the running game, by name.
///
/// Reflection rather than compiled references: the mod builds without the
/// game's assemblies, and a member the game renames in an update makes only
/// that field come back empty instead of stopping the mod. Every read is a
/// plain getter; nothing here changes the game.
/// </summary>
internal static class Game
{
    private const BindingFlags Any =
        BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static;

    private static readonly Assembly? Sts2 =
        AppDomain.CurrentDomain.GetAssemblies().FirstOrDefault(a => a.GetName().Name == "sts2");

    private static string? _gameVersion;

    public static LinkFile Snapshot()
    {
        var runManager = Static("MegaCrit.Sts2.Core.Runs.RunManager", "Instance");
        var inProgress = Get(runManager, "IsInProgress") is true;
        var state = inProgress ? Call(runManager, "DebugOnlyGetState") : null;
        var act = Get(state, "Act");

        var profileId = Get(Static("MegaCrit.Sts2.Core.Saves.SaveManager", "Instance"), "CurrentProfileId") as int?;
        var saveDir = SaveDir(profileId);
        var runSave = saveDir == null ? null : Path.Combine(saveDir, "current_run.save");
        long? modified = runSave != null && File.Exists(runSave)
            ? new DateTimeOffset(File.GetLastWriteTimeUtc(runSave)).ToUnixTimeMilliseconds()
            : null;

        return new LinkFile(
            Schema: 1,
            ModVersion: Link.Version,
            GameVersion: GameVersion(),
            InProgress: inProgress,
            ProfileId: profileId,
            SaveDir: saveDir,
            RunSave: modified == null ? null : runSave,
            RunSaveModified: modified,
            HistoryDir: saveDir == null ? null : Path.Combine(saveDir, "history"),
            ActIndex: Get(state, "CurrentActIndex") as int?,
            Act: Entry(Get(act, "Id")),
            Boss: Entry(Get(Get(act, "BossEncounter"), "Id")),
            SecondBoss: Entry(Get(Get(act, "SecondBossEncounter"), "Id")));
    }

    /// <summary>
    /// The profile's <c>saves</c> folder: where <c>progress.save</c>,
    /// <c>current_run.save</c> and <c>history/</c> live. With mods on, the
    /// game keeps these under a separate <c>modded/</c> profile.
    /// </summary>
    private static string? SaveDir(int? profileId)
    {
        if (profileId == null)
        {
            return null;
        }
        var type = Sts2?.GetType("MegaCrit.Sts2.Core.Saves.Managers.ProgressSaveManager");
        var method = type?.GetMethod("GetProgressPathForProfile", Any, [typeof(int)]);
        if (method?.Invoke(null, [profileId.Value]) is not string progress || progress.Length == 0)
        {
            return null;
        }
        if (progress.StartsWith("user://", StringComparison.Ordinal) || progress.StartsWith("res://", StringComparison.Ordinal))
        {
            progress = ProjectSettings.GlobalizePath(progress);
        }
        else if (!Path.IsPathRooted(progress))
        {
            progress = Path.Combine(OS.GetUserDataDir(), progress);
        }
        var dir = Path.GetDirectoryName(progress);
        return dir != null && Directory.Exists(dir) ? dir : null;
    }

    /// <summary>The game's version from <c>release_info.json</c> beside it.</summary>
    private static string? GameVersion()
    {
        if (_gameVersion != null)
        {
            return _gameVersion;
        }
        try
        {
            var exe = OS.GetExecutablePath();
            // macOS: the executable is inside the app bundle's Contents/MacOS.
            foreach (var dir in new[] { Path.GetDirectoryName(exe), Path.Combine(Path.GetDirectoryName(exe) ?? "", "..", "Resources") })
            {
                var file = dir == null ? null : Path.Combine(dir, "release_info.json");
                if (file != null && File.Exists(file))
                {
                    using var doc = System.Text.Json.JsonDocument.Parse(File.ReadAllText(file));
                    _gameVersion = doc.RootElement.TryGetProperty("version", out var v) ? v.GetString() : null;
                    break;
                }
            }
        }
        catch (Exception)
        {
            _gameVersion = null;
        }
        return _gameVersion;
    }

    /// <summary>A model id as the save files write it, e.g. <c>ENCOUNTER.VANTOM_BOSS</c>.</summary>
    private static string? Entry(object? id)
    {
        if (id == null)
        {
            return null;
        }
        var entry = Get(id, "Entry") as string;
        var category = Get(id, "Category")?.ToString();
        return entry == null ? id.ToString() : category == null ? entry : $"{category}.{entry}";
    }

    private static object? Static(string type, string member) => Get(Sts2?.GetType(type), member, statics: true);

    private static object? Get(object? target, string member, bool statics = false)
    {
        if (target == null)
        {
            return null;
        }
        var type = statics ? target as Type : target.GetType();
        var instance = statics ? null : target;
        try
        {
            if (type?.GetProperty(member, Any) is { } p)
            {
                return p.GetValue(instance);
            }
            if (type?.GetField(member, Any) is { } f)
            {
                return f.GetValue(instance);
            }
        }
        catch (Exception)
        {
            // A getter that throws (e.g. no act yet) reads as empty.
        }
        return null;
    }

    private static object? Call(object? target, string method)
    {
        try
        {
            return target?.GetType().GetMethod(method, Any, Type.EmptyTypes)?.Invoke(target, null);
        }
        catch (Exception)
        {
            return null;
        }
    }
}
