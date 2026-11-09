<script setup lang="ts">
import { computed } from 'vue'
import type { Rgb } from '@/scene/types'

// Une couleur (0–255 par canal) saisie comme un sélecteur de couleur hexadécimal.
const props = defineProps<{ id: string; label: string; modelValue: Rgb }>()
const emit = defineEmits<{ 'update:modelValue': [value: Rgb] }>()

const hex = computed(
  () => `#${props.modelValue.map((channel) => Math.round(channel).toString(16).padStart(2, '0')).join('')}`,
)

function onInput(event: Event): void {
  const value = (event.target as HTMLInputElement).value
  const channels = [1, 3, 5].map((offset) => parseInt(value.slice(offset, offset + 2), 16))
  if (channels.every((channel) => Number.isFinite(channel)))
    emit('update:modelValue', channels as Rgb)
}
</script>

<template>
  <div class="field">
    <label :for="id">{{ label }}</label>
    <input :id="id" class="input" type="color" :value="hex" @input="onInput" />
  </div>
</template>
