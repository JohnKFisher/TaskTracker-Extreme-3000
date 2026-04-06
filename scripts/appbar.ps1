Add-Type @"
using System;
using System.Runtime.InteropServices;

public class AppBarHelper {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct APPBARDATA {
        public uint cbSize;
        public IntPtr hWnd;
        public uint uCallbackMessage;
        public uint uEdge;
        public RECT rc;
        public IntPtr lParam;
    }

    [DllImport("shell32.dll")]
    public static extern uint SHAppBarMessage(uint dwMessage, ref APPBARDATA pData);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SystemParametersInfo(uint a, uint b, ref RECT c, uint d);

    public const uint ABM_NEW = 0x00;
    public const uint ABM_REMOVE = 0x01;
    public const uint ABM_QUERYPOS = 0x02;
    public const uint ABM_SETPOS = 0x03;
    public const uint ABE_RIGHT = 2;

    public static string Register(IntPtr hwnd, int left, int top, int right, int bottom) {
        APPBARDATA abd = new APPBARDATA();
        abd.cbSize = (uint)Marshal.SizeOf(abd);
        abd.hWnd = hwnd;
        abd.uCallbackMessage = 0x8001;

        uint regResult = SHAppBarMessage(ABM_NEW, ref abd);

        abd.uEdge = ABE_RIGHT;
        abd.rc.Left = left;
        abd.rc.Top = top;
        abd.rc.Right = right;
        abd.rc.Bottom = bottom;
        SHAppBarMessage(ABM_QUERYPOS, ref abd);

        // Force our left edge after query (Windows may adjust it)
        abd.rc.Left = left;
        SHAppBarMessage(ABM_SETPOS, ref abd);

        // Read back work area
        RECT wa = new RECT();
        SystemParametersInfo(0x0030, 0, ref wa, 0);
        return "reg=" + regResult + " reserved=" + abd.rc.Left + "," + abd.rc.Top + "," + abd.rc.Right + "," + abd.rc.Bottom + " workarea=" + wa.Left + "," + wa.Top + "," + wa.Right + "," + wa.Bottom;
    }

    public static string Remove(IntPtr hwnd) {
        APPBARDATA abd = new APPBARDATA();
        abd.cbSize = (uint)Marshal.SizeOf(abd);
        abd.hWnd = hwnd;
        SHAppBarMessage(ABM_REMOVE, ref abd);

        RECT wa = new RECT();
        SystemParametersInfo(0x0030, 0, ref wa, 0);
        return "removed workarea=" + wa.Left + "," + wa.Top + "," + wa.Right + "," + wa.Bottom;
    }

    public static string GetWorkArea() {
        RECT rc = new RECT();
        SystemParametersInfo(0x0030, 0, ref rc, 0);
        return rc.Left + "," + rc.Top + "," + rc.Right + "," + rc.Bottom;
    }
}
"@

$action = $args[0]

if ($action -eq "register") {
    $hwnd = [IntPtr]::new([Convert]::ToInt64($args[1], 16))
    [AppBarHelper]::Register($hwnd, [int]$args[2], [int]$args[3], [int]$args[4], [int]$args[5])
} elseif ($action -eq "remove") {
    $hwnd = [IntPtr]::new([Convert]::ToInt64($args[1], 16))
    [AppBarHelper]::Remove($hwnd)
} elseif ($action -eq "get") {
    [AppBarHelper]::GetWorkArea()
}
