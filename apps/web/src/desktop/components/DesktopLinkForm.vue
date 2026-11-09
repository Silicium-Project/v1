<script setup lang="ts">
import { ref } from 'vue'
import { t } from '@/i18n'
import { errorMessageKey } from '@/api/errors'
import { DesktopFailure } from '@/platform/desktop'
import { linkThisMachine } from '@/desktop/link'
import FormField from '@/components/FormField.vue'
import Notice from '@/components/Notice.vue'

// Manual linking, when the code was not opened from the website. The same proof is made as
// for a one-click link (desktop/link.ts).
const emit = defineEmits<{ linked: [] }>()

const code = ref('')
const label = ref('')
const busy = ref(false)
const error = ref('')
const success = ref('')
const codeError = ref('')

function desktopMessage(failure: unknown): string {
  if (failure instanceof DesktopFailure) return t(`desktop.${failure.code}`)
  return t(errorMessageKey(failure))
}

async function submit(): Promise<void> {
  error.value = ''
  success.value = ''
  codeError.value = code.value.trim().length === 0 ? t('desktop.bad_code') : ''
  if (codeError.value) {
    document.getElementById('link-code')?.focus()
    return
  }
  busy.value = true
  try {
    await linkThisMachine(code.value, label.value)
    success.value = t('devices.linkHereDone')
    code.value = ''
    label.value = ''
    emit('linked')
  } catch (failure) {
    error.value = desktopMessage(failure)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="card stack" aria-labelledby="link-here-title">
    <h2 id="link-here-title">{{ t('devices.linkHereTitle') }}</h2>
    <p>{{ t('devices.linkHereText') }}</p>
    <Notice kind="error" :message="error" />
    <Notice kind="success" :message="success" />
    <form class="stack" novalidate @submit.prevent="submit">
      <FormField
        id="link-code"
        v-model="code"
        :label="t('devices.linkHereCode')"
        autocomplete="off"
        :error="codeError"
        required
      />
      <FormField id="link-label" v-model="label" :label="t('devices.linkHereLabel')" autocomplete="off" />
      <div class="actions">
        <button type="submit" class="btn btn-primary" :disabled="busy">
          {{ busy ? t('common.loading') : t('devices.linkHereSubmit') }}
        </button>
      </div>
    </form>
  </section>
</template>
