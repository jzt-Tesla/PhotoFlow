<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getThumbnailUrl } from "../api";
import type { Photo } from "../types";

// Bounded cache for thumbnail URLs — evicts oldest entries at capacity
const MAX_CACHE_SIZE = 10000;
const urlCache = new Map<string, string>();

function getCachedUrl(thumbPath: string): string {
  let url = urlCache.get(thumbPath);
  if (!url) {
    url = getThumbnailUrl(thumbPath);
    if (urlCache.size >= MAX_CACHE_SIZE) {
      const firstKey = urlCache.keys().next().value;
      if (firstKey !== undefined) urlCache.delete(firstKey);
    }
    urlCache.set(thumbPath, url);
  }
  return url;
}

const props = defineProps<{
  photo: Photo;
}>();

const thumbSrc = ref<string>("");
const loaded = ref(false);
const error = ref(false);

onMounted(() => {
  thumbSrc.value = getCachedUrl(props.photo.thumbnail_path);
});

function onImgLoad() {
  loaded.value = true;
}

function onImgError() {
  error.value = true;
}
</script>

<template>
  <div class="thumb-card">
    <div v-if="!loaded && !error" class="thumb-placeholder" />
    <img
      v-if="!error"
      :src="thumbSrc"
      :alt="photo.filename"
      class="thumb-img"
      :class="{ visible: loaded }"
      loading="lazy"
      @load="onImgLoad"
      @error="onImgError"
    />
    <div v-if="error" class="thumb-error">!</div>
    <div class="thumb-name">{{ photo.filename }}</div>
  </div>
</template>

<style scoped>
.thumb-card {
  width: 100%;
  height: 100%;
  border-radius: 6px;
  overflow: hidden;
  cursor: pointer;
  position: relative;
  background: #16213e;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.thumb-card:hover {
  transform: scale(1.03);
  box-shadow: 0 4px 20px rgba(102, 126, 234, 0.3);
}

.thumb-placeholder {
  width: 100%;
  height: 100%;
  background: #16213e;
  animation: shimmer 1.5s infinite;
  position: absolute;
  inset: 0;
}

@keyframes shimmer {
  0% { opacity: 0.5; }
  50% { opacity: 1; }
  100% { opacity: 0.5; }
}

.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity 0.3s ease;
}

.thumb-img.visible {
  opacity: 1;
}

.thumb-error {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #f55;
  font-size: 2rem;
}

.thumb-name {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 6px 8px;
  background: linear-gradient(transparent, rgba(0, 0, 0, 0.7));
  font-size: 0.7rem;
  color: #ddd;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
