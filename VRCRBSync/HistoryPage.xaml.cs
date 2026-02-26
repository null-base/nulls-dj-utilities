using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace VRCRBSync;

public sealed partial class HistoryPage : Page
{
    public HistoryPage()
    {
        InitializeComponent();
        HistoryList.ItemsSource = HistoryService.Instance.Entries;
        HistoryService.Instance.Entries.CollectionChanged += (_, _) => DispatcherQueue.TryEnqueue(UpdateEmptyState);
        UpdateEmptyState();
        UpdateUIText();
    }

    private void UpdateUIText()
    {
        HistoryHeaderText.Text = AppResources.HistoryTitle;
        ClearHistoryButton.Content = AppResources.HistoryClearHistory;
        EmptyPlaceholder.Text = AppResources.HistoryNoTracks;
    }

    private void UpdateEmptyState()
    {
        var hasEntries = HistoryService.Instance.Entries.Count > 0;
        EmptyPlaceholder.Visibility = hasEntries ? Visibility.Collapsed : Visibility.Visible;
        HistoryList.Visibility = hasEntries ? Visibility.Visible : Visibility.Collapsed;
    }

    private void ClearHistory_Click(object sender, RoutedEventArgs e) => HistoryService.Instance.Clear();
}
