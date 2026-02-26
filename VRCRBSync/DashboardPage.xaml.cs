using System;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Threading.Tasks;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Storage.Streams;

namespace VRCRBSync;

public sealed partial class DashboardPage : Page
{
    private readonly GlobalSyncManager _syncManager = GlobalSyncManager.Instance;

    public DashboardPage()
    {
        InitializeComponent();
        UpdateUIText();
    }

    private void UpdateUIText()
    {
        HeaderText.Text = AppResources.DashboardNowPlaying;
        StartButton.Content = AppResources.DashboardStartSync;
        StopButton.Content = AppResources.DashboardStopSync;
        StatusText.Text = AppResources.DashboardOscDisconnected;
    }

    protected override void OnNavigatedTo(Microsoft.UI.Xaml.Navigation.NavigationEventArgs e)
    {
        base.OnNavigatedTo(e);
        _syncManager.SyncStateChanged += OnSyncStateChanged;
        _syncManager.TrackUpdated += OnTrackUpdated;
        UpdateUIFromSyncState();

        if (_syncManager.CachedAlbumArt is { Length: > 0 } art)
            DispatcherQueue.TryEnqueue(async () => await UpdateAlbumArt(art));
    }

    protected override void OnNavigatedFrom(Microsoft.UI.Xaml.Navigation.NavigationEventArgs e)
    {
        base.OnNavigatedFrom(e);
        _syncManager.SyncStateChanged -= OnSyncStateChanged;
        _syncManager.TrackUpdated -= OnTrackUpdated;
    }

    private async void StartSync_Click(object sender, RoutedEventArgs e)
    {
        if (_syncManager.IsSyncing) return;
        SetSyncUI(true);
        await _syncManager.StartSyncAsync();
    }

    private void StopSync_Click(object sender, RoutedEventArgs e) => _syncManager.StopSync();

    private void OnSyncStateChanged(SyncStateChangedEventArgs args)
    {
        DispatcherQueue.TryEnqueue(() =>
        {
            SetSyncUI(args.IsSyncing);
            StatusText.Text = args.IsSyncing
                ? _syncManager.StatusText
                : AppResources.DashboardOscDisconnected;
        });
    }

    private void OnTrackUpdated(TrackInfo? track)
    {
        DispatcherQueue.TryEnqueue(async () =>
        {
            if (track is not null)
            {
                TitleText.Text = string.IsNullOrWhiteSpace(track.Title) ? AppResources.DashboardWaitingForTrack : track.Title;
                ArtistText.Text = track.Artist;
                ArtistText.Visibility = string.IsNullOrWhiteSpace(track.Artist) ? Visibility.Collapsed : Visibility.Visible;
                AlbumText.Text = track.Album;
                AlbumText.Visibility = string.IsNullOrWhiteSpace(track.Album) ? Visibility.Collapsed : Visibility.Visible;
                await UpdateAlbumArt(track.AlbumArt);
            }
            else
            {
                ShowNoTrack();
            }
        });
    }

    private void SetSyncUI(bool syncing)
    {
        StartButton.IsEnabled = !syncing;
        StopButton.IsEnabled = syncing;
        StatusDot.Fill = new SolidColorBrush(syncing ? Microsoft.UI.Colors.Green : Microsoft.UI.Colors.Red);
    }

    private void ShowNoTrack()
    {
        TitleText.Text = AppResources.DashboardWaitingForTrack;
        ArtistText.Text = AppResources.DashboardRekordboxNotRunning;
        ArtistText.Visibility = Visibility.Visible;
        AlbumText.Text = "";
        AlbumText.Visibility = Visibility.Collapsed;
        AlbumArtBrush.ImageSource = null;
        AlbumArtPlaceholder.Visibility = Visibility.Visible;
    }

    private void UpdateUIFromSyncState()
    {
        SetSyncUI(_syncManager.IsSyncing);
        StatusText.Text = _syncManager.IsSyncing ? _syncManager.StatusText : AppResources.DashboardOscDisconnected;

        var track = _syncManager.CurrentTrack;
        if (track is not null)
        {
            TitleText.Text = string.IsNullOrWhiteSpace(track.Title) ? AppResources.DashboardWaitingForTrack : track.Title;
            ArtistText.Text = track.Artist;
            ArtistText.Visibility = string.IsNullOrWhiteSpace(track.Artist) ? Visibility.Collapsed : Visibility.Visible;
            AlbumText.Text = track.Album;
            AlbumText.Visibility = string.IsNullOrWhiteSpace(track.Album) ? Visibility.Collapsed : Visibility.Visible;
        }
        else
        {
            TitleText.Text = AppResources.DashboardWaitingForTrack;
            ArtistText.Text = AppResources.DashboardRekordboxNotRunning;
            ArtistText.Visibility = Visibility.Visible;
            AlbumText.Text = "";
            AlbumText.Visibility = Visibility.Collapsed;
        }
    }

    private async Task UpdateAlbumArt(byte[]? albumArtData)
    {
        if (albumArtData is not { Length: > 0 })
        {
            AlbumArtBrush.ImageSource = null;
            AlbumArtPlaceholder.Visibility = Visibility.Visible;
            return;
        }

        try
        {
            var bitmapImage = new BitmapImage();
            using var stream = new InMemoryRandomAccessStream();
            await stream.WriteAsync(albumArtData.AsBuffer()).AsTask();
            stream.Seek(0);
            await bitmapImage.SetSourceAsync(stream).AsTask();
            AlbumArtBrush.ImageSource = bitmapImage;
            AlbumArtPlaceholder.Visibility = Visibility.Collapsed;
        }
        catch
        {
            AlbumArtBrush.ImageSource = null;
            AlbumArtPlaceholder.Visibility = Visibility.Visible;
        }
    }
}
