using System;
using System.IO;
using System.Text;
using Newtonsoft.Json;

namespace VRCRBSync;

public sealed class AppSettings
{
    private static readonly string SettingsDir = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "VRCRBSync");

    private static readonly string SettingsPath = Path.Combine(SettingsDir, "settings.json");

    public string OscHost { get; set; } = "127.0.0.1";
    public int OscPort { get; set; } = 9000;
    public int PollingIntervalMs { get; set; } = 3000;
    public string MessageFormat { get; set; } = "♫ {title} - {artist} - {album}";
    public bool SendNotificationSound { get; set; } = false;

    public string FormatMessage(TrackInfo track)
    {
        return MessageFormat
            .Replace("{title}", track.Title)
            .Replace("{artist}", track.Artist)
            .Replace("{album}", track.Album);
    }

    public void Save()
    {
        try
        {
            Directory.CreateDirectory(SettingsDir);
            var json = JsonConvert.SerializeObject(this, Formatting.Indented);
            File.WriteAllText(SettingsPath, json, Encoding.UTF8);
        }
        catch
        {
        }
    }

    public static AppSettings Load()
    {
        try
        {
            if (File.Exists(SettingsPath))
            {
                var json = File.ReadAllText(SettingsPath, Encoding.UTF8);
                return JsonConvert.DeserializeObject<AppSettings>(json) ?? new AppSettings();
            }
        }
        catch
        {
        }

        return new AppSettings();
    }
}
