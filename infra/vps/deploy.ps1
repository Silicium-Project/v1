# Copie les binaires compilés et les unités systemd sur le VPS, puis relance les services.
# Équivalent Windows de deploy.sh : utilise le OpenSSH de Windows et la clé du poste.
# Le compte de déploiement n'a pas de droits root généraux : les fichiers passent sur stdin au helper
# /usr/local/sbin/silicium-deploy, comme dans deploy.sh (voir infra/vps/silicium-deploy-helper.sh).
#
# Usage, depuis la racine du dépôt :
#   powershell -ExecutionPolicy Bypass -File infra\vps\deploy.ps1 -Target silicium-deploy-prod@<hote-vps> -EnvName prod -Key $HOME\.ssh\silicium_ci_prod
#   ... -DryRun : prépare les fichiers et vérifie la connexion, sans rien copier ni redémarrer.
# La sauvegarde (infra/vps/backup.sh) est exigée avant tout déploiement prod ; elle se lance via Git Bash.
param(
  [Parameter(Mandatory = $true)][string]$Target,
  [ValidateSet('prod', 'dev')][string]$EnvName = 'prod',
  [string]$Key = "$env:USERPROFILE\.ssh\id_ed25519",
  [switch]$DryRun
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$dist = Join-Path $root "dist\linux-amd64"
$ssh = "$env:SystemRoot\System32\OpenSSH\ssh.exe"
$scp = "$env:SystemRoot\System32\OpenSSH\scp.exe"
if (-not (Test-Path $Key)) { throw "clé SSH absente : $Key (-Key)" }

$suffix = if ($EnvName -eq 'dev') { '-dev' } else { '' }
$helper = "sudo /usr/local/sbin/silicium-deploy $EnvName"
$binaries = @("silicium-api", "silicium-node", "sil-sandbox-launch", "sil-raytracer-block", "libonnxruntime.so")
$units = @("silicium-api", "silicium-node")
$opts = "-o BatchMode=yes -o ConnectTimeout=15 -o StrictHostKeyChecking=yes -i `"$Key`""
$utf8 = New-Object System.Text.UTF8Encoding($false)

# Envoie un fichier par scp (octets bruts, sans passer par un flux texte), puis le fait lire sur stdin
# du helper par le shell distant, depuis le dossier de transit du compte. Le fichier de transit est effacé.
$stageDir = "deploy-stage"
function Send-File([string]$path, [string]$remoteCommand) {
  $name = Split-Path $path -Leaf
  & $scp -q -o BatchMode=yes -o ConnectTimeout=15 -o StrictHostKeyChecking=yes -i $Key $path "${Target}:$stageDir/$name"
  if ($LASTEXITCODE -ne 0) { throw "scp en échec : $name (exit $LASTEXITCODE)" }
  $cmd = $remoteCommand + ' < ' + $stageDir + '/' + $name + '; c=$?; rm -f ' + $stageDir + '/' + $name + '; exit $c'
  Run-Remote $cmd
}

function Run-Remote([string]$remoteCommand) {
  & $ssh -o BatchMode=yes -o ConnectTimeout=15 -o StrictHostKeyChecking=yes -i $Key $Target $remoteCommand
  if ($LASTEXITCODE -ne 0) { throw "commande distante en échec : $remoteCommand (exit $LASTEXITCODE)" }
}

$stage = Join-Path $env:TEMP ("silicium-deploy-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory $stage | Out-Null
try {
  foreach ($file in $binaries) {
    $source = Join-Path $dist $file
    if (-not (Test-Path $source)) { throw "absent : dist\linux-amd64\$file" }
    Copy-Item $source $stage
  }
  # Unités : chemins réécrits pour l'environnement, comme deploy.sh.
  foreach ($unit in $units) {
    $text = Get-Content -Raw (Join-Path $root "infra\systemd\$unit.service")
    $text = $text.Replace('/opt/silicium/', "/opt/silicium$suffix/").Replace('/etc/silicium/', "/etc/silicium$suffix/").Replace('/var/lib/silicium/', "/var/lib/silicium$suffix/")
    [System.IO.File]::WriteAllText((Join-Path $stage "$unit$suffix.service"), ($text -replace "`r", ""), $utf8)
  }

  # Vérifie la connexion avant toute copie.
  Run-Remote "echo CONNEXION_OK"
  if ($DryRun) { "dry run : fichiers prêts dans $stage, rien copié."; return }

  # Règle : aucun déploiement prod sans sauvegarde réussie des données (journal, node.seed, base API).
  if ($EnvName -eq 'prod') {
    $bash = "$env:ProgramFiles\Git\bin\bash.exe"
    if (-not (Test-Path $bash)) { throw "bash (Git for Windows) introuvable : $bash" }
    $env:SIL_SSH_KEY = $Key
    Push-Location $root
    try { & $bash "infra/vps/backup.sh" $Target } finally { Pop-Location }
    if ($LASTEXITCODE -ne 0) { throw "sauvegarde échouée : déploiement annulé" }
  }

  # Binaires puis unités : écriture par le helper (fichier écrit à côté puis renommé).
  Run-Remote "mkdir -p $stageDir && chmod 700 $stageDir"
  foreach ($bin in $binaries) {
    Send-File (Join-Path $stage $bin) "$helper bin $bin"
  }
  foreach ($unit in $units) {
    Send-File (Join-Path $stage "$unit$suffix.service") "$helper unit $unit"
  }
  Run-Remote "$helper restart"
  "DEPLOY_OK $suffix"
} finally {
  Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
}
