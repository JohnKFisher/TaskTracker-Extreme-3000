Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WorkAreaHelper {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SystemParametersInfo(uint uiAction, uint uiParam, ref RECT pvParam, uint fWinIni);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr SendMessageTimeout(
        IntPtr hWnd, uint Msg, IntPtr wParam, string lParam,
        uint fuFlags, uint uTimeout, out IntPtr lpdwResult);

    public static string Get() {
        RECT r = new RECT();
        SystemParametersInfo(0x0030, 0, ref r, 0);
        return r.Left + "," + r.Top + "," + r.Right + "," + r.Bottom;
    }

    public static bool Set(int l, int t, int r, int b) {
        RECT rect = new RECT { Left = l, Top = t, Right = r, Bottom = b };
        // SPIF_UPDATEINIFILE | SPIF_SENDCHANGE = 0x0003
        bool ok = SystemParametersInfo(0x002F, 0, ref rect, 0x0003);

        // Broadcast WM_SETTINGCHANGE to all top-level windows
        // HWND_BROADCAST = 0xFFFF, WM_SETTINGCHANGE = 0x001A
        // SMTO_ABORTIFHUNG = 0x0002
        IntPtr result;
        SendMessageTimeout(
            (IntPtr)0xFFFF, 0x001A, IntPtr.Zero, "Environment",
            0x0002, 1000, out result);

        return ok;
    }
}
"@

if ($args[0] -eq "get") {
    [WorkAreaHelper]::Get()
} elseif ($args[0] -eq "set") {
    $l = [int]$args[1]
    $t = [int]$args[2]
    $r = [int]$args[3]
    $b = [int]$args[4]
    [WorkAreaHelper]::Set($l, $t, $r, $b)
}
