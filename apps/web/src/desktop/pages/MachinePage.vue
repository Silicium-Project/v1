<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { t } from '@/i18n'
import { formatDateTime } from '@/i18n/format'
import Notice from '@/components/Notice.vue'
import DesktopLinkForm from '@/desktop/components/DesktopLinkForm.vue'
import LinkConfirm from '@/desktop/components/LinkConfirm.vue'
import { currentLinkRecord, type LinkRecord } from '@/desktop/link-record'
import { requestedCode } from '@/desktop/pending-link'

// A link opened from the website shows its confirmation here; otherwise the code can be typed.
function clearRequest(): void {
  requestedCode.value = null
}

// Whether this machine is linked to an account, and since when, is read from this machine.
const linked = ref<LinkRecord | null>(null)
const checked = ref(false)

async function refreshLink(): Promise<void> {
  linked.value = await currentLinkRecord()
  checked.value = true
}

const linkedText = computed(() =>
  linked.value
    ? t('desktop.linkedSince', {
        date: formatDateTime(linked.value.linked_at),
        label: linked.value.label || '—',
      })
    : '',
)

onMounted(() => void refreshLink())
</script>

<template>
  <section class="stack" aria-labelledby="machine-title">
    <h1 id="machine-title" tabindex="-1">{{ t('desktop.machineTitle') }}</h1>
    <template v-if="checked">
      <Notice v-if="linked" kind="success" :message="linkedText" />
      <Notice v-else kind="info" :message="t('desktop.notLinked')" />
    </template>
    <LinkConfirm
      v-if="requestedCode"
      :key="requestedCode"
      :code="requestedCode"
      @linked="clearRequest(); void refreshLink()"
      @cancel="clearRequest"
    />
    <DesktopLinkForm v-else @linked="refreshLink" />
  </section>
</template>
