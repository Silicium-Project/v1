// Publie l'installeur signé de l'application de bureau et le certificat de signature (public
// uniquement) dans public/downloads/, avec app.json (nom, somme SHA-256, taille, certificat).
// Source de l'installeur : apps/desktop/src-tauri/target/release/bundle/nsis/ (après npm run build).
// Usage : node scripts/copy-app-installer.mjs <certificat.cer>
import { createHash } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const target = join(here, '..', 'public', 'downloads')
const VERSION = '0.1.5'
const SETUP = `Silicium_${VERSION}_x64-setup.exe`
const setup = resolve(here, '..', '..', 'desktop', 'src-tauri', 'target', 'release', 'bundle', 'nsis', SETUP)
const CERTIFICATE = 'silicium-school-certificate.cer'
const certificate = process.argv[2]

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex')
}

if (!existsSync(setup)) {
  console.error(`installeur absent : ${setup}\nLe construire d'abord : npm run build (dans apps/desktop).`)
  process.exit(1)
}
if (!certificate || !existsSync(certificate)) {
  console.error('usage : node scripts/copy-app-installer.mjs <certificat.cer>')
  process.exit(1)
}
const certBytes = readFileSync(certificate)
// Un certificat public au format DER commence par 0x30 et ne contient jamais de clé privée.
if (certBytes[0] !== 0x30 || certBytes.length > 8192 || certBytes.includes(Buffer.from('PRIVATE KEY'))) {
  console.error('refus : le fichier n’est pas un certificat public DER (pas de clé privée ni de .pfx)')
  process.exit(1)
}

mkdirSync(target, { recursive: true })
copyFileSync(setup, join(target, SETUP))
copyFileSync(certificate, join(target, CERTIFICATE))
const app = {
  version: VERSION,
  file: SETUP,
  sha256: sha256(setup),
  bytes: statSync(setup).size,
  certificate: CERTIFICATE,
  certificate_sha256: sha256(certificate),
}
writeFileSync(join(target, 'app.json'), `${JSON.stringify(app, null, 2)}\n`)
console.log(`installeur prêt : public/downloads/${SETUP} (${app.bytes} octets, sha256 ${app.sha256})`)
console.log(`certificat prêt : public/downloads/${CERTIFICATE} (sha256 ${app.certificate_sha256})`)
