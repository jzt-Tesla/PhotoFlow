<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { selectAndScanDirectory, rescanDirectory, loadPhotos, getPhotoCount } from "./api";
import type { Photo, ScanResult } from "./types";
import { listen } from "@tauri-apps/api/event";
import PhotoGallery from "./components/PhotoGallery.vue";
import PhotoViewer from "./components/PhotoViewer.vue";

const status = ref<"idle" | "scanning" | "ready">("idle");
const scanMessage = ref("");
const scanFound = ref(0);
const scanIndexed = ref(0);
const scanErrors = ref(0);
const photos = ref<Photo[]>([]);
const totalCount = ref(0);
const offset = ref(0);
const viewerIndex = ref<number | null>(null);
const PAGE_SIZE = 60;

let unlistenScan: (() => void) | null = null;

onMounted(async () => {
  unlistenScan = await listen<{
    found: number;
    indexed: number;
    errors: number;
  }>("scan-progress", (event) => {
    scanFound.value = event.payload.found;
    scanIndexed.value = event.payload.indexed;
    scanErrors.value = event.payload.errors;
  });

  try {
    const count = await getPhotoCount();
    if (count > 0) {
      totalCount.value = count;
      photos.value = await loadPhotos(0, PAGE_SIZE);
      offset.value = photos.value.length;
      status.value = "ready";
    }
  } catch {
    // No database yet
  }
});

onUnmounted(() => {
  unlistenScan?.();
});

function resetScanState() {
  scanFound.value = 0;
  scanIndexed.value = 0;
  scanErrors.value = 0;
}

async function afterScan(result: ScanResult) {
  scanMessage.value = result.message;
  totalCount.value = await getPhotoCount();
  photos.value = await loadPhotos(0, PAGE_SIZE);
  offset.value = photos.value.length;
  status.value = "ready";
}

async function handleSelectDirectory() {
  status.value = "scanning";
  scanMessage.value = "正在扫描照片…";
  resetScanState();
  try {
    const result = await selectAndScanDirectory();
    await afterScan(result);
  } catch (err) {
    console.error("Scan error:", err);
    scanMessage.value = "扫描出错，请重试。";
    status.value = "idle";
  }
}

async function handleRescan() {
  status.value = "scanning";
  scanMessage.value = "正在重新扫描照片…";
  resetScanState();
  try {
    const result = await rescanDirectory();
    await afterScan(result);
  } catch (err) {
    console.error("Rescan error:", err);
    scanMessage.value = "重新扫描出错，请重试。";
    status.value = "ready";
  }
}

async function handleLoadMore(): Promise<boolean> {
  if (offset.value >= totalCount.value) return false;
  const more = await loadPhotos(offset.value, PAGE_SIZE);
  photos.value = [...photos.value, ...more];
  offset.value += more.length;
  return more.length > 0;
}

function handlePhotoClick(index: number) {
  viewerIndex.value = index;
}

function handleViewerClose() {
  viewerIndex.value = null;
}

function handleViewerPrev() {
  if (viewerIndex.value !== null && viewerIndex.value > 0) {
    viewerIndex.value--;
  }
}

function handleViewerNext() {
  if (viewerIndex.value !== null && viewerIndex.value < photos.value.length - 1) {
    viewerIndex.value++;
  }
}
</script>

<template>
  <div class="app">
    <!-- Welcome screen -->
    <div v-if="status === 'idle' || status === 'scanning'" class="welcome">
      <div class="welcome-content">
        <h1>PhotoFlow</h1>
        <p class="subtitle">管理你的本地照片收藏</p>
        <button
          v-if="status === 'idle'"
          class="btn-select"
          @click="handleSelectDirectory"
        >
          选择照片目录
        </button>
        <div v-else class="scanning-info">
          <p class="scanning-msg">{{ scanMessage }}</p>
          <div v-if="scanFound > 0" class="scan-stats">
            <span>已发现: {{ scanFound.toLocaleString() }}</span>
            <span>已索引: {{ scanIndexed.toLocaleString() }}</span>
            <span v-if="scanErrors > 0" class="stat-error">
              错误: {{ scanErrors.toLocaleString() }}
            </span>
          </div>
          <div class="progress-bar">
            <div class="progress-bar-inner" />
          </div>
        </div>
      </div>
    </div>

    <!-- Photo gallery -->
    <PhotoGallery
      v-else
      :photos="photos"
      :total-count="totalCount"
      :has-more="offset < totalCount"
      @load-more="handleLoadMore"
      @photo-click="handlePhotoClick"
      @rescan="handleRescan"
    />

    <!-- Full-screen viewer -->
    <PhotoViewer
      v-if="viewerIndex !== null"
      :photo="photos[viewerIndex]"
      :has-prev="viewerIndex > 0"
      :has-next="viewerIndex < photos.length - 1"
      :current="viewerIndex + 1"
      :total="photos.length"
      @close="handleViewerClose"
      @prev="handleViewerPrev"
      @next="handleViewerNext"
    />
  </div>
</template>

<style scoped>
.app {
  width: 100%;
  height: 100%;
  background: #1a1a2e;
}

.welcome {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
}

.welcome-content {
  text-align: center;
}

.welcome-content h1 {
  font-size: 3rem;
  font-weight: 300;
  letter-spacing: 0.2em;
  margin-bottom: 0.5rem;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.subtitle {
  color: #888;
  margin-bottom: 2rem;
  font-size: 1.1rem;
}

.btn-select {
  padding: 14px 36px;
  font-size: 1rem;
  border: none;
  border-radius: 8px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  cursor: pointer;
  transition: opacity 0.2s;
}

.btn-select:hover {
  opacity: 0.85;
}

.scanning-info {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.scanning-msg {
  color: #aaa;
  font-size: 1rem;
  animation: pulse 1.5s ease-in-out infinite;
}

.scan-stats {
  display: flex;
  gap: 24px;
  color: #888;
  font-size: 0.9rem;
}

.stat-error {
  color: #e57373;
}

.progress-bar {
  width: 240px;
  height: 3px;
  background: #2a2a4a;
  border-radius: 2px;
  overflow: hidden;
}

.progress-bar-inner {
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, #667eea, #764ba2);
  animation: progress-slide 2s ease-in-out infinite;
  transform-origin: left;
}

@keyframes progress-slide {
  0% { transform: translateX(-100%); }
  50% { transform: translateX(0%); }
  100% { transform: translateX(100%); }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
</style>
