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
    public string MessageFormat { get; set; } = "♫ {title} - {artist}";
    public bool SendNotificationSound { get; set; }

    public string FormatMessage(TrackInfo track)
    {
        var result = MessageFormat;

        // 空のフィールドはプレースホルダーごと除去し、余分な区切りも整理
        result = ReplaceOrRemove(result, "{title}", track.Title);
        result = ReplaceOrRemove(result, "{artist}", track.Artist);
        result = ReplaceOrRemove(result, "{album}", track.Album);

        // 連続する区切り文字（" - "）を整理
        while (result.Contains(" -  - "))
            result = result.Replace(" -  - ", " - ");
        result = result.Trim(' ', '-');
        // 先頭のシンボル(♫等)の後のスペースと区切りを整理
        if (result.Length > 0 && result[^1] == '-')
            result = result.TrimEnd(' ', '-');

        return result.Trim();
    }

    private static string ReplaceOrRemove(string format, string placeholder, string value)
    {
        if (!string.IsNullOrWhiteSpace(value))
            return format.Replace(placeholder, value);

        return format.Replace(placeholder, "");
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
