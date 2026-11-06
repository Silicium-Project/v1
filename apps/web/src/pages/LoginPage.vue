<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { t } from '@/i18n'
import { login, register } from '@/api/endpoints'
import { walletChallenge, walletLogin } from '@/api/wallet-endpoints'
import { errorMessageKey } from '@/api/errors'
import { connectWallet, signWithWallet, WalletFailure } from '@/platform/wallet'
import { encodeBase58 } from '@/signing/base58'
import { saveSession } from '@/state/session'
import { safeNext } from '@/router/safe-next'
import FormField from '@/components/FormField.vue'
import Notice from '@/components/Notice.vue'

const MIN_PASSWORD = 12
const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

const route = useRoute()
const router = useRouter()

const mode = ref<'login' | 'register'>('login')
const email = ref('')
const password = ref('')
const consent = ref(false)
const showPassword = ref(false)
const busy = ref(false)
const titleRef = ref<HTMLElement | null>(null)

const errors = ref<{ email?: string; password?: string; consent?: string }>({})
const alertText = ref('')
const infoText = ref(
  route.query.expired !== undefined ? t('auth.sessionEnded') : route.query.deleted !== undefined ? t('auth.deleted') : '',
)
const successText = ref('')

const isRegister = computed(() => mode.value === 'register')
const heading = computed(() => (isRegister.value ? t('auth.registerTitle') : t('auth.loginTitle')))

function validate(): boolean {
  const next: typeof errors.value = {}
  if (!EMAIL_PATTERN.test(email.value.trim())) next.email = t('auth.needEmail')
  if (isRegister.value && password.value.length < MIN_PASSWORD) next.password = t('auth.needPassword')
  if (!isRegister.value && password.value.length === 0) next.password = t('auth.needPasswordLogin')
  if (isRegister.value && !consent.value) next.consent = t('auth.needConsent')
  errors.value = next
  return Object.keys(next).length === 0
}

async function focusFirstError(): Promise<void> {
  await nextTick()
  const target = errors.value.email ? 'email' : errors.value.password ? 'password' : 'consent'
  document.getElementById(target)?.focus()
}

async function submit(): Promise<void> {
  alertText.value = ''
  successText.value = ''
  infoText.value = ''
  if (!validate()) {
    void focusFirstError()
    return
  }
  busy.value = true
  const address = email.value.trim()
  try {
    if (isRegister.value) {
      await register(address, password.value)
      successText.value = t('auth.registered')
    }
    const answer = await login(address, password.value)
    saveSession({ token: answer.token, expiresAt: answer.expires_at, email: address })
    password.value = ''
    await router.push(safeNext(route.query.next))
  } catch (failure) {
    alertText.value = t(errorMessageKey(failure))
    successText.value = ''
  } finally {
    busy.value = false
  }
}

// Wallet sign-in: the wallet proves it owns the address by signing the challenge message.
// The first valid signature creates the account.
async function loginWithWallet(): Promise<void> {
  alertText.value = ''
  successText.value = ''
  infoText.value = ''
  busy.value = true
  try {
    const address = encodeBase58(await connectWallet())
    const challenge = await walletChallenge(address)
    const signature = await signWithWallet(new TextEncoder().encode(challenge.message), 'utf8')
    const answer = await walletLogin(address, challenge.nonce, signature)
    saveSession({ token: answer.token, expiresAt: answer.expires_at, email: '' })
    await router.push(safeNext(route.query.next))
  } catch (failure) {
    alertText.value = failure instanceof WalletFailure ? t(`wallet.${failure.code}`) : t(errorMessageKey(failure))
  } finally {
    busy.value = false
  }
}

async function toggleMode(): Promise<void> {
  mode.value = isRegister.value ? 'login' : 'register'
  errors.value = {}
  alertText.value = ''
  successText.value = ''
  await nextTick()
  titleRef.value?.focus()
}
</script>

<template>
  <section class="auth" aria-labelledby="auth-title">
    <h1 id="auth-title" ref="titleRef" tabindex="-1">{{ heading }}</h1>
    <Notice kind="info" :message="infoText" />
    <Notice kind="success" :message="successText" />
    <Notice kind="error" :message="alertText" />

    <form class="stack" novalidate @submit.prevent="submit">
      <FormField
        id="email"
        v-model="email"
        type="email"
        :label="t('auth.email')"
        autocomplete="email"
        :error="errors.email"
        required
      />
      <FormField
        id="password"
        v-model="password"
        :type="showPassword ? 'text' : 'password'"
        :label="t('auth.password')"
        :autocomplete="isRegister ? 'new-password' : 'current-password'"
        :hint="isRegister ? t('auth.passwordHint') : undefined"
        :minlength="isRegister ? MIN_PASSWORD : undefined"
        :error="errors.password"
        required
      />
      <div>
        <button type="button" class="btn btn-quiet" :aria-pressed="showPassword ? 'true' : 'false'" @click="showPassword = !showPassword">
          {{ showPassword ? t('auth.hidePassword') : t('auth.showPassword') }}
        </button>
      </div>

      <div v-if="isRegister" class="checkbox-row">
        <input
          id="consent"
          v-model="consent"
          type="checkbox"
          :aria-invalid="errors.consent ? 'true' : undefined"
          :aria-describedby="errors.consent ? 'consent-error' : undefined"
        />
        <div>
          <label for="consent">{{ t('auth.consent') }}</label>
          <p><RouterLink to="/confidentialite">{{ t('auth.readPrivacy') }}</RouterLink></p>
          <p v-if="errors.consent" id="consent-error" class="field__error">{{ errors.consent }}</p>
        </div>
      </div>

      <div class="actions">
        <button type="submit" class="btn btn-primary" :disabled="busy">
          {{ isRegister ? t('auth.register') : t('auth.login') }}
        </button>
        <p v-if="busy" role="status" class="muted">{{ t('common.loading') }}</p>
      </div>
    </form>

    <section class="card stack wallet-login" aria-labelledby="wallet-title">
      <h2 id="wallet-title">{{ t('wallet.connectTitle') }}</h2>
      <p>{{ t('wallet.connectText') }}</p>
      <div class="actions">
        <button type="button" class="btn btn-primary" :disabled="busy" @click="loginWithWallet">
          {{ t('wallet.connect') }}
        </button>
      </div>
    </section>

    <p class="actions">
      <button type="button" class="btn-link" @click="toggleMode">
        {{ isRegister ? t('auth.toLogin') : t('auth.toRegister') }}
      </button>
    </p>
  </section>
</template>
