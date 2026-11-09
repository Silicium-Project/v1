<script setup lang="ts">
// Un champ numérique libellé. Une saisie vide ou invalide garde la dernière valeur valide.
const props = defineProps<{
  id: string
  label: string
  modelValue: number
  min?: number
  max?: number
  step?: number
}>()

const emit = defineEmits<{ 'update:modelValue': [value: number] }>()

function onInput(event: Event): void {
  const value = Number((event.target as HTMLInputElement).value)
  if (!Number.isFinite(value)) return
  let clamped = value
  if (props.min !== undefined) clamped = Math.max(props.min, clamped)
  if (props.max !== undefined) clamped = Math.min(props.max, clamped)
  emit('update:modelValue', clamped)
}
</script>

<template>
  <div class="field">
    <label :for="id">{{ label }}</label>
    <input
      :id="id"
      class="input"
      type="number"
      :value="modelValue"
      :min="min"
      :max="max"
      :step="step ?? 'any'"
      @change="onInput"
    />
  </div>
</template>
