using System;
using System.IO;
using System.Diagnostics;
using Microsoft.Data.Sqlite;

namespace VRCRBSync;

public sealed class RekordboxMonitor : IDisposable
{
    private static readonly string RekordboxDbDir = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "Pioneer", "rekordbox");

    private static readonly string MasterDbPath = Path.Combine(RekordboxDbDir, "master.db");
    private static readonly string ShareDir = Path.Combine(RekordboxDbDir, "share");

    private const string DbPassword = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";

    public static bool IsRekordboxRunning()
    {
        var processes = Process.GetProcessesByName("rekordbox");
        var running = processes.Length > 0;
        foreach (var p in processes) p.Dispose();
        return running;
    }


    private static string BuildConnectionString()
    {
        return new SqliteConnectionStringBuilder
        {
            DataSource = MasterDbPath,
            Mode = SqliteOpenMode.ReadOnly,
            Password = DbPassword
        }.ToString();
    }


    public TrackInfo? GetCurrentTrack()
    {
        if (!IsRekordboxRunning()) return null;
        if (!File.Exists(MasterDbPath)) return null;

        try
        {
            SQLitePCL.Batteries_V2.Init();

            using var conn = new SqliteConnection(BuildConnectionString());
            conn.Open();

            var latestEntry = GetLatestSongHistory(conn);
            if (latestEntry is null) return null;

            var (songHistoryId, contentId) = latestEntry.Value;

            return GetTrackInfoByContentId(conn, contentId);
        }
        catch
        {
            return null;
        }
    }

    
    private static (string Id, string ContentId)? GetLatestSongHistory(SqliteConnection conn)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"
            SELECT ID, ContentID
            FROM djmdSongHistory
            ORDER BY created_at DESC
            LIMIT 1";

        using var reader = cmd.ExecuteReader();
        if (!reader.Read()) return null;

        var id = reader.GetString(0);
        var contentId = reader.GetString(1);
        return (id, contentId);
    }

  private TrackInfo? GetTrackInfoByContentId(SqliteConnection conn, string contentId)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"
            SELECT c.Title, c.FolderPath,
                   a.Name AS ArtistName,
                   al.Name AS AlbumName,
                   c.ImagePath
            FROM djmdContent c
            LEFT JOIN djmdArtist a ON c.ArtistID = a.ID
            LEFT JOIN djmdAlbum al ON c.AlbumID = al.ID
            WHERE c.ID = @contentId";
        cmd.Parameters.AddWithValue("@contentId", contentId);

        using var reader = cmd.ExecuteReader();
        if (!reader.Read()) return null;

        var title = reader.IsDBNull(0) ? "" : reader.GetString(0);
        var folderPath = reader.IsDBNull(1) ? "" : reader.GetString(1);
        var artistName = reader.IsDBNull(2) ? "" : reader.GetString(2);
        var albumName = reader.IsDBNull(3) ? "" : reader.GetString(3);
        var imagePath = reader.IsDBNull(4) ? null : reader.GetString(4);

      byte[]? albumArt = LoadAlbumArt(imagePath);

        return new TrackInfo
        {
            Title = title,
            Artist = artistName,
            Album = albumName,
            FilePath = folderPath.Replace('/', Path.DirectorySeparatorChar),
            AlbumArt = albumArt
        };
    }

    private static byte[]? LoadAlbumArt(string? imagePath)
    {
        if (string.IsNullOrWhiteSpace(imagePath)) return null;

        try
        {
            var relativePath = imagePath.TrimStart('/').Replace('/', Path.DirectorySeparatorChar);
            var fullPath = Path.Combine(ShareDir, relativePath);

            if (File.Exists(fullPath))
            {
                return File.ReadAllBytes(fullPath);
            }
        }
        catch
        {
        }

        return null;
    }

    public void Dispose()
    {
    }
}
