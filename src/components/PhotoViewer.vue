<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { getPhotoUrl } from "../api";
import type { Photo } from "../types";

const props = defineProps<{
  photo: Photo;
  hasPrev: boolean;
  hasNext: boolean;
  current: number;
  total: number;
}>();

const emit = defineEmits<{
  close: [];
  prev: [];
  next: [];
}>();

const fullSrc = ref<string>("");
const loading = ref(true);

function loadPhoto() {
  loading.value = true;
  fullSrc.value = getPhotoUrl(props.photo.path);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
  if (e.key === "ArrowLeft") emit("prev");
  if (e.key === "ArrowRight") emit("next");
}

onMounted(() => {
  loadPhoto();
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});

watch(() => props.photo, loadPhoto);
</script>

<template>
  <div class="viewer" @click.self="emit('close')">
    <button class="viewer-close" @click="emit('close')" title="关闭 (ESC)">
      &times;
    </button>

    <div class="viewer-counter">{{ current }} / {{ total }}</div>

    <button
      v-if="hasPrev"
      class="viewer-nav viewer-prev"
      @click.stop="emit('prev')"
      title="上一张 (←)"
    >
      ‹
    </button>

    <div class="viewer-image-wrap" @click.self="emit('close')">
      <div v-if="loading" class="viewer-spinner" />
      <img
        v-if="fullSrc"
        :src="fullSrc"
        :alt="photo.filename"
        class="viewer-img"
        :class="{ visible: !loading }"
        draggable="false"
        @load="loading = false"
      />
    </div>

    <button
      v-if="hasNext"
      class="viewer-nav viewer-next"
      @click.stop="emit('next')"
      title="下一张 (→)"
    >
      ›
    </button>

    <div class="viewer-filename">{{ photo.filename }}</div>
  </div>
</template>

<style scoped>
.viewer {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: rgba(0, 0, 0, 0.95);
  display: flex;
  align-items: center;
  justify-content: center;
}

.viewer-close {
  position: absolute;
  top: 16px;
  right: 24px;
  z-index: 10;
  background: none;
  border: none;
  color: #aaa;
  font-size: 2.5rem;
  cursor: pointer;
  line-height: 1;
  transition: color 0.15s;
}

.viewer-close:hover {
  color: #fff;
}

.viewer-counter {
  position: absolute;
  top: 20px;
  left: 24px;
  z-index: 10;
  color: #888;
  font-size: 0.9rem;
}

.viewer-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  z-index: 10;
  background: rgba(255, 255, 255, 0.08);
  border: none;
  color: #aaa;
  font-size: 3rem;
  cursor: pointer;
  width: 56px;
  height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: background 0.15s, color 0.15s;
}

.viewer-nav:hover {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}

.viewer-prev {
  left: 16px;
}

.viewer-next {
  right: 16px;
}

.viewer-image-wrap {
  max-width: calc(100vw - 140px);
  max-height: calc(100vh - 100px);
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
}

.viewer-img {
  max-width: 100%;
  max-height: calc(100vh - 100px);
  object-fit: contain;
  opacity: 0;
  transition: opacity 0.3s ease;
  user-select: none;
}

.viewer-img.visible {
  opacity: 1;
}

.viewer-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(255, 255, 255, 0.1);
  border-top-color: #667eea;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  position: absolute;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.viewer-filename {
  position: absolute;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  color: #666;
  font-size: 0.8rem;
}
</style>
