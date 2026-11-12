<script setup lang="ts">
import { computed, reactive } from 'vue'
import { messages } from '@/i18n'
import { CONTACT_EMAIL, mailtoLink } from './contact'

const contact = computed(() => messages().site.contact)
const form = reactive({ name: '', email: '', subject: 'general', message: '' })

// Le message part depuis la messagerie du visiteur : le site ne le transmet à personne.
function prepare(): void {
  const label = contact.value.subjects.find((item) => item.value === form.subject)?.label ?? ''
  const body = [
    form.message.trim(),
    '',
    `${contact.value.nameLabel} : ${form.name.trim()}`,
    `${contact.value.emailLabel} : ${form.email.trim()}`,
  ].join('\n')
  window.location.href = mailtoLink(`[Silicium] ${label}`, body)
}
</script>

<template>
  <div class="site-page">
    <section class="site-hero site-container site-hero--plain">
      <h1 id="contact-title" tabindex="-1">{{ contact.title }}</h1>
      <p class="site-lede">{{ contact.lede }}</p>
    </section>

    <section class="site-section site-container site-columns" aria-labelledby="contact-title">
      <form class="card stack contact-form" novalidate @submit.prevent="prepare">
        <div class="field">
          <label for="contact-name">{{ contact.nameLabel }}</label>
          <input id="contact-name" v-model="form.name" class="input" type="text" autocomplete="name" />
        </div>
        <div class="field">
          <label for="contact-email">{{ contact.emailLabel }}</label>
          <input id="contact-email" v-model="form.email" class="input" type="email" autocomplete="email" required />
        </div>
        <div class="field">
          <label for="contact-subject">{{ contact.subjectLabel }}</label>
          <select id="contact-subject" v-model="form.subject" class="input">
            <option v-for="item in contact.subjects" :key="item.value" :value="item.value">{{ item.label }}</option>
          </select>
        </div>
        <div class="field">
          <label for="contact-message">{{ contact.messageLabel }}</label>
          <p id="contact-privacy" class="field__hint">{{ contact.privacy }}</p>
          <textarea
            id="contact-message"
            v-model="form.message"
            class="input"
            rows="6"
            required
            aria-describedby="contact-privacy"
          ></textarea>
        </div>
        <div class="actions">
          <button type="submit" class="btn btn-primary" :disabled="!form.email || !form.message.trim()">
            {{ contact.send }}
          </button>
        </div>
        <p class="muted">{{ contact.sendHint }}</p>
      </form>

      <aside class="stack">
        <h2>{{ contact.direct }}</h2>
        <p><a :href="`mailto:${CONTACT_EMAIL}`">{{ CONTACT_EMAIL }}</a></p>
      </aside>
    </section>
  </div>
</template>

<style scoped>
.contact-form {
  padding: 1.5rem;
}
</style>
