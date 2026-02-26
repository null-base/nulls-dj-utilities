using System;

namespace VRCRBSync;

public sealed class TrackInfo
{
    public string Title { get; init; } = string.Empty;
    public string Artist { get; init; } = string.Empty;
    public string Album { get; init; } = string.Empty;
    public string FilePath { get; init; } = string.Empty;
    public byte[]? AlbumArt { get; init; }

    public bool Equals(TrackInfo? other)
    {
        if (other is null) return false;
        return Title == other.Title && Artist == other.Artist && FilePath == other.FilePath;
    }

    public override bool Equals(object? obj) => Equals(obj as TrackInfo);
    public override int GetHashCode() => HashCode.Combine(Title, Artist, FilePath);
}
