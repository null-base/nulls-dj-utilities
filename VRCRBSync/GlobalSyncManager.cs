using System;
using System.Threading;
using System.Threading.Tasks;

namespace VRCRBSync;

public sealed class GlobalSyncManager
{
    private static readonly Lazy<GlobalSyncManager> _lazy = new(() => new GlobalSyncManager());
    public static GlobalSyncManager Instance => _lazy.Value;

    private bool _isSyncing;
    private CancellationTokenSource? _monitorCts;
    private readonly RekordboxMonitor _monitor = new();
    private OscSenderService? _oscSender;
    private TrackInfo? _lastTrack;
    private byte[]? _cachedAlbumArt;
    private AppSettings _settings = AppSettings.Load();

    public event Action<SyncStateChangedEventArgs>? SyncStateChanged;
    public event Action<TrackInfo?>? TrackUpdated;

    public bool IsSyncing => _isSyncing;
    public TrackInfo? CurrentTrack => _lastTrack;
    public byte[]? CachedAlbumArt => _cachedAlbumArt;
    public string StatusText { get; private set; } = "";

    private GlobalSyncManager()
    {
        StartBackgroundMonitoring();
    }

    private void StartBackgroundMonitoring()
    {
        _monitorCts = new CancellationTokenSource();
        _ = RunBackgroundLoop(_monitorCts.Token);
    }

    private async Task RunBackgroundLoop(CancellationToken token)
    {
        var resendCounter = 0;

        while (!token.IsCancellationRequested)
        {
            try
            {
                var track = await Task.Run(() => _monitor.GetCurrentTrack(), token);

                if (track is not null && !track.Equals(_lastTrack))
                {
                    _lastTrack = track;
                    if (track.AlbumArt is { Length: > 0 })
                        _cachedAlbumArt = track.AlbumArt;

                    if (_isSyncing)
                        _oscSender?.SendTrackInfo(track, _settings);

                    resendCounter = 0;
                    HistoryService.Instance.Add(track);
                    TrackUpdated?.Invoke(track);
                }
                else if (track is null && _lastTrack is not null)
                {
                    _lastTrack = null;
                    TrackUpdated?.Invoke(null);
                }
                else if (_isSyncing && _lastTrack is not null)
                {
                    // VRChat chatboxは約15秒で消えるため、一定間隔で再送信
                    resendCounter++;
                    var resendInterval = 10000 / _settings.PollingIntervalMs; // 約10秒ごと
                    if (resendInterval < 1) resendInterval = 1;
                    if (resendCounter >= resendInterval)
                    {
                        _oscSender?.ResendLastMessage();
                        resendCounter = 0;
                    }
                }
            }
            catch (OperationCanceledException) { break; }
            catch { /* ignore */ }

            try { await Task.Delay(_settings.PollingIntervalMs, token); }
            catch (OperationCanceledException) { break; }
        }
    }

    public Task StartSyncAsync()
    {
        if (_isSyncing) return Task.CompletedTask;

        _settings = AppSettings.Load();
        _isSyncing = true;
        _oscSender = new OscSenderService(_settings.OscHost, _settings.OscPort);
        StatusText = $"{AppResources.DashboardSyncActive} ({_settings.OscHost}:{_settings.OscPort})";

        // 現在のトラックがあれば即座に送信
        if (_lastTrack is not null)
            _oscSender.SendTrackInfo(_lastTrack, _settings);

        SyncStateChanged?.Invoke(new SyncStateChangedEventArgs { IsSyncing = true });
        return Task.CompletedTask;
    }

    public void StopSync()
    {
        if (!_isSyncing) return;

        _isSyncing = false;
        _oscSender?.Dispose();
        _oscSender = null;
        StatusText = AppResources.DashboardOscDisconnected;

        SyncStateChanged?.Invoke(new SyncStateChangedEventArgs { IsSyncing = false });
    }

    public void Dispose()
    {
        StopSync();
        _monitorCts?.Cancel();
        _monitorCts?.Dispose();
    }
}

public class SyncStateChangedEventArgs
{
    public bool IsSyncing { get; set; }
}
