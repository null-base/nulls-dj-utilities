using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace VRCRBSync;

public sealed partial class MainWindow
{
    public MainWindow()
    {
        InitializeComponent();
        Title = "VRCRBSync";
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);
        SetupTitleBar();
    }

    private void MainNav_Loaded(object sender, RoutedEventArgs e)
    {
        MainNav.SelectedItem = MainNav.MenuItems[0];
        ContentFrame.Navigate(typeof(DashboardPage));

        if (MainNav.MenuItems[0] is NavigationViewItem dashboard)
            dashboard.Content = AppResources.NavDashboard;
        if (MainNav.MenuItems[1] is NavigationViewItem history)
            history.Content = AppResources.NavHistory;

        if (MainNav.SettingsItem is NavigationViewItem settingsItem)
            settingsItem.Content = AppResources.NavSettings;
    }

    private void SetupTitleBar()
    {
        var titleBar = AppWindow.TitleBar;
        titleBar.ExtendsContentIntoTitleBar = true;
        titleBar.ButtonBackgroundColor = Windows.UI.Color.FromArgb(0, 0, 0, 0);
        titleBar.ButtonInactiveBackgroundColor = Windows.UI.Color.FromArgb(0, 0, 0, 0);
        titleBar.ButtonForegroundColor = Colors.White;
        titleBar.ButtonHoverForegroundColor = Colors.White;
        titleBar.ButtonPressedForegroundColor = Colors.White;
        titleBar.ButtonInactiveForegroundColor = Colors.Gray;
    }

    private void MainNav_ItemInvoked(NavigationView sender, NavigationViewItemInvokedEventArgs args)
    {
        if (args.IsSettingsInvoked)
        {
            ContentFrame.Navigate(typeof(SettingsPage));
            return;
        }

        if (args.InvokedItemContainer is NavigationViewItem { Tag: string tag })
        {
            var pageType = tag switch
            {
                "Dashboard" => typeof(DashboardPage),
                "History" => typeof(HistoryPage),
                _ => typeof(DashboardPage)
            };

            if (ContentFrame.CurrentSourcePageType != pageType)
                ContentFrame.Navigate(pageType);
        }
    }
}