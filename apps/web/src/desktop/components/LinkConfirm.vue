<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { t } from '@/i18n'
import { ApiError } from '@/api/client'
import { errorMessageKey } from '@/api/errors'
import { previewLinkCode, type LinkPreview } from '@/api/endpoints'
import { DesktopFailure, machineKey } from '@/platform/desktop'
import { linkThisMachine } from '@/desktop/link'
import FormField from '@/components/FormField.vue'
import Notice from '@/components/Notice.vue'

// One-click linking: the website opened the application with a code. The account the code
// belongs to is shown first; nothing is linked until the user confirms.
const props = defineProps<{ code: string }>()
const emit = defineEmits<{ linked: []; cancel: [] }>()

const preview = ref<LinkPreview | null>(null)
const key = ref<string | null>(null)
const label = ref('')
const loading = ref(true)
const busy = ref(false)
const error = ref('')
const success = ref('')

function failureText(failure: unknown): string {
  if (failure instanceof ApiError && failure.code === 'bad_link_code') return t('desktop.previewFailed')
  if (failure instanceof DesktopFailure) return t(`desktop.${failure.code}`)
  return t(errorMessageKey(failure))
}

onMounted(async () => {
  try {
    preview.value = await previewLinkCode(props.code)
    key.value = await machineKey()
  } catch (failure) {
    error.value = failureText(failure)
  } finally {
    loading.value = false
  }
})

async function confirm(): Promise<void> {
  busy.value = true
  error.value = ''
  try {
    await linkThisMachine(props.code, label.value)
    success.value = t('desktop.linkedDone')
    emit('linked')
  } catch (failure) {
    error.value = failureText(failure)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="card stack" aria-labelledby="confirm-title">
    <h2 id="confirm-title">{{ t('desktop.confirmTitle') }}</h2>
    <p>{{ t('desktop.confirmText') }}</p>
    <Notice kind="error" :message="error" />
    <Notice kind="success" :message="success" />
    <p v-if="loading" role="status" class="muted">{{ t('desktop.confirmLoading') }}</p>

    <template v-if="preview && !success">
      <dl class="dl">
        <dt>{{ t('desktop.confirmAccount') }}</dt>
        <dd>{{ preview.account }}</dd>
        <dt>{{ t('desktop.confirmMachine') }}</dt>
        <dd><code>{{ key ? `${key.slice(0, 16)}…` : '—' }}</code></dd>
      </dl>
      <form class="stack" novalidate @submit.prevent="confirm">
        <FormField id="confirm-label" v-model="label" :label="t('desktop.confirmLabel')" autocomplete="off" />
        <div class="actions">
          <button type="submit" class="btn btn-primary" :disabled="busy">
            {{ busy ? t('common.loading') : t('desktop.confirmSubmit') }}
          </button>
          <button type="button" class="btn btn-quiet" :disabled="busy" @click="emit('cancel')">
            {{ t('desktop.confirmCancel') }}
          </button>
        </div>
      </form>
    </template>
  </section>
</template>
