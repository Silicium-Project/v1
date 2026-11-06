<script setup lang="ts">
// Errors use role="alert" (announced at once). Other notices use role="status" (polite).
defineProps<{ kind: 'error' | 'success' | 'info'; message: string }>()
</script>

<template>
  <p v-if="message" class="notice" :class="`notice--${kind}`" :role="kind === 'error' ? 'alert' : 'status'">
    {{ message }}
  </p>
</template>
