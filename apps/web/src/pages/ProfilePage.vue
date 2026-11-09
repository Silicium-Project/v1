<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import { t } from '@/i18n'
import { formatDateTime } from '@/i18n/format'
import { listDevices, listJobs, profile, type Device, type JobSummary, type Profile } from '@/api/endpoints'
import { errorMessageKey, isSessionEnded } from '@/api/errors'
import { currentToken, endSession } from '@/state/session'
import Notice from '@/components/Notice.vue'

// Profil : qui est connecté, l'activité du compte et les derniers calculs. Les données viennent de
// l'API ; rien n'est stocké ici.
const router = useRouter()
const account = ref<Profile | null>(null)
const jobs = ref<JobSummary[]>([])
const devices = ref<Device[]>([])
const error = ref('')
const copied = ref(false)

const activeDevices = computed(() => devices.value.filter((device) => !device.revoked_at).length)
const recentJobs = computed(() =>
  [...jobs.value].sort((a, b) => b.created_at.localeCompare(a.created_at)).slice(0, 5),
)

async function load(): Promise<void> {
  const token = currentToken()
  if (!token) return
  try {
    const [me, list, machines] = await Promise.all([profile(token), listJobs(token), listDevices(token)])
    account.value = me
    jobs.value = list.jobs
    devices.value = machines.devices
  } catch (failure) {
    if (isSessionEnded(failure)) await endSession(router)
    else error.value = t(errorMessageKey(failure))
  }
}

async function copyAddress(): Promise<void> {
  if (!account.value?.address) return
  try {
    await navigator.clipboard.writeText(account.value.address)
    copied.value = true
  } catch {
    error.value = t('profile.copyFailed')
  }
}

onMounted(load)
</script>

<template>
  <section class="stack" aria-labelledby="profile-title">
    <h1 id="profile-title" tabindex="-1">{{ t('profile.title') }}</h1>

    <Notice kind="error" :message="error" />

    <section v-if="account" class="card stack" aria-labelledby="identity-title">
      <h2 id="identity-title">{{ t('profile.identity') }}</h2>
      <dl class="dl">
        <template v-if="account.address">
          <dt>{{ t('profile.wallet') }}</dt>
          <dd>
            <code>{{ account.address }}</code>
            <button type="button" class="btn btn-quiet" @click="copyAddress">{{ t('profile.copy') }}</button>
            <span v-if="copied" class="muted" role="status">{{ t('profile.copied') }}</span>
          </dd>
        </template>
        <template v-if="account.email">
          <dt>{{ t('account.email') }}</dt>
          <dd>{{ account.email }}</dd>
        </template>
        <dt>{{ t('profile.accountId') }}</dt>
        <dd><code>{{ account.id }}</code></dd>
        <dt>{{ t('account.created') }}</dt>
        <dd><time :datetime="account.created_at">{{ formatDateTime(account.created_at) }}</time></dd>
      </dl>
    </section>

    <section class="card stack" aria-labelledby="activity-title">
      <h2 id="activity-title">{{ t('profile.activity') }}</h2>
      <dl class="dl">
        <dt>{{ t('profile.jobs') }}</dt>
        <dd>{{ jobs.length }}</dd>
        <dt>{{ t('profile.machines') }}</dt>
        <dd>{{ devices.length }}</dd>
        <dt>{{ t('profile.activeMachines') }}</dt>
        <dd>{{ activeDevices }}</dd>
      </dl>
    </section>

    <section class="card stack" aria-labelledby="recent-title">
      <h2 id="recent-title">{{ t('profile.recentJobs') }}</h2>
      <p v-if="recentJobs.length === 0" class="muted">{{ t('profile.noJobs') }}</p>
      <ul v-else class="profile-list">
        <li v-for="job in recentJobs" :key="job.id">
          <RouterLink :to="{ name: 'job', params: { id: job.id } }">{{ job.label || job.id }}</RouterLink>
          <time class="muted" :datetime="job.created_at">{{ formatDateTime(job.created_at) }}</time>
        </li>
      </ul>
      <div class="actions">
        <RouterLink class="btn" to="/calculs">{{ t('profile.seeJobs') }}</RouterLink>
        <RouterLink class="btn btn-quiet" to="/machines">{{ t('profile.manageMachines') }}</RouterLink>
        <RouterLink class="btn btn-quiet" to="/parametres">{{ t('profile.openSettings') }}</RouterLink>
      </div>
    </section>
  </section>
</template>
