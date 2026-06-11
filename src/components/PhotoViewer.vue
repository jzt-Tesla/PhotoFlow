<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { getPhotoUrl, toggleFavorite } from "../api";
import { usePhotoStore } from "../stores/photoStore";
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

const photoStore = usePhotoStore();

const fullSrc = ref("");
const loading = ref(true);
const showInfo = ref(false);

function loadPhoto() {
  loading.value = true;
  fullSrc.value = getPhotoUrl(props.photo.path);
}

async function handleToggleFavorite() {
  try {
    const isFav = await toggleFavorite(props.photo.id);
    photoStore.updateFavorite(props.photo.id, isFav);
  } catch (e) {
    console.error("Failed to toggle favorite:", e);
  }
}

function onKeydown(e: KeyboardEvent) {
  // Ignore when typing in input/textarea or using modifier keys
  const tag = (e.target as HTMLElement)?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA") return;
  if (e.ctrlKey || e.altKey || e.metaKey) return;

  if (e.key === "Escape") emit("close");
  if (e.key === "ArrowLeft") emit("prev");
  if (e.key === "ArrowRight") emit("next");
  if (e.key === "f" || e.key === "F") handleToggleFavorite();
  if (e.key === "i" || e.key === "I") showInfo.value = !showInfo.value;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / (1024 * 1024)).toFixed(1) + " MB";
}

onMounted(() => {
  loadPhoto();
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});

watch(() => props.photo, () => {
  loadPhoto();
  showInfo.value = false;
});
</script>

<template>
  <div class="viewer" @click.self="emit('close')">
    <button class="viewer-close" @click="emit('close')" title="关闭 (ESC)">
      &times;
    </button>

    <div class="viewer-counter">{{ current }} / {{ total }}</div>

    <div class="viewer-actions">
      <button
        class="btn-fav"
        :class="{ active: photo.favorite }"
        @click="handleToggleFavorite"
        title="收藏 (F)"
      >
        {{ photo.favorite ? "★" : "☆" }}
      </button>
      <button class="btn-info" @click="showInfo = !showInfo" title="信息 (I)">
        ℹ
      </button>
    </div>

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

    <!-- Info panel -->
    <div v-if="showInfo" class="info-panel">
      <div class="info-section">
        <div class="info-label">文件名</div>
        <div class="info-value">{{ photo.filename }}</div>
      </div>
      <div class="info-section">
        <div class="info-label">尺寸</div>
        <div class="info-value">{{ photo.width }} × {{ photo.height }}</div>
      </div>
      <div class="info-section">
        <div class="info-label">文件大小</div>
        <div class="info-value">{{ formatFileSize(photo.file_size) }}</div>
      </div>
      <div v-if="photo.taken_time" class="info-section">
        <div class="info-label">拍摄时间</div>
        <div class="info-value">{{ photo.taken_time }}</div>
      </div>
      <div v-if="photo.camera_model" class="info-section">
        <div class="info-label">相机</div>
        <div class="info-value">{{ photo.camera_model }}</div>
      </div>
      <div v-if="photo.lens_model" class="info-section">
        <div class="info-label">镜头</div>
        <div class="info-value">{{ photo.lens_model }}</div>
      </div>
      <div v-if="photo.latitude && photo.longitude" class="info-section">
        <div class="info-label">GPS</div>
        <div class="info-value">{{ photo.latitude.toFixed(6) }}, {{ photo.longitude.toFixed(6) }}</div>
        <div class="info-map">
          <iframe
            :src="`https://www.openstreetmap.org/export/embed.html?bbox=${photo.longitude - 0.01},${photo.latitude - 0.005},${photo.longitude + 0.01},${photo.latitude + 0.005}&layer=mapnik&marker=${photo.latitude},${photo.longitude}`"
            class="map-iframe"
            loading="lazy"
            referrerpolicy="no-referrer"
          />
          <a
            :href="`https://www.openstreetmap.org/?mlat=${photo.latitude}&mlon=${photo.longitude}#map=15/${photo.latitude}/${photo.longitude}`"
            target="_blank"
            class="map-link"
          >在地图中打开 ↗</a>
        </div>
      </div>
      <div v-if="photo.tags.length > 0" class="info-section">
        <div class="info-label">标签</div>
        <div class="info-tags">
          <span v-for="tag in photo.tags" :key="tag" class="info-tag">{{ tag }}</span>
        </div>
      </div>
      <div class="info-section">
        <div class="info-label">路径</div>
        <div class="info-value info-path">{{ photo.path }}</div>
      </div>
    </div>
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

.viewer-actions {
  position: absolute;
  top: 16px;
  right: 80px;
  z-index: 10;
  display: flex;
  gap: 8px;
}

.btn-fav,
.btn-info {
  background: rgba(255, 255, 255, 0.08);
  border: none;
  color: #888;
  font-size: 1.3rem;
  cursor: pointer;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: background 0.15s, color 0.15s;
}

.btn-fav:hover,
.btn-info:hover {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}

.btn-fav.active {
  color: #ffb74d;
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

/* Info panel */
.info-panel {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 300px;
  background: rgba(15, 15, 35, 0.95);
  backdrop-filter: blur(10px);
  overflow-y: auto;
  padding: 60px 20px 20px;
  border-left: 1px solid #1a1a2e;
  z-index: 5;
}

.info-section {
  margin-bottom: 14px;
}

.info-label {
  color: #555;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 3px;
}

.info-value {
  color: #bbb;
  font-size: 0.85rem;
  word-break: break-all;
}

.info-path {
  font-size: 0.75rem;
  color: #666;
}

.info-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.info-tag {
  background: rgba(102, 126, 234, 0.2);
  color: #667eea;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 0.75rem;
}

.info-map {
  margin-top: 6px;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid #2a2a4a;
}

.map-iframe {
  width: 100%;
  height: 160px;
  border: none;
  display: block;
  filter: brightness(0.85) contrast(1.1);
}

.map-link {
  display: block;
  text-align: center;
  padding: 6px;
  font-size: 0.7rem;
  color: #667eea;
  text-decoration: none;
  background: rgba(102, 126, 234, 0.08);
  transition: background 0.15s;
}

.map-link:hover {
  background: rgba(102, 126, 234, 0.2);
}
</style>
