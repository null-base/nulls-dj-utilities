using System.Globalization;

namespace VRCRBSync;

public sealed class AppResources
{
    private static string _currentLanguage = CultureInfo.CurrentUICulture.TwoLetterISOLanguageName;

    public static string CurrentLanguage
    {
        get => _currentLanguage;
        set
        {
            if (value != _currentLanguage)
            {
                _currentLanguage = value;
                AppSettings.Load().Save();
            }
        }
    }

    // Dashboard Page
    public static string DashboardNowPlaying => GetString("dashboard_now_playing");
    public static string DashboardWaitingForTrack => GetString("dashboard_waiting_for_track");
    public static string DashboardRekordboxNotRunning => GetString("dashboard_rekordbox_not_running");
    public static string DashboardOscDisconnected => GetString("dashboard_osc_disconnected");
    public static string DashboardSyncActive => GetString("dashboard_sync_active");
    public static string DashboardStartSync => GetString("dashboard_start_sync");
    public static string DashboardStopSync => GetString("dashboard_stop_sync");

    // Settings Page
    public static string SettingsTitle => GetString("settings_title");
    public static string SettingsOscConnection => GetString("settings_osc_connection");
    public static string SettingsOscHost => GetString("settings_osc_host");
    public static string SettingsOscPort => GetString("settings_osc_port");
    public static string SettingsPollingInterval => GetString("settings_polling_interval");
    public static string SettingsMessageFormat => GetString("settings_message_format");
    public static string SettingsFormatTemplate => GetString("settings_format_template");
    public static string SettingsSendNotification => GetString("settings_send_notification");
    public static string SettingsMonitoring => GetString("settings_monitoring");
    public static string SettingsPreview => GetString("settings_preview");
    public static string SettingsSave => GetString("settings_save");
    public static string SettingsSaved => GetString("settings_saved");
    public static string SettingsSavedDesc => GetString("settings_saved_desc");
    public static string SettingsVariables => GetString("settings_variables");
    public static string SettingsOn => GetString("settings_on");
    public static string SettingsOff => GetString("settings_off");

    // History Page
    public static string HistoryTitle => GetString("history_title");
    public static string HistoryClearHistory => GetString("history_clear_history");
    public static string HistoryEmpty => GetString("history_empty");
    public static string HistoryNoTracks => GetString("history_no_tracks");

    // Navigation
    public static string NavDashboard => GetString("nav_dashboard");
    public static string NavHistory => GetString("nav_history");
    public static string NavSettings => GetString("nav_settings");

    private static string GetString(string key) => _currentLanguage switch
    {
        "ja" => GetJapanese(key),
        _ => GetEnglish(key),
    };

    private static string GetEnglish(string key) => key switch
    {
        // Dashboard
        "dashboard_now_playing" => "Now Playing",
        "dashboard_waiting_for_track" => "Waiting for track...",
        "dashboard_rekordbox_not_running" => "No music is playing.",
        "dashboard_osc_disconnected" => "OSC Disconnected",
        "dashboard_sync_active" => "Sync Active",
        "dashboard_start_sync" => "Start Sync",
        "dashboard_stop_sync" => "Stop Sync",

        // Settings
        "settings_title" => "Settings",
        "settings_osc_connection" => "OSC Connection",
        "settings_osc_host" => "OSC Host",
        "settings_osc_port" => "OSC Port",
        "settings_polling_interval" => "Polling Interval (ms)",
        "settings_message_format" => "Message Format",
        "settings_format_template" => "Format template",
        "settings_send_notification" => "Notification sound on track change",
        "settings_monitoring" => "Monitoring",
        "settings_preview" => "Preview:",
        "settings_save" => "Save Settings",
        "settings_saved" => "Settings saved",
        "settings_saved_desc" => "Changes will take effect on the next sync start.",
        "settings_variables" => "Variables: {title}, {artist}, {album}",
        "settings_on" => "On",
        "settings_off" => "Off",

        // History
        "history_title" => "History",
        "history_clear_history" => "Clear History",
        "history_empty" => "No tracks yet",
        "history_no_tracks" => "No tracks in history",

        // Navigation
        "nav_dashboard" => "Dashboard",
        "nav_history" => "History",
        "nav_settings" => "Settings",

        _ => key,
    };

    private static string GetJapanese(string key) => key switch
    {
        // Dashboard
        "dashboard_now_playing" => "現在再生中",
        "dashboard_waiting_for_track" => "トラック待機中...",
        "dashboard_rekordbox_not_running" => "楽曲が再生されていません",
        "dashboard_osc_disconnected" => "OSC 接続解除",
        "dashboard_sync_active" => "同期活動中",
        "dashboard_start_sync" => "同期開始",
        "dashboard_stop_sync" => "同期停止",

        // Settings
        "settings_title" => "設定",
        "settings_osc_connection" => "OSC接続",
        "settings_osc_host" => "OSCホスト",
        "settings_osc_port" => "OSCポート",
        "settings_polling_interval" => "ポーリング間隔 (ms)",
        "settings_message_format" => "メッセージフォーマット",
        "settings_format_template" => "フォーマットテンプレート",
        "settings_send_notification" => "トラック変更時に通知音を再生",
        "settings_monitoring" => "モニタリング",
        "settings_preview" => "プレビュー:",
        "settings_save" => "設定を保存",
        "settings_saved" => "設定が保存されました",
        "settings_saved_desc" => "次回の同期開始時に変更が反映されます。",
        "settings_variables" => "変数: {title}, {artist}, {album}",
        "settings_on" => "オン",
        "settings_off" => "オフ",

        // History
        "history_title" => "履歴",
        "history_clear_history" => "履歴をクリア",
        "history_empty" => "まだトラックがありません",
        "history_no_tracks" => "履歴にトラックがありません",

        // Navigation
        "nav_dashboard" => "ダッシュボード",
        "nav_history" => "履歴",
        "nav_settings" => "設定",

        _ => key,
    };
}

