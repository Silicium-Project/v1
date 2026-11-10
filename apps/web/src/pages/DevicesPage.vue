<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { t } from '@/i18n'
import { formatDateTime, formatTime } from '@/i18n/format'
import { listDevices, newLinkCode, purgeDevice, revokeDevice, type Device, type LinkCode } from '@/api/endpoints'
import { errorMessageKey, isSessionEnded } from '@/api/errors'
import { currentToken, endSession } from '@/state/session'
import EmptyState from '@/components/EmptyState.vue'
import { desktopLinkUrl } from '@/platform/desktop-link'
import Notice from '@/components/Notice.vue'

const router = useRouter()
const devices = ref<Device[]>([])
const loading = ref(true)
const error = ref('')
const success = ref('')
const code = ref<LinkCode | null>(null)
const generating = ref(false)
const pendingRevoke = ref('')
const revoking = ref('')
const pendingPurge = ref('')
const purging = ref('')

async function run(action: (token: string) => Promise<void>): Promise<void> {
  const token = currentToken()
  if (!token) return
  error.value = ''
  try {
    await action(token)
  } catch (failure) {
    if (isSessionEnded(failure)) {
      await endSession(router)
      return
    }
    error.value = t(errorMessageKey(failure))
  }
}

async function load(): Promise<void> {
  loading.value = true
  await run(async (token) => {
    devices.value = (await listDevices(token)).devices
  })
  loading.value = false
}

// One click: issue a code, then open the desktop application on it. The code stays on the page
// as a fallback for typing it into the application. The link opens within the click, so the
// browser accepts it.
async function linkHere(): Promise<void> {
  generating.value = true
  success.value = ''
  await run(async (token) => {
    const issued = await newLinkCode(token)
    code.value = issued
    window.location.href = desktopLinkUrl(issued.code)
  })
  generating.value = false
}

async function purge(id: string): Promise<void> {
  purging.value = id
  success.value = ''
  await run(async (token) => {
    await purgeDevice(token, id)
    pendingPurge.value = ''
    success.value = t('devices.purgeDone')
    devices.value = (await listDevices(token)).devices
  })
  purging.value = ''
}

async function revoke(id: string): Promise<void> {
  revoking.value = id
  success.value = ''
  await run(async (token) => {
    await revokeDevice(token, id)
    pendingRevoke.value = ''
    success.value = t('devices.revokeDone')
    devices.value = (await listDevices(token)).devices
  })
  revoking.value = ''
}

function isActive(device: Device): boolean {
  return !device.revoked_at
}

// A machine reports every minute: silent for three minutes, it is shown offline.
const ONLINE_MS = 3 * 60 * 1000
function isOnline(device: Device): boolean {
  const seen = device.telemetry ? Date.parse(device.telemetry.received_at) : NaN
  return Number.isFinite(seen) && Date.now() - seen < ONLINE_MS
}

onMounted(load)
</script>

<template>
  <section class="stack" aria-labelledby="devices-title">
    <h1 id="devices-title" tabindex="-1">{{ t('devices.title') }}</h1>

    <Notice kind="error" :message="error" />
    <Notice kind="success" :message="success" />

    <div class="card stack">
      <h2>{{ t('devices.linkTitle') }}</h2>
      <p>{{ t('devices.linkText') }}</p>
      <div class="actions">
        <button type="button" class="btn btn-primary" :disabled="generating" @click="linkHere">
          {{ t('desktop.openDesktop') }}
        </button>
      </div>
      <div v-if="code">
        <p id="code-label">{{ t('desktop.fallbackCode') }}</p>
        <code class="code-display" aria-labelledby="code-label">{{ code.code }}</code>
        <p class="muted">{{ t('devices.codeExpires', { time: formatTime(code.expires_at) }) }}</p>
        <p class="muted">
          {{ t('desktop.notOpened') }} <RouterLink to="/installation">{{ t('desktop.installNode') }}</RouterLink>
        </p>
      </div>
    </div>

    <p v-if="loading" role="status" class="muted">{{ t('common.loading') }}</p>

    <EmptyState v-if="!loading && !error && devices.length === 0" :title="t('devices.empty')">
      <p>{{ t('devices.emptyText') }}</p>
    </EmptyState>

    <ul v-if="devices.length > 0" class="list">
      <li v-for="device in devices" :key="device.id" class="card">
        <div class="stack">
          <p class="list__title">{{ device.label || t('devices.unnamed') }}</p>
          <p class="list__meta">
            <span class="badge" :class="isActive(device) ? 'badge--success' : 'badge--neutral'">
              {{ isActive(device) ? t('devices.active') : t('devices.revoked') }}
            </span>
            <time :datetime="device.linked_at">{{ t('devices.linkedAt', { date: formatDateTime(device.linked_at) }) }}</time>
            <time v-if="device.revoked_at" :datetime="device.revoked_at">
              {{ t('devices.revokedAt', { date: formatDateTime(device.revoked_at) }) }}
            </time>
          </p>
          <p>
            <span class="muted">{{ t('devices.keyLabel') }} :</span>
            <code>{{ device.public_key }}</code>
          </p>
          <div v-if="isActive(device)" class="device-telemetry">
            <template v-if="device.telemetry">
              <p class="list__meta">
                <span class="badge" :class="isOnline(device) ? 'badge--success' : 'badge--neutral'">
                  {{ isOnline(device) ? t('devices.telemetryOnline') : t('devices.telemetryOffline') }}
                </span>
                <time :datetime="device.telemetry.received_at">
                  {{ t('devices.telemetrySeen', { time: formatDateTime(device.telemetry.received_at) }) }}
                </time>
              </p>
              <dl class="device-telemetry__grid">
                <div>
                  <dt>{{ t('devices.telemetryCpu') }}</dt>
                  <dd>{{ device.telemetry.cpu_pct.toFixed(1) }} %</dd>
                </div>
                <div>
                  <dt>{{ t('devices.telemetryMemory') }}</dt>
                  <dd>{{ device.telemetry.mem_pct.toFixed(1) }} %</dd>
                </div>
                <div>
                  <dt>{{ t('devices.telemetryRoles') }}</dt>
                  <dd>
                    {{ [device.telemetry.compute ? t('devices.telemetryRoleCompute') : '', device.telemetry.verify ? t('devices.telemetryRoleVerify') : ''].filter(Boolean).join(', ') || '—' }}
                  </dd>
                </div>
              </dl>
              <p class="muted">
                {{ t('devices.telemetryTasks', { active: device.telemetry.tasks_active, progress: device.telemetry.tasks_in_progress }) }}
                ·
                {{ t('devices.telemetryDone', { done: device.telemetry.tasks_finalized, other: device.telemetry.tasks_other }) }}
              </p>
              <p class="muted">
                {{ t('devices.telemetryVersion', { version: device.telemetry.version, cores: device.telemetry.cpu_cores }) }}
              </p>
            </template>
            <p v-else class="muted">{{ t('devices.telemetryNone') }}</p>
          </div>
          <div v-if="isActive(device)" class="actions">
            <button
              v-if="pendingRevoke !== device.id"
              type="button"
              class="btn"
              @click="pendingRevoke = device.id"
            >
              {{ t('devices.revoke') }}
            </button>
            <template v-else>
              <button type="button" class="btn btn-danger" :disabled="revoking === device.id" @click="revoke(device.id)">
                {{ t('devices.revokeConfirm') }}
              </button>
              <button type="button" class="btn" @click="pendingRevoke = ''">{{ t('common.cancel') }}</button>
            </template>
          </div>
          <div v-else class="stack">
            <p class="muted">{{ t('devices.purgeHint') }}</p>
            <div class="actions">
              <button
                v-if="pendingPurge !== device.id"
                type="button"
                class="btn"
                @click="pendingPurge = device.id"
              >
                {{ t('devices.purge') }}
              </button>
              <template v-else>
                <button type="button" class="btn btn-danger" :disabled="purging === device.id" @click="purge(device.id)">
                  {{ t('devices.purgeConfirm') }}
                </button>
                <button type="button" class="btn" @click="pendingPurge = ''">{{ t('common.cancel') }}</button>
              </template>
            </div>
          </div>
        </div>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.device-telemetry__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
  gap: 0.75rem;
  margin: 0.5rem 0;
}

.device-telemetry__grid dt {
  color: var(--color-text-muted);
  font-size: 0.9rem;
}

.device-telemetry__grid dd {
  margin: 0.15rem 0 0;
  font-size: 1.35rem;
  font-weight: 600;
}
</style>
