using System;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace VRCRBSync;

public sealed partial class SettingsPage : Page
{
    private AppSettings _settings;

    public SettingsPage()
    {
        this.InitializeComponent();
        _settings = AppSettings.Load();
        LoadSettingsToUI();
        FormatBox.TextChanged += FormatBox_TextChanged;
        UpdateUIText();
    }

    private void UpdateUIText()
    {
        SettingsHeaderText.Text = AppResources.SettingsTitle;
        OscSectionText.Text = AppResources.SettingsOscConnection;
        HostBox.Header = AppResources.SettingsOscHost;
        PortBox.Header = AppResources.SettingsOscPort;
        MessageFormatSectionText.Text = AppResources.SettingsMessageFormat;
        FormatBox.Header = AppResources.SettingsFormatTemplate;
        VariablesText.Text = AppResources.SettingsVariables;
        MonitoringSectionText.Text = AppResources.SettingsMonitoring;
        IntervalBox.Header = AppResources.SettingsPollingInterval;
        NotifySoundToggle.Header = AppResources.SettingsSendNotification;
        NotifySoundToggle.OnContent = AppResources.SettingsOn;
        NotifySoundToggle.OffContent = AppResources.SettingsOff;
        PreviewLabelText.Text = AppResources.SettingsPreview;
        SaveSettingsButton.Content = AppResources.SettingsSave;
    }

    private void LoadSettingsToUI()
    {
        HostBox.Text = _settings.OscHost;
        PortBox.Value = _settings.OscPort;
        IntervalBox.Value = _settings.PollingIntervalMs;
        FormatBox.Text = _settings.MessageFormat;
        NotifySoundToggle.IsOn = _settings.SendNotificationSound;
        UpdatePreview();
    }

    private void FormatBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        UpdatePreview();
    }

    private void UpdatePreview()
    {
        var sample = new TrackInfo
        {
            Title = "How Many Boogie",
            Artist = "SKRYU; WAZGOGG; Fuma no KTR",
            Album = "わすれもの"
        };

        var format = FormatBox.Text;
        if (string.IsNullOrWhiteSpace(format))
        {
            PreviewText.Text = "(empty)";
            return;
        }

        PreviewText.Text = format
            .Replace("{title}", sample.Title)
            .Replace("{artist}", sample.Artist)
            .Replace("{album}", sample.Album);
    }

    private async void SaveSettings_Click(object sender, RoutedEventArgs e)
    {
        _settings.OscHost = HostBox.Text.Trim();
        _settings.OscPort = (int)PortBox.Value;
        _settings.PollingIntervalMs = (int)IntervalBox.Value;
        _settings.MessageFormat = FormatBox.Text;
        _settings.SendNotificationSound = NotifySoundToggle.IsOn;
        _settings.Save();

        var dialog = new ContentDialog
        {
            Title = AppResources.SettingsSaved,
            Content = AppResources.SettingsSavedDesc,
            CloseButtonText = "OK",
            XamlRoot = this.XamlRoot
        };
        await dialog.ShowAsync();
    }
}
