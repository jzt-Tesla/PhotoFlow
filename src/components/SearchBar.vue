<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{ modelValue?: string }>();
const emit = defineEmits<{ search: [query: string] }>();
const query = ref(props.modelValue ?? "");
let timer: ReturnType<typeof setTimeout> | null = null;

// Sync external modelValue changes (e.g. App.vue clearing search on navigate)
watch(
  () => props.modelValue,
  (val) => {
    if (val !== undefined && val !== query.value) query.value = val;
  }
);

function onInput() {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => emit("search", query.value.trim()), 300);
}

function onClear() {
  query.value = "";
  emit("search", "");
}
</script>

<template>
  <div class="search-bar">
    <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
    <input
      v-model="query"
      type="text"
      placeholder="搜索文件名或标签…"
      class="search-input"
      @input="onInput"
    />
    <button v-if="query" class="btn-clear" @click="onClear">×</button>
  </div>
</template>

<style scoped>
.search-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  max-width: 400px;
  background: #1a1a2e;
  border: 1px solid #2a2a4a;
  border-radius: 8px;
  padding: 0 12px;
  transition: border-color 0.15s;
}

.search-bar:focus-within {
  border-color: #667eea;
}

.search-icon {
  color: #555;
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  color: #e0e0e0;
  font-size: 0.85rem;
  padding: 8px 0;
}

.search-input::placeholder {
  color: #555;
}

.btn-clear {
  background: none;
  border: none;
  color: #888;
  font-size: 1.2rem;
  cursor: pointer;
  padding: 0 4px;
}

.btn-clear:hover {
  color: #e57373;
}
</style>
