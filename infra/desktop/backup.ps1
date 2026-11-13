# Sauvegarde des données du nœud de l'application de bureau Silicium (Windows), avant toute installation
# d'une nouvelle version. Copie seulement : l'original n'est ni déplacé ni renommé ni supprimé.
#
# Usage (PowerShell) :
#   infra\desktop\backup.ps1 [-Dest "$env:USERPROFILE\silicium-backups"]
#
# Contenu : identité du nœud (node.seed), journal (ledger), règlements en attente (settlement),
# lien au compte (link.json), jeton de contrôle, configuration. Le rendu, le cache de contenus et les
# embeddings sont régénérables et exclus.
param(
  [string]$Dest = (Join-Path $env:USERPROFILE 'silicium-backups')
)
$ErrorActionPreference = 'Stop'

$src = Join-Path $env:APPDATA 'org.silicium.desktop\node'
if (-not (Test-Path (Join-Path $src 'node.seed'))) { throw "node.seed introuvable dans $src" }
if (-not (Test-Path (Join-Path $src 'ledger'))) { throw "ledger introuvable dans $src" }

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$out = Join-Path $Dest "desktop-$stamp"
New-Item -ItemType Directory -Path $out -Force | Out-Null

# robocopy : code 0 à 7 = succès (8 et plus = échec).
$excluded = @('render-work', 'content', 'embed')
& robocopy $src (Join-Path $out 'node') /E /R:1 /W:1 /NFL /NDL /NJH /NJS /XD @excluded | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy a échoué (code $LASTEXITCODE)" }

foreach ($must in @('node.seed', 'ledger')) {
  if (-not (Test-Path (Join-Path $out "node\$must"))) { throw "absent de la sauvegarde : $must" }
}

Get-ChildItem -Path (Join-Path $out 'node') -Recurse -File |
  ForEach-Object {
    $rel = $_.FullName.Substring($out.Length + 1)
    '{0}  {1}' -f (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLower(), $rel
  } | Set-Content -LiteralPath (Join-Path $out 'SHA256SUMS') -Encoding utf8

Write-Output "sauvegarde vérifiée : $out"
# robocopy renvoie 1 quand il a copié des fichiers : ce n'est pas une erreur.
exit 0
