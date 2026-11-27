param(
  [Parameter(Mandatory=$true)][string]$FilePath,
  [string]$Arguments = '',
  [int]$Seconds = 5
)
# Start a sleeping process, let it settle, then sample RSS and CPU over $Seconds.
$p = Start-Process -FilePath $FilePath -ArgumentList $Arguments -PassThru
try {
  Start-Sleep -Milliseconds 800
  $p.Refresh()
  $rss = $p.WorkingSet64
  $private = $p.PrivateMemorySize64
  $c0 = $p.CPU
  $t0 = [DateTime]::UtcNow
  Start-Sleep -Seconds $Seconds
  $p.Refresh()
  $c1 = $p.CPU
  $wall = ([DateTime]::UtcNow - $t0).TotalSeconds
  "process=$FilePath args='$Arguments'"
  "rss_MB=$([math]::Round($rss/1MB,2)) private_MB=$([math]::Round($private/1MB,2))"
  "cpu_delta_s=$([math]::Round($c1-$c0,4)) wall_s=$([math]::Round($wall,3)) cpu_pct_of_one_core=$([math]::Round(100*($c1-$c0)/$wall,3))"
} finally {
  if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
}
