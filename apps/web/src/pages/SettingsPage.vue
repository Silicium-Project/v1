<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import { LOCALES, locale, setLocale, t, type Locale } from '@/i18n'
import { logout } from '@/api/endpoints'
import { WORKLOADS } from '@/workloads/generated'
import { loadPreferences, savePreferences, type Preferences } from '@/state/preferences'
import { clearDraft } from '@/state/submit-draft'
import { clearSession, currentToken } from '@/state/session'
import Notice from '@/components/Notice.vue'

// Paramètres : préférences de cet appareil (langue, type de calcul par défaut, brouillon) et
// session. Rien ici n'est envoyé à l'API, qui ne stocke pas de préférences.
const router = useRouter()
const workloadIds = WORKLOADS.map((item) => item.id)
const preferences = reactive<Preferences>(loadPreferences(workloadIds))
const success = ref('')
const LANGUAGE_NAMES: Record<Locale, string> = { fr: 'Français', en: 'English' }

watch(
  () => preferences.defaultWorkload,
  () => {
    savePreferences({ defaultWorkload: preferences.defaultWorkload })
    success.value = t('settings.saved')
  },
)

function changeLanguage(event: Event): void {
  setLocale((event.target as HTMLSelectElement).value as Locale)
  success.value = t('settings.saved')
}

function emptyDraft(): void {
  clearDraft()
  success.value = t('settings.draftCleared')
}

async function signOut(): Promise<void> {
  const token = currentToken()
  clearSession()
  try {
    if (token) await logout(token)
  } catch {
    // The token expires on its own; the local session is already gone.
  }
  await router.push({ name: 'login' })
}
</script>

<template>
  <section class="stack" aria-labelledby="settings-title">
    <h1 id="settings-title" tabindex="-1">{{ t('settings.title') }}</h1>

    <Notice kind="success" :message="success" />

    <section class="card stack" aria-labelledby="language-title">
      <h2 id="language-title">{{ t('settings.language') }}</h2>
      <div class="field">
        <label for="settings-language">{{ t('settings.languageLabel') }}</label>
        <select id="settings-language" class="input" :value="locale" @change="changeLanguage">
          <option v-for="code in LOCALES" :key="code" :value="code">{{ LANGUAGE_NAMES[code] }}</option>
        </select>
        <p class="field__hint">{{ t('settings.deviceOnly') }}</p>
      </div>
    </section>

    <section class="card stack" aria-labelledby="submit-title">
      <h2 id="submit-title">{{ t('settings.submitTitle') }}</h2>
      <div class="field">
        <label for="settings-workload">{{ t('settings.defaultWorkload') }}</label>
        <select id="settings-workload" v-model="preferences.defaultWorkload" class="input">
          <option :value="null">{{ t('settings.firstWorkload') }}</option>
          <option v-for="item in WORKLOADS" :key="item.id" :value="item.id">{{ t(item.labelKey) }}</option>
        </select>
        <p class="field__hint">{{ t('settings.deviceOnly') }}</p>
      </div>
      <div class="stack">
        <p>{{ t('settings.draftText') }}</p>
        <div class="actions">
          <button type="button" class="btn" @click="emptyDraft">{{ t('settings.draftClear') }}</button>
          <RouterLink class="btn btn-quiet" to="/soumettre">{{ t('settings.goSubmit') }}</RouterLink>
        </div>
      </div>
    </section>

    <section class="card stack" aria-labelledby="session-title">
      <h2 id="session-title">{{ t('settings.sessionTitle') }}</h2>
      <p>{{ t('settings.sessionText') }}</p>
      <div class="actions">
        <button type="button" class="btn btn-danger" @click="signOut">
          {{ t('common.logout') }}
        </button>
      </div>
    </section>

    <section class="card stack" aria-labelledby="privacy-title">
      <h2 id="privacy-title">{{ t('settings.privacyTitle') }}</h2>
      <p>{{ t('settings.privacyText') }}</p>
      <div class="actions">
        <RouterLink class="btn btn-quiet" to="/compte">{{ t('settings.openAccount') }}</RouterLink>
        <RouterLink class="btn btn-quiet" to="/confidentialite">{{ t('settings.openPrivacy') }}</RouterLink>
      </div>
    </section>
  </section>
</template>
