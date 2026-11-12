<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { t } from '@/i18n'
import { formatTime } from '@/i18n/format'
import { getNetworkStatus, type NetworkStatus } from '@/api/endpoints'
import RoleBars from '@/components/network/RoleBars.vue'
import OnlineSpark from '@/components/network/OnlineSpark.vue'

// Public page: the counts the node sees, refreshed every 30 seconds. Nothing here identifies a
// machine. The line keeps the samples taken since the page was opened (kept in memory).
const POLL_MS = 30_000
const MAX_SAMPLES = 24
interface Sample {
  value: number
  at: string
}
const status = ref<NetworkStatus | null>(null)
const samples = ref<Sample[]>([])
const loading = ref(true)
let timer: ReturnType<typeof setInterval> | undefined

const roles = computed(() => {
  const byRole = status.value?.by_role ?? {}
  return [
    { label: t('site.network.compute'), value: byRole.compute ?? 0 },
    { label: t('site.network.verify'), value: byRole.verify ?? 0 },
  ]
})

const peak = computed(() => Math.max(0, ...samples.value.map((sample) => sample.value)))
const first = computed(() => samples.value[0])
const last = computed(() => samples.value[samples.value.length - 1])

async function load(): Promise<void> {
  loading.value = true
  try {
    const next = await getNetworkStatus()
    status.value = next
    if (next.node_reachable) {
      samples.value = [...samples.value, { value: next.online, at: next.updated_at }].slice(-MAX_SAMPLES)
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
      <h1 id="network-title" tabindex="-1">{{ t('site.network.title') }}</h1>
      <p class="site-lede">{{ t('site.network.lede') }}</p>
    </section>

    <section class="site-section site-container" aria-labelledby="network-title">
      <p v-if="loading && !status" role="status" class="muted">{{ t('common.loading') }}</p>
      <p v-else-if="!status || !status.node_reachable" class="site-notice" role="status">
        {{ t('site.network.nodeUnreachable') }}
      </p>
      <div v-if="status?.node_reachable" class="network-stats">
        <article class="stat-tile">
          <p class="stat-tile__label">
            <span class="network-live__dot" aria-hidden="true"></span>
            {{ t('site.network.online') }}
          </p>
          <p class="stat-tile__value">{{ status.online }}</p>
          <p class="stat-tile__note">{{ t('site.network.updated', { time: formatTime(status.updated_at) }) }}</p>
        </article>
        <article class="stat-tile">
          <p class="stat-tile__label">{{ t('site.network.compute') }}</p>
          <p class="stat-tile__value">{{ status.by_role.compute ?? 0 }}</p>
        </article>
        <article class="stat-tile">
          <p class="stat-tile__label">{{ t('site.network.verify') }}</p>
          <p class="stat-tile__value">{{ status.by_role.verify ?? 0 }}</p>
        </article>
      </div>

      <div class="network-panels">
        <article class="panel network-panel--wide">
          <header class="panel__header">
            <h2>{{ t('site.network.chartTitle') }}</h2>
            <p class="muted">{{ t('site.network.historyHint') }}</p>
          </header>
          <p v-if="samples.length === 0" class="muted network-empty">{{ t('site.network.noSamples') }}</p>
          <template v-else>
            <OnlineSpark
              :values="samples.map((sample) => sample.value)"
              :summary="t('site.network.onlineChart', { count: last?.value ?? 0 })"
            />
            <p class="network-axis">
              <span>{{ first ? formatTime(first.at) : '' }}</span>
              <span>{{ t('site.network.peak', { count: peak }) }}</span>
              <span>{{ last ? formatTime(last.at) : '' }}</span>
            </p>
          </template>
        </article>
      </div>

      <div class="network-panels">
        <article class="panel">
          <header class="panel__header">
            <h2>{{ t('site.network.infoTitle') }}</h2>
          </header>
          <dl class="network-info">
            <div>
              <dt>{{ t('site.network.online') }}</dt>
              <dd>{{ t('site.network.infoWindow') }}</dd>
            </div>
            <div>
              <dt>{{ t('site.network.refresh') }}</dt>
              <dd>{{ t('site.network.infoRefresh') }}</dd>
            </div>
            <div>
              <dt>{{ t('site.network.nodeReachable') }}</dt>
              <dd>{{ status?.node_reachable ? t('site.network.live') : t('site.network.nodeUnreachable') }}</dd>
            </div>
            <div>
              <dt>{{ t('site.network.infoSourceLabel') }}</dt>
              <dd>{{ t('site.network.infoSource') }}</dd>
            </div>
          </dl>
        </article>

        <article class="panel">
          <header class="panel__header">
            <h2>{{ t('site.network.byRole') }}</h2>
          </header>
          <RoleBars :rows="roles" :summary="t('site.network.rolesChart')" />
          <p class="muted">{{ t('site.network.infoRoles') }}</p>
        </article>
      </div>

      <div class="actions">
        <button type="button" class="btn btn-quiet" :disabled="loading" @click="load">
          {{ t('site.network.refresh') }}
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.network-empty {
  padding: 2rem 0;
  text-align: center;
}

.network-axis {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  margin: 0.5rem 0 0;
  font-size: 0.85rem;
  color: var(--color-text-muted);
}

.network-info {
  margin: 0;
}

.network-info div {
  padding: 0.75rem 0;
  border-bottom: 1px solid var(--color-border);
}

.network-info div:last-child {
  border-bottom: 0;
}

.network-info dt {
  font-weight: 600;
}

.network-info dd {
  margin: 0.2rem 0 0;
  color: var(--color-text-muted);
}
</style>
