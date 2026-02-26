using System;
using System.IO;
using System.Net.Sockets;
using System.Text;

namespace VRCRBSync;

/// <summary>
/// VRChatのOSC chatboxにUTF-8でメッセージを送信する。
/// </summary>
public sealed class OscSenderService : IDisposable
{
    private readonly UdpClient _udpClient;
    private readonly string _host;
    private readonly int _port;
    private string? _lastSentMessage;

    public OscSenderService(string host = "127.0.0.1", int port = 9000)
    {
        _host = host;
        _port = port;
        _udpClient = new UdpClient();
    }

    public void SendTrackInfo(TrackInfo track, AppSettings settings)
    {
        var message = settings.FormatMessage(track);
        if (message == _lastSentMessage) return;

        try
        {
            SendChatboxMessage(message, true, settings.SendNotificationSound);
            _lastSentMessage = message;
        }
        catch
        {
            // VRChat未起動など
        }
    }

    /// <summary>
    /// 最後に送信したメッセージを再送信する（chatbox表示維持用）。
    /// </summary>
    public void ResendLastMessage()
    {
        if (_lastSentMessage is null) return;

        try
        {
            SendChatboxMessage(_lastSentMessage, true, false);
        }
        catch
        {
            // ignore
        }
    }

    public void ClearChatbox()
    {
        try
        {
            SendChatboxMessage("", true, false);
            _lastSentMessage = null;
        }
        catch
        {
            // ignore
        }
    }

    /// <summary>
    /// OSCメッセージを直接バイナリで構築して送信（UTF-8対応）。
    /// /chatbox/input: string(s), bool(T/F), bool(T/F)
    /// </summary>
    private void SendChatboxMessage(string text, bool bypass, bool notify)
    {
        using var ms = new MemoryStream();

        // OSC Address
        WriteOscString(ms, "/chatbox/input");

        // OSC Type Tag String: comma + 's' for string + 'T'/'F' for each bool
        var typeTag = ",s" + (bypass ? "T" : "F") + (notify ? "T" : "F");
        WriteOscString(ms, typeTag);

        // OSC String argument (UTF-8)
        WriteOscString(ms, text);

        // Note: Boolean True/False are encoded only in the type tag (no payload bytes)

        var data = ms.ToArray();
        _udpClient.Send(data, data.Length, _host, _port);
    }

    private static void WriteOscString(MemoryStream ms, string str)
    {
        var bytes = Encoding.UTF8.GetBytes(str);
        ms.Write(bytes, 0, bytes.Length);
        // Null terminator
        ms.WriteByte(0);
        // Pad to 4-byte boundary
        var totalLen = bytes.Length + 1;
        var padding = (4 - totalLen % 4) % 4;
        for (var i = 0; i < padding; i++)
            ms.WriteByte(0);
    }

    public void Dispose()
    {
        ClearChatbox();
        _udpClient.Dispose();
    }
}
