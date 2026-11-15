<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { t } from '@/i18n'
import { formatTime } from '@/i18n/format'
import { getNetworkStatus, type NetworkStatus } from '@/api/endpoints'
import OnlineSpark from '@/components/network/OnlineSpark.vue'

// Live card for the home page: only what the node reports, refreshed every 30 seconds. The line
// keeps the samples received since the page was opened; nothing older is shown or invented.
const POLL_MS = 30_000
const MAX_SAMPLES = 24
const status = ref<NetworkStatus | null>(null)
const samples = ref<number[]>([])
let timer: ReturnType<typeof setInterval> | undefined

async function load(): Promise<void> {
  try {
    const next = await getNetworkStatus()
    status.value = next
    if (next.node_reachable) samples.value = [...samples.value, next.online].slice(-MAX_SAMPLES)
  } catch {
    status.value = null
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
  <article class="panel live-network" aria-labelledby="live-network-title">
    <header class="panel__header live-network__header">
      <h2 id="live-network-title">{{ t('site.home.liveTitle') }}</h2>
      <p v-if="status?.node_reachable" class="live-network__state">
        <span class="network-live__dot" aria-hidden="true"></span>
        {{ t('site.network.live') }}
      </p>
    </header>

    <p v-if="!status" class="muted" role="status">{{ t('common.loading') }}</p>
    <p v-else-if="!status.node_reachable" class="site-notice" role="status">
      {{ t('site.network.nodeUnreachable') }}
    </p>
    <template v-else>
      <dl class="live-network__stats">
        <div>
          <dt>{{ t('site.network.online') }}</dt>
          <dd>{{ status.online }}</dd>
        </div>
        <div>
          <dt>{{ t('site.network.compute') }}</dt>
          <dd>{{ status.by_role.compute ?? 0 }}</dd>
        </div>
        <div>
          <dt>{{ t('site.network.verify') }}</dt>
          <dd>{{ status.by_role.verify ?? 0 }}</dd>
        </div>
        <div>
          <dt>{{ t('site.network.claim') }}</dt>
          <dd>{{ status.by_role.claim ?? 0 }}</dd>
        </div>
      </dl>
      <OnlineSpark :values="samples" :summary="t('site.network.onlineChart', { count: status.online })" />
      <p v-if="samples.length < 2" class="muted live-network__note">{{ t('site.home.liveWaiting') }}</p>
      <p class="muted live-network__note">
        {{ t('site.home.liveNote') }}
        {{ t('site.network.updated', { time: formatTime(status.updated_at) }) }}
      </p>
    </template>
  </article>
</template>

<style scoped>
.live-network__header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.live-network__header h2 {
  margin: 0;
  font-size: 1.1rem;
}

.live-network__state {
  margin: 0;
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  color: var(--color-success);
  font-weight: 600;
  font-size: 0.95rem;
}

.live-network__stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.75rem;
  margin: 1rem 0;
}

.live-network__stats div {
  min-width: 0;
}

.live-network__stats dt {
  font-size: 0.9rem;
  color: var(--color-text-muted);
}

.live-network__stats dd {
  margin: 0.15rem 0 0;
  font-size: 1.9rem;
  font-weight: 600;
  letter-spacing: -0.02em;
}

.live-network__note {
  margin: 0.75rem 0 0;
  font-size: 0.9rem;
}
</style>
