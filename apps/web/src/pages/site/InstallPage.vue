<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { messages } from '@/i18n'

const install = computed(() => messages().site.install)

// Paquets produits par apps/web/scripts/copy-node-bundle.mjs (public/downloads/, manifest.json).
interface Bundle {
  platform: 'linux-x86_64' | 'windows-x86_64'
  file: string
  sha256: string
  bytes: number
}
const bundles = ref<Bundle[]>([])
const linux = computed(() => bundles.value.find((bundle) => bundle.platform === 'linux-x86_64') ?? null)
const windows = computed(() => bundles.value.find((bundle) => bundle.platform === 'windows-x86_64') ?? null)

// Installeur de l'application de bureau, produit par apps/web/scripts/copy-app-installer.mjs (app.json).
interface AppBundle {
  version: string
  file: string
  sha256: string
  bytes: number
  certificate: string
  certificate_sha256: string
}
const app = ref<AppBundle | null>(null)

// Application de bureau Linux (AppImage, deb), produite par la même compilation Tauri (app-linux.json).
interface LinuxAppPackage {
  kind: string
  file: string
  sha256: string
  bytes: number
}
const linuxApps = ref<LinuxAppPackage[]>([])

onMounted(async () => {
  try {
    const response = await fetch('/downloads/manifest.json')
    if (response.ok) bundles.value = ((await response.json()) as { bundles: Bundle[] }).bundles
  } catch {
    // Without the manifest the page still explains how to build the node.
  }
  try {
    const response = await fetch('/downloads/app.json')
    if (response.ok) app.value = (await response.json()) as AppBundle
  } catch {
    // Without the application file the page still offers the node.
  }
  try {
    const response = await fetch('/downloads/app-linux.json')
    if (response.ok) linuxApps.value = ((await response.json()) as { packages: LinuxAppPackage[] }).packages
  } catch {
    // Without the Linux application the page still offers the node.
  }
})

// Commandes reprises de docs/user/run-a-node.md : elles doivent rester identiques à la doc.
const BUILD = 'cargo build --release -p sil-node'
const RUN = './target/release/silicium-node --config node.toml --roles worker,verifier'
const LINUX_VERIFY = 'sha256sum -c silicium-node-linux-amd64.tar.gz.sha256'
const LINUX_UNPACK = 'tar -xzf silicium-node-linux-amd64.tar.gz\ncd silicium-node\n./silicium-node --config node.toml --roles worker,verifier'
const WINDOWS_UNPACK = 'Expand-Archive .\\silicium-node-windows-x86_64.zip -DestinationPath .\ncd silicium-node\n.\\silicium-node.exe --config node.toml --roles worker,verifier'
// Commandes d'import du certificat (public) : racines de confiance, puis éditeurs approuvés.
const IMPORT_ROOT =
  'Import-Certificate -FilePath .\\silicium-school-certificate.cer -CertStoreLocation Cert:\\CurrentUser\\Root'
const IMPORT_PUBLISHER =
  'Import-Certificate -FilePath .\\silicium-school-certificate.cer -CertStoreLocation Cert:\\CurrentUser\\TrustedPublisher'
// Dépôts signés (apt-get / dnf) : clé de release publique, dépôt Debian ou Fedora.
const APT_SETUP = [
  'curl -fsSL https://siliciumlabs.org/download/silicium-release.asc | sudo tee /usr/share/keyrings/silicium.asc >/dev/null',
  'echo "deb [signed-by=/usr/share/keyrings/silicium.asc] https://siliciumlabs.org/download/apt stable main" | sudo tee /etc/apt/sources.list.d/silicium.list',
  'sudo apt update && sudo apt install silicium',
].join('\n')
const DNF_SETUP = [
  'sudo curl -fsSL -o /etc/yum.repos.d/silicium.repo https://siliciumlabs.org/download/silicium.repo',
  'sudo dnf install silicium',
].join('\n')
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container site-hero--plain">
      <h1>{{ install.title }}</h1>
      <p class="site-lede">{{ install.lede }}</p>
    </section>

    <section class="site-section site-container" aria-labelledby="install-status">
      <div class="site-notice" role="note">
        <h2 id="install-status">{{ install.statusTitle }}</h2>
        <p>{{ install.statusText }}</p>
      </div>
    </section>

    <section class="site-section site-container" aria-labelledby="install-app">
      <h2 id="install-app">{{ install.appTitle }}</h2>
      <p>{{ install.appText }}</p>
      <div class="actions">
        <a
          v-if="app"
          class="btn btn-primary"
          :href="`/downloads/${app.file}`"
          :download="app.file"
        >{{ install.appDownload }}</a>
      </div>
      <p v-if="app" class="muted">{{ install.downloadHash }} <code>{{ app.sha256 }}</code></p>
      <p v-else class="muted">{{ install.downloadMissing }}</p>

      <div class="site-notice" role="note">
        <h3>{{ install.appWarnTitle }}</h3>
        <p>{{ install.appWarnText }}</p>
      </div>

      <h3>{{ install.certTitle }}</h3>
      <p>{{ install.certText }}</p>
      <div class="actions">
        <a
          v-if="app"
          class="btn"
          :href="`/downloads/${app.certificate}`"
          :download="app.certificate"
        >{{ install.certDownload }}</a>
      </div>
      <p>{{ install.certRoot }}</p>
      <pre class="code-display"><code>{{ IMPORT_ROOT }}</code></pre>
      <p>{{ install.certPublisher }}</p>
      <pre class="code-display"><code>{{ IMPORT_PUBLISHER }}</code></pre>
    </section>

    <section class="site-section site-container" aria-labelledby="install-download">
      <h2 id="install-download">{{ install.downloadTitle }}</h2>
      <p>{{ install.downloadText }}</p>

      <div class="site-columns">
        <div>
          <h3>{{ install.downloadLinux }}</h3>
          <template v-for="pkg in linuxApps" :key="pkg.file">
            <div class="actions">
              <a class="btn btn-primary" :href="`/downloads/${pkg.file}`" :download="pkg.file"
                >{{ install.appDownload }} ({{ pkg.kind }})</a>
            </div>
            <p class="muted">{{ install.downloadHash }} <code>{{ pkg.sha256 }}</code></p>
          </template>
          <div class="actions">
            <a
              v-if="linux"
              class="btn"
              :href="`/downloads/${linux.file}`"
              :download="linux.file"
            >{{ install.downloadButton }}</a>
          </div>
          <p v-if="linux" class="muted">{{ install.downloadHash }} <code>{{ linux.sha256 }}</code></p>
          <p v-else class="muted">{{ install.downloadMissing }}</p>
          <p>{{ install.downloadVerify }}</p>
          <pre class="code-display"><code>{{ LINUX_VERIFY }}</code></pre>
          <p>{{ install.downloadRun }}</p>
          <pre class="code-display"><code>{{ LINUX_UNPACK }}</code></pre>
        </div>

        <div>
          <h3>{{ install.downloadWindows }}</h3>
          <div class="actions">
            <a
              v-if="windows"
              class="btn btn-primary"
              :href="`/downloads/${windows.file}`"
              :download="windows.file"
            >{{ install.downloadButton }}</a>
          </div>
          <p v-if="windows" class="muted">{{ install.downloadHash }} <code>{{ windows.sha256 }}</code></p>
          <p v-else class="muted">{{ install.downloadMissing }}</p>
          <p>{{ install.downloadVerifyWindows }}</p>
          <pre class="code-display"><code>Get-FileHash .\silicium-node-windows-x86_64.zip -Algorithm SHA256</code></pre>
          <p>{{ install.downloadRun }}</p>
          <pre class="code-display"><code>{{ WINDOWS_UNPACK }}</code></pre>
        </div>
      </div>
    </section>

    <section class="site-section site-container" aria-labelledby="install-repo">
      <h2 id="install-repo">{{ install.repoTitle }}</h2>
      <p>{{ install.repoText }}</p>
      <div class="site-columns">
        <div>
          <h3>{{ install.repoAptTitle }}</h3>
          <pre class="code-display"><code>{{ APT_SETUP }}</code></pre>
        </div>
        <div>
          <h3>{{ install.repoDnfTitle }}</h3>
          <pre class="code-display"><code>{{ DNF_SETUP }}</code></pre>
        </div>
      </div>
    </section>

    <section class="site-section site-container" aria-labelledby="install-build">
      <h2 id="install-build">{{ install.buildTitle }}</h2>
      <p>{{ install.buildHint }}</p>
      <pre class="code-display"><code>{{ BUILD }}
{{ RUN }}</code></pre>
    </section>

    <section class="site-section site-container site-columns">
      <div>
        <h2>{{ install.rolesTitle }}</h2>
        <ul class="site-list">
          <li v-for="item in install.rolesItems" :key="item">{{ item }}</li>
        </ul>
      </div>
      <div>
        <h2>{{ install.keyTitle }}</h2>
        <p>{{ install.keyText }}</p>
      </div>
    </section>
  </div>
</template>
