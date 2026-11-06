<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { t } from '@/i18n'
import { formatDateTime } from '@/i18n/format'
import { deleteAccount, exportAccount, profile, type Profile } from '@/api/endpoints'
import { deleteAccountWithWallet, deleteChallenge } from '@/api/wallet-endpoints'
import { connectWallet, signWithWallet, WalletFailure } from '@/platform/wallet'
import { encodeBase58 } from '@/signing/base58'
import { errorMessageKey, isSessionEnded } from '@/api/errors'
import { clearSession, currentToken, endSession } from '@/state/session'
import Notice from '@/components/Notice.vue'

const router = useRouter()
const account = ref<Profile | null>(null)
const error = ref('')
const success = ref('')
const exporting = ref(false)

const password = ref('')
const confirmed = ref(false)
const deleting = ref(false)

async function run(action: (token: string) => Promise<void>): Promise<void> {
  const token = currentToken()
  if (!token) return
  error.value = ''
  try {
    await action(token)
  } catch (failure) {
    if (isSessionEnded(failure)) {
      await endSession(router)
      return
    }
    error.value = t(errorMessageKey(failure))
  }
}

async function load(): Promise<void> {
  await run(async (token) => {
    account.value = await profile(token)
  })
}

async function downloadExport(): Promise<void> {
  exporting.value = true
  success.value = ''
  await run(async (token) => {
    const file = await exportAccount(token)
    const url = URL.createObjectURL(file)
    const link = document.createElement('a')
    link.href = url
    link.download = 'silicium-account-data.json'
    document.body.append(link)
    link.click()
    link.remove()
    // Revoke after the browser had time to start the download.
    setTimeout(() => URL.revokeObjectURL(url), 1000)
    success.value = t('account.exportDone')
  })
  exporting.value = false
}

// Wallet accounts have no password: the deletion is confirmed by a signature of a fresh
// delete challenge, made by the wallet that owns the account.
async function removeWalletAccount(): Promise<void> {
  const token = currentToken()
  if (!token || !account.value) return
  deleting.value = true
  success.value = ''
  error.value = ''
  try {
    if (encodeBase58(await connectWallet()) !== account.value.address) {
      error.value = t('wallet.mismatch')
      return
    }
    const challenge = await deleteChallenge(token)
    const signature = await signWithWallet(new TextEncoder().encode(challenge.message), 'utf8')
    await deleteAccountWithWallet(token, challenge.nonce, signature)
    clearSession()
    await router.push({ name: 'login', query: { deleted: '1' } })
  } catch (failure) {
    if (failure instanceof WalletFailure) error.value = t(`wallet.${failure.code}`)
    else if (isSessionEnded(failure)) await endSession(router)
    else error.value = t(errorMessageKey(failure))
  } finally {
    deleting.value = false
  }
}

async function removeAccount(): Promise<void> {
  deleting.value = true
  success.value = ''
  await run(async (token) => {
    await deleteAccount(token, password.value)
    clearSession()
    await router.push({ name: 'login', query: { deleted: '1' } })
  })
  password.value = ''
  deleting.value = false
}

onMounted(load)
</script>

<template>
  <section class="stack" aria-labelledby="account-title">
    <h1 id="account-title" tabindex="-1">{{ t('account.title') }}</h1>

    <Notice kind="error" :message="error" />
    <Notice kind="success" :message="success" />

    <dl v-if="account" class="card dl">
      <template v-if="account.email">
        <dt>{{ t('account.email') }}</dt>
        <dd>{{ account.email }}</dd>
      </template>
      <template v-if="account.address">
        <dt>{{ t('wallet.address') }}</dt>
        <dd><code>{{ account.address }}</code></dd>
      </template>
      <dt>{{ t('account.created') }}</dt>
      <dd><time :datetime="account.created_at">{{ formatDateTime(account.created_at) }}</time></dd>
    </dl>

    <section class="card stack" aria-labelledby="export-title">
      <h2 id="export-title">{{ t('account.exportTitle') }}</h2>
      <p>{{ t('account.exportText') }}</p>
      <div class="actions">
        <button type="button" class="btn" :disabled="exporting" @click="downloadExport">
          {{ t('account.exportButton') }}
        </button>
      </div>
    </section>

    <section class="card stack" aria-labelledby="delete-title">
      <h2 id="delete-title">{{ t('account.deleteTitle') }}</h2>
      <p>{{ t('account.deleteText') }}</p>
      <form v-if="account?.email" class="stack" novalidate @submit.prevent="removeAccount">
        <div class="field">
          <label for="delete-password">{{ t('account.deletePassword') }}</label>
          <input id="delete-password" v-model="password" class="input" type="password" autocomplete="current-password" />
        </div>
        <div class="checkbox-row">
          <input id="delete-confirm" v-model="confirmed" type="checkbox" />
          <label for="delete-confirm">{{ t('account.deleteConfirm') }}</label>
        </div>
        <div class="actions">
          <button type="submit" class="btn btn-danger" :disabled="deleting || !confirmed || password.length === 0">
            {{ t('account.deleteButton') }}
          </button>
        </div>
      </form>
      <div v-else-if="account?.address" class="actions">
        <p class="muted">{{ t('account.deleteWalletText') }}</p>
        <button type="button" class="btn btn-danger" :disabled="deleting" @click="removeWalletAccount">
          {{ t('account.deleteButton') }}
        </button>
      </div>
    </section>
  </section>
</template>
