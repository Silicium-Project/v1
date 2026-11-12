<script setup lang="ts">
// Un curseur libellé : on bouge la barre, la valeur s'affiche à côté (pas de chiffre à saisir).
defineProps<{ id: string; label: string; modelValue: number; min: number; max: number; step?: number }>()

const emit = defineEmits<{ 'update:modelValue': [value: number] }>()

function onInput(event: Event): void {
  emit('update:modelValue', Number((event.target as HTMLInputElement).value))
}
</script>

<template>
  <div class="field slider-field">
    <label :for="id">
      {{ label }} : <output :for="id">{{ modelValue }}</output>
    </label>
    <input :id="id" class="slider" type="range" :min="min" :max="max" :step="step ?? 1" :value="modelValue" @input="onInput" />
  </div>
</template>

<style scoped>
.slider-field {
  display: grid;
  gap: 0.35rem;
}

.slider {
  width: 100%;
  accent-color: var(--color-accent);
}
</style>
