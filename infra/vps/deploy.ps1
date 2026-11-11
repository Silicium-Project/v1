# Copie les binaires compilés et les unités systemd sur le VPS, puis relance les services.
# Équivalent de deploy.sh pour Windows : utilise le OpenSSH de Windows et la clé du poste.
# Usage, depuis la racine du dépôt :
#   powershell -ExecutionPolicy Bypass -File infra\vps\deploy.ps1 -Target ubuntu@213.32.68.67
#   ... -DryRun : prépare les fichiers et vérifie la connexion, sans rien copier ni redémarrer.
param(
  [Parameter(Mandatory = $true)][string]$Target,
  [string]$Key = "$env:USERPROFILE\.ssh\id_ed25519",
  [switch]$DryRun
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$dist = Join-Path $root "dist\linux-amd64"
$ssh = "$env:SystemRoot\System32\OpenSSH\ssh.exe"
$scp = "$env:SystemRoot\System32\OpenSSH\scp.exe"
if (-not (Test-Path $Key)) { throw "clé SSH absente : $Key (-Key)" }

$files = @("silicium-api", "silicium-node", "sil-sandbox-launch", "sil-raytracer-block", "libonnxruntime.so")
$stage = Join-Path $env:TEMP ("silicium-deploy-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory $stage | Out-Null
try {
  foreach ($file in $files) {
    $source = Join-Path $dist $file
    if (-not (Test-Path $source)) { throw "absent : dist\linux-amd64\$file" }
    Copy-Item $source $stage
  }
  Copy-Item (Join-Path $root "infra\systemd\silicium-api.service") $stage
  Copy-Item (Join-Path $root "infra\systemd\silicium-node.service") $stage

  $opts = @("-o", "BatchMode=yes", "-o", "ConnectTimeout=15", "-o", "StrictHostKeyChecking=yes", "-i", $Key)
  # Vérifie la connexion avant toute copie.
  & $ssh @opts $Target "echo CONNEXION_OK"
  if ($LASTEXITCODE -ne 0) { throw "connexion SSH impossible (exit $LASTEXITCODE)" }
  if ($DryRun) { "dry run : fichiers prêts dans $stage, rien copié."; return }

  $names = @($files + @("silicium-api.service", "silicium-node.service")) | ForEach-Object { Join-Path $stage $_ }
  & $scp -q @opts $names "${Target}:/tmp/"
  if ($LASTEXITCODE -ne 0) { throw "scp en échec (exit $LASTEXITCODE)" }

  # Écriture atomique : un binaire en cours d'exécution ne peut pas être écrasé sur place.
  $remote = @'
set -e
for bin in silicium-api silicium-node sil-sandbox-launch sil-raytracer-block libonnxruntime.so; do
  sudo install -m 0755 "/tmp/$bin" "/opt/silicium/bin/$bin.new"
  sudo mv -f "/opt/silicium/bin/$bin.new" "/opt/silicium/bin/$bin"
  rm -f "/tmp/$bin"
done
for unit in silicium-api silicium-node; do
  sudo install -m 0644 "/tmp/$unit.service" "/etc/systemd/system/$unit.service"
  rm -f "/tmp/$unit.service"
done
sudo systemctl daemon-reload
sudo systemctl try-restart silicium-node.service silicium-api.service
echo DEPLOY_OK
'@
  $script = Join-Path $stage "remote.sh"
  [System.IO.File]::WriteAllText($script, ($remote -replace "`r", ""), (New-Object System.Text.UTF8Encoding($false)))
  & $scp -q @opts $script "${Target}:/tmp/silicium-deploy-remote.sh"
  if ($LASTEXITCODE -ne 0) { throw "scp du script distant en échec" }
  & $ssh @opts $Target "bash /tmp/silicium-deploy-remote.sh; rm -f /tmp/silicium-deploy-remote.sh"
  if ($LASTEXITCODE -ne 0) { throw "installation distante en échec (exit $LASTEXITCODE)" }
} finally {
  Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
}
