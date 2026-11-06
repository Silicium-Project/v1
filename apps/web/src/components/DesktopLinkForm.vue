<script setup lang="ts">
import { ref } from 'vue'
import { t } from '@/i18n'
import { linkDevice } from '@/api/endpoints'
import { errorMessageKey } from '@/api/errors'
import { DesktopFailure, createMachineKey, machineKey, signLinkCode } from '@/platform/desktop'
import FormField from './FormField.vue'
import Notice from './Notice.vue'

// Desktop only. The machine key is created here if missing; only the public key and the
// signature of the code leave the machine. The API checks the proof itself.
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
    if ((await machineKey()) === null) await createMachineKey()
    const proof = await signLinkCode(code.value)
    await linkDevice(code.value.trim().toUpperCase(), proof.public_key, proof.signature, label.value.trim())
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
