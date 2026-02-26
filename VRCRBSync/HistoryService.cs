using System;
using System.Collections.ObjectModel;
using Microsoft.UI.Xaml;

namespace VRCRBSync;

public sealed class HistoryEntry
{
    public string Title { get; init; } = string.Empty;
    public string Artist { get; init; } = string.Empty;
    public string Album { get; init; } = string.Empty;
    public DateTime PlayedAt { get; init; } = DateTime.Now;

    public string PlayedAtFormatted => PlayedAt.ToString("HH:mm:ss");
    public Visibility ArtistVisibility => string.IsNullOrWhiteSpace(Artist) ? Visibility.Collapsed : Visibility.Visible;
    public Visibility AlbumVisibility => string.IsNullOrWhiteSpace(Album) ? Visibility.Collapsed : Visibility.Visible;
}

public sealed class HistoryService
{
    private static readonly Lazy<HistoryService> Lazy = new(() => new HistoryService());
    public static HistoryService Instance => Lazy.Value;

    public ObservableCollection<HistoryEntry> Entries { get; } = new();

    private const int MaxEntries = 200;

    private HistoryService() { }

    public void Add(TrackInfo track)
    {
        var entry = new HistoryEntry
        {
            Title = track.Title,
            Artist = track.Artist,
            Album = track.Album,
            PlayedAt = DateTime.Now
        };

        Entries.Insert(0, entry);

        while (Entries.Count > MaxEntries)
        {
            Entries.RemoveAt(Entries.Count - 1);
        }
    }

    public void Clear()
    {
        Entries.Clear();
    }
}
