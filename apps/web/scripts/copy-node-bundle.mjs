// Prépare les paquets « nœud » téléchargeables depuis le site : silicium-node, le moteur de rendu
// (et, sous Linux, son lanceur de sandbox), une configuration d'exemple, un README et les sommes de
// contrôle. Sources : dist/linux-amd64 et dist/windows-amd64 (binaires compilés à la racine du
// dépôt). Sorties : public/downloads/ (archive .tar.gz pour Linux, .zip pour Windows, manifest.json).
// Usage : node scripts/copy-node-bundle.mjs
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { gzipSync } from 'node:zlib'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..', '..', '..')
const staging = join(root, 'dist', 'bundle-staging')
const target = join(here, '..', 'public', 'downloads')
const isWindows = process.platform === 'win32'
// tar.exe du système : celui de Git confond « C: » avec une machine distante.
const tar = isWindows ? join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'tar.exe') : 'tar'

/** Chiffre octal sur `width` caractères, terminé par NUL (format ustar). */
function octal(value, width) {
  return `${value.toString(8).padStart(width - 1, '0')}\0`
}

/**
 * Archive .tar.gz écrite ici, sans le tar du système : tar.exe de Windows ne garde pas le bit
 * exécutable (les binaires Linux sortaient en 644 et « Permission denied »). Chaque fichier
 * reçoit son mode : 755 pour les binaires, 644 pour la configuration et le README.
 */
function tarGz(files) {
  const blocks = []
  for (const { name, data, mode } of files) {
    const header = Buffer.alloc(512)
    header.write(name, 0, 100, 'utf8')
    header.write(octal(mode, 8), 100, 8)
    header.write(octal(0, 8), 108, 8)
    header.write(octal(0, 8), 116, 8)
    header.write(octal(data.length, 12), 124, 12)
    header.write(octal(Math.floor(Date.now() / 1000), 12), 136, 12)
    header.fill(0x20, 148, 156)
    header.write('0', 156, 1)
    header.write('ustar\0' + '00', 257, 8)
    header.write('root', 265, 32)
    header.write('root', 297, 32)
    let sum = 0
    for (const byte of header) sum += byte
    header.write(`${sum.toString(8).padStart(6, '0')}\0 `, 148, 8)
    blocks.push(header, data, Buffer.alloc((512 - (data.length % 512)) % 512))
  }
  blocks.push(Buffer.alloc(1024))
  return gzipSync(Buffer.concat(blocks))
}

const BUNDLES = [
  {
    platform: 'linux-x86_64',
    source: join(root, 'dist', 'linux-amd64'),
    // libonnxruntime.so : runtime de l'embedding, cherché à côté de silicium-node (cli.rs).
    binaries: ['silicium-node', 'sil-raytracer-block', 'sil-sandbox-launch', 'libonnxruntime.so'],
    archive: 'silicium-node-linux-amd64.tar.gz',
    engine: './sil-raytracer-block',
    readme: `Nœud Silicium, paquet Linux x86-64.

Démarrer (depuis ce dossier) :
  ./silicium-node --config node.toml --roles worker,verifier

Sans configuration de chaîne, le nœud rejoint le réseau de test et ne réclame aucun calcul.
La clé du nœud est créée dans data/node.seed : sauvegardez-la, elle porte votre identité.
`,
  },
  {
    platform: 'windows-x86_64',
    source: join(root, 'dist', 'windows-amd64'),
    binaries: ['silicium-node.exe', 'sil-raytracer-block.exe', 'onnxruntime.dll'],
    archive: 'silicium-node-windows-x86_64.zip',
    engine: './sil-raytracer-block.exe',
    readme: `Nœud Silicium, paquet Windows x86-64.

Démarrer (depuis ce dossier, dans PowerShell) :
  .\\silicium-node.exe --config node.toml --roles worker,verifier

Sans configuration de chaîne, le nœud rejoint le réseau de test et ne réclame aucun calcul.
La clé du nœud est créée dans data\\node.seed : sauvegardez-la, elle porte votre identité.
`,
  },
]

// Configuration d'exemple : aucun secret, le nœud de test du réseau comme point d'amorçage.
const configFor = (engine) => `# Configuration d'un nœud Silicium (réseau de test). Aucun secret dans ce fichier.
data_dir = "data"

[network]
listen = ["/ip4/0.0.0.0/tcp/4001", "/ip4/0.0.0.0/udp/4001/quic-v1"]
bootstrap = ["/ip4/213.32.68.67/tcp/4001/p2p/12D3KooWHXaQz2ou13jcnXq33yZbHKSmabBJJYxuLhx4UhG2k8ob"]
enable_mdns = false

[control]
bind = "127.0.0.1"
port = 8750

[log]
level = "info"
json = false

# Rendu raytracing : le moteur est à côté de silicium-node.
[render]
engine = "${engine}"
wall_ms = 600000
isolation = "best-effort"
`

rmSync(staging, { recursive: true, force: true })
mkdirSync(target, { recursive: true })
const manifest = []

for (const bundle of BUNDLES) {
  for (const name of bundle.binaries) {
    if (!existsSync(join(bundle.source, name))) {
      console.error(`binaire absent : ${join(bundle.source, name)}\nLe construire d'abord (voir docs/technical/deployment.md).`)
      process.exit(1)
    }
  }
  const dir = join(staging, bundle.platform, 'silicium-node')
  mkdirSync(dir, { recursive: true })
  for (const name of bundle.binaries) copyFileSync(join(bundle.source, name), join(dir, name))
  writeFileSync(join(dir, 'node.toml'), configFor(bundle.engine))
  writeFileSync(join(dir, 'README.txt'), bundle.readme)

  if (bundle.archive.endsWith('.zip')) {
    const parent = join(staging, bundle.platform)
    execFileSync(tar, ['-a', '-c', '-f', join(target, bundle.archive), '-C', parent, 'silicium-node'], { stdio: 'inherit' })
  } else {
    const files = [...bundle.binaries.map((name) => ({ name, mode: 0o755 })), { name: 'node.toml', mode: 0o644 }, { name: 'README.txt', mode: 0o644 }]
    writeFileSync(
      join(target, bundle.archive),
      tarGz(files.map(({ name, mode }) => ({ name: `silicium-node/${name}`, mode, data: readFileSync(join(dir, name)) }))),
    )
  }

  const bytes = readFileSync(join(target, bundle.archive))
  const sha256 = createHash('sha256').update(bytes).digest('hex')
  writeFileSync(join(target, `${bundle.archive}.sha256`), `${sha256}  ${bundle.archive}\n`)
  manifest.push({ platform: bundle.platform, file: bundle.archive, sha256, bytes: bytes.length })
  console.log(`paquet prêt : public/downloads/${bundle.archive} (${bytes.length} octets, sha256 ${sha256})`)
}

writeFileSync(join(target, 'manifest.json'), `${JSON.stringify({ bundles: manifest }, null, 2)}\n`)
rmSync(staging, { recursive: true, force: true })
