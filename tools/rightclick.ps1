param(
    [string]$Title = "ResoRust",
    [Parameter(Mandatory = $true)][int]$X,
    [Parameter(Mandatory = $true)][int]$Y,
    [int]$WaitMs = 900
)

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class Win32Click {
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);

    public const uint RIGHTDOWN = 0x0008;
    public const uint RIGHTUP = 0x0010;

    public static IntPtr FindByTitle(string title) {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate(IntPtr hWnd, IntPtr lParam) {
            if (!IsWindowVisible(hWnd)) return true;
            var sb = new StringBuilder(512);
            GetWindowTextW(hWnd, sb, sb.Capacity);
            if (sb.ToString() == title) { found = hWnd; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@

$proc = Get-Process resorust -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne 0 } |
    Select-Object -First 1
$hwnd = if ($proc) { $proc.MainWindowHandle } else { [Win32Click]::FindByTitle($Title) }
if ($hwnd -eq [IntPtr]::Zero) { throw "Janela '$Title' nÃ£o encontrada." }

[Win32Click]::ShowWindow($hwnd, 9) | Out-Null
[Win32Click]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 400

$rect = New-Object Win32Click+RECT
[Win32Click]::GetWindowRect($hwnd, [ref]$rect) | Out-Null

$screenX = $rect.Left + $X
$screenY = $rect.Top + $Y
[Win32Click]::SetCursorPos($screenX, $screenY) | Out-Null
Start-Sleep -Milliseconds 120
[Win32Click]::mouse_event([Win32Click]::RIGHTDOWN, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 80
[Win32Click]::mouse_event([Win32Click]::RIGHTUP, 0, 0, 0, [UIntPtr]::Zero)

Start-Sleep -Milliseconds $WaitMs
Write-Output "Clique em ($X,$Y) -> tela ($screenX,$screenY)"

