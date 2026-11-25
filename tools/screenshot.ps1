param(
    [string]$OutFile = "$env:TEMP\resorust-shot.png",
    [string]$Title = "ResoRust",
    [int]$WaitSeconds = 20
)

Add-Type -AssemblyName System.Windows.Forms, System.Drawing

Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class Win32 {
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);

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

$exe = Join-Path (Split-Path $PSScriptRoot -Parent) "target\release\resorust.exe"
if (-not (Test-Path $exe)) { throw "Executável não encontrado: $exe" }

$existing = Get-Process resorust -ErrorAction SilentlyContinue
if (-not $existing) { Start-Process -FilePath $exe | Out-Null }

$deadline = (Get-Date).AddSeconds($WaitSeconds)
$hwnd = [IntPtr]::Zero
$rect = New-Object Win32+RECT
while ((Get-Date) -lt $deadline) {
    $proc = Get-Process resorust -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne 0 } |
        Select-Object -First 1
    if ($proc) {
        $hwnd = $proc.MainWindowHandle
    } else {
        $hwnd = [Win32]::FindByTitle($Title)
    }
    if ($hwnd -ne [IntPtr]::Zero) {
        [Win32]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
        if (($rect.Right - $rect.Left) -gt 400) { break }
    }
    Start-Sleep -Milliseconds 250
}

if ($hwnd -eq [IntPtr]::Zero -or (($rect.Right - $rect.Left) -le 400)) {
    throw "Janela '$Title' não encontrada ou ainda não desenhada."
}

[Win32]::ShowWindow($hwnd, 9) | Out-Null
[Win32]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 1500

[Win32]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top

$bmp = New-Object System.Drawing.Bitmap $width, $height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
[Win32]::PrintWindow($hwnd, $hdc, 2) | Out-Null
$g.ReleaseHdc($hdc)
$g.Dispose()
$bmp.Save($OutFile, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

Write-Output "Janela: ${width}x${height} em ($($rect.Left),$($rect.Top))"
Write-Output "Screenshot: $OutFile"
