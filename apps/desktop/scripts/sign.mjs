// Signe des fichiers Windows avec le certificat de code Silicium (magasin CurrentUser\My) et les
// horodate, pour que la signature reste valide après l'expiration du certificat.
// Usage : node scripts/sign.mjs <fichier>...   Empreinte : SIL_SIGN_THUMBPRINT (défaut : celle du certificat).
import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'

const thumbprint = process.env.SIL_SIGN_THUMBPRINT ?? 'A58274656DA5B0B0731139D8D0E46D26ECF1B4D4'
const signtool =
  process.env.SIGNTOOL ?? 'C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.22621.0\\x64\\signtool.exe'
const timestampUrl = 'http://timestamp.digicert.com'

const files = process.argv.slice(2)
if (files.length === 0) {
  console.error('usage : node scripts/sign.mjs <fichier>...')
  process.exit(1)
}
for (const file of files) {
  if (!existsSync(file)) {
    console.error(`absent : ${file}`)
    process.exit(1)
  }
  execFileSync(signtool, ['sign', '/sha1', thumbprint, '/fd', 'sha256', '/td', 'sha256', '/tr', timestampUrl, file], {
    stdio: 'inherit',
  })
  console.log(`signé : ${file}`)
}
