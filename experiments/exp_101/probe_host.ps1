# Read-only probe of the target host and console capabilities.
$src = @"
using System;
using System.Runtime.InteropServices;
public static class ConsoleProbe {
  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern bool GetConsoleMode(IntPtr h, out uint mode);
  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern IntPtr GetStdHandle(int n);
  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern bool GetConsoleScreenBufferInfo(IntPtr h, out CSBI info);
  [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
  public static extern IntPtr CreateFile(string name, uint access, uint share,
      IntPtr security, uint creation, uint flags, IntPtr template);
  [StructLayout(LayoutKind.Sequential)]
  public struct COORD { public short X; public short Y; }
  [StructLayout(LayoutKind.Sequential)]
  public struct SMALL_RECT { public short Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)]
  public struct CSBI {
    public COORD dwSize; public COORD dwCursorPosition; public ushort wAttributes;
    public SMALL_RECT srWindow; public COORD dwMaximumWindowSize;
  }
}
"@
Add-Type -TypeDefinition $src
function Mode($n) {
  $h = [ConsoleProbe]::GetStdHandle($n)
  [uint32]$m = 0
  $ok = [ConsoleProbe]::GetConsoleMode($h, [ref]$m)
  if ($ok) { '0x{0:X4}' -f $m } else { 'FAILED' }
}
"HOST_NAME            = $($host.Name)"
"PS_VERSION           = $($PSVersionTable.PSVersion)"
"WT_SESSION           = $env:WT_SESSION"
"TERM_PROGRAM         = $env:TERM_PROGRAM"
"CONEMU               = $env:ConEmuPID"
"ANSICON              = $env:ANSICON"
"SESSIONNAME          = $env:SESSIONNAME"
"ComSpec              = $env:ComSpec"
"OUTPUT_MODE(std)     = $(Mode -11)"
"INPUT_MODE(std)      = $(Mode -10)"
$conout = [ConsoleProbe]::CreateFile('CONOUT$', [uint32]3221225472, [uint32]3, [IntPtr]::Zero, [uint32]3, [uint32]0, [IntPtr]::Zero)
"CONOUT_HANDLE        = $conout"
if ($conout -ne [IntPtr]::Zero -and $conout -ne [IntPtr]::new(-1)) {
  [uint32]$cm = 0
  $ok = [ConsoleProbe]::GetConsoleMode($conout, [ref]$cm)
  "CONOUT_MODE         = $(if ($ok) { '0x{0:X4}' -f $cm } else { 'FAILED' })"
}
$hOut = $conout
if ($hOut -eq [IntPtr]::Zero -or $hOut -eq [IntPtr]::new(-1)) { $hOut = [ConsoleProbe]::GetStdHandle(-11) }
$csbi = New-Object ConsoleProbe+CSBI
if ([ConsoleProbe]::GetConsoleScreenBufferInfo($hOut, [ref]$csbi)) {
  "BUFFER_WxH          = $($csbi.dwSize.X)x$($csbi.dwSize.Y)"
  "WINDOW_WxH          = $($csbi.srWindow.Right - $csbi.srWindow.Left + 1)x$($csbi.srWindow.Bottom - $csbi.srWindow.Top + 1)"
}
"CODEPAGE_OUTPUT      = $([Console]::OutputEncoding.WebName)"
"CODEPAGE_INPUT       = $([Console]::InputEncoding.WebName)"
$mem = (Get-CimInstance Win32_PhysicalMemory | Measure-Object -Property Capacity -Sum).Sum
"RAM_BYTES            = $mem"
"RAM_GB               = $([math]::Round($mem/1GB,2))"
