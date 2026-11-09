<script setup lang="ts">
import { computed } from 'vue'

// A labelled text input. The label is always visible; the hint and the error are linked to
// the input with aria-describedby, and an invalid input carries aria-invalid.
const props = defineProps<{
  id: string
  label: string
  modelValue: string
  type?: string
  autocomplete?: string
  hint?: string
  error?: string
  required?: boolean
  minlength?: number
  maxlength?: number
}>()

const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const describedBy = computed(() => {
  const ids = [props.hint ? `${props.id}-hint` : '', props.error ? `${props.id}-error` : ''].filter(Boolean)
  return ids.length > 0 ? ids.join(' ') : undefined
})
</script>

<template>
  <div class="field">
    <label :for="id">{{ label }}</label>
    <input
      :id="id"
      class="input"
      :type="type ?? 'text'"
      :value="modelValue"
      :autocomplete="autocomplete"
      :required="required"
      :minlength="minlength"
      :maxlength="maxlength"
      :aria-required="required ? 'true' : undefined"
      :aria-invalid="error ? 'true' : undefined"
      :aria-describedby="describedBy"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
    <p v-if="hint" :id="`${id}-hint`" class="field__hint">{{ hint }}</p>
    <p v-if="error" :id="`${id}-error`" class="field__error">{{ error }}</p>
  </div>
</template>
