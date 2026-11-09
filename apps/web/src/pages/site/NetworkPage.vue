<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { t } from '@/i18n'
import { formatTime } from '@/i18n/format'
import { getNetworkStatus, type NetworkStatus } from '@/api/endpoints'
import RoleBars from '@/components/network/RoleBars.vue'
import OnlineSpark from '@/components/network/OnlineSpark.vue'

// Public page: the counts the node sees, refreshed every 30 seconds. Nothing here identifies a
// machine. The line keeps the last samples taken since the page was opened (kept in memory).
const POLL_MS = 30_000
const MAX_SAMPLES = 24
const status = ref<NetworkStatus | null>(null)
const samples = ref<number[]>([])
const loading = ref(true)
let timer: ReturnType<typeof setInterval> | undefined

const roles = computed(() => {
  const byRole = status.value?.by_role ?? {}
  return [
    { label: t('site.network.compute'), value: byRole.compute ?? 0 },
    { label: t('site.network.verify'), value: byRole.verify ?? 0 },
  ]
})

async function load(): Promise<void> {
  loading.value = true
  try {
    const next = await getNetworkStatus()
    status.value = next
    if (next.node_reachable) {
      samples.value = [...samples.value, next.online].slice(-MAX_SAMPLES)
    }
  } catch {
    status.value = null
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  void load()
  timer = setInterval(() => void load(), POLL_MS)
})
onBeforeUnmount(() => {
  if (timer !== undefined) clearInterval(timer)
})
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container site-hero--plain">
      <p class="site-kicker">{{ t('site.network.kicker') }}</p>
      <h1 id="network-title" tabindex="-1">{{ t('site.network.title') }}</h1>
      <p class="site-lede">{{ t('site.network.lede') }}</p>
    </section>

    <section class="site-section site-container" aria-labelledby="network-title">
      <p v-if="loading && !status" role="status" class="muted">{{ t('common.loading') }}</p>
      <p v-else-if="!status || !status.node_reachable" class="site-notice" role="status">
        {{ t('site.network.nodeUnreachable') }}
      </p>
      <template v-else>
        <div class="network-stats">
          <article class="stat-tile">
            <p class="stat-tile__label">
              <span class="network-live__dot" aria-hidden="true"></span>
              {{ t('site.network.online') }}
            </p>
            <p class="stat-tile__value">{{ status.online }}</p>
            <p class="stat-tile__note">{{ t('site.network.live') }}</p>
          </article>
          <article class="stat-tile">
            <p class="stat-tile__label">{{ t('site.network.verify') }}</p>
            <p class="stat-tile__value">{{ status.by_role.verify ?? 0 }}</p>
            <p class="stat-tile__note">{{ t('site.network.updated', { time: formatTime(status.updated_at) }) }}</p>
          </article>
        </div>

        <article class="panel network-panel--wide">
          <header class="panel__header">
            <h2>{{ t('site.network.online') }}</h2>
            <p class="muted">{{ t('site.network.historyHint') }}</p>
          </header>
          <OnlineSpark
            :values="samples"
            :summary="t('site.network.onlineChart', { count: status.online })"
          />
        </article>

        <div class="network-panels">
          <article class="panel">
            <header class="panel__header">
              <h2>{{ t('site.network.byRole') }}</h2>
            </header>
            <RoleBars :rows="roles" :summary="t('site.network.rolesChart')" />
          </article>
        </div>
      </template>
      <div class="actions">
        <button type="button" class="btn btn-quiet" :disabled="loading" @click="load">
          {{ t('site.network.refresh') }}
        </button>
      </div>
    </section>
  </div>
</template>
