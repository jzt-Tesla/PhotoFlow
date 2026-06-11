<script setup lang="ts">
import { onMounted } from "vue";
import { storeToRefs } from "pinia";
import type { ViewMode } from "./types";
import { usePhotoStore } from "./stores/photoStore";
import { useAppStore } from "./stores/appStore";
import { useTauriEvents } from "./composables/useTauriEvents";
import Sidebar from "./components/Sidebar.vue";
import PhotoGallery from "./components/PhotoGallery.vue";
import PhotoViewer from "./components/PhotoViewer.vue";
import TimelineView from "./components/TimelineView.vue";
import SettingsView from "./components/SettingsView.vue";
import TagManager from "./components/TagManager.vue";
import SearchBar from "./components/SearchBar.vue";

const photoStore = usePhotoStore();
const appStore = useAppStore();
const { photos, viewerIndex } = storeToRefs(photoStore);
const { status, tags, directories, showTagManager, scanProgress } =
  storeToRefs(appStore);

useTauriEvents();

onMounted(async () => {
  await appStore.loadMetadata();
  if (appStore.directories.length === 0) {
    // 无目录 → 欢迎页，不加载旧数据
    appStore.status = "idle";
    return;
  }
  await photoStore.loadPhotos(null);
  if (photoStore.totalCount > 0) {
    appStore.status = "ready";
  }
  appStore.rescanAll();
});

function handleSelectDirectory() {
  appStore.startScan();
}

function handleRescanAll() {
  appStore.rescanAll();
}

function handleViewChange(view: ViewMode) {
  appStore.navigateTo(view);
}

function handleTagSelect(tagId: number) {
  appStore.navigateTo("tag-photos", { tagId });
}

function handleDirSelect(dirPath: string) {
  appStore.navigateTo("directory", { dirPath });
}

function handleSearch(query: string) {
  appStore.navigateTo(query ? "search" : "timeline", { searchQuery: query });
}

function handleAddDirectory() {
  appStore.addDirectory();
}

function handlePhotoClick(index: number) {
  photoStore.openViewer(index);
}
</script>

<template>
  <div class="app-layout">
    <!-- Welcome screen -->
    <div
      v-if="status === 'idle' && photos.length === 0 && directories.length === 0"
      class="welcome"
    >
      <div class="welcome-content">
        <h1>PhotoFlow</h1>
        <p class="subtitle">管理你的本地照片收藏</p>
        <button class="btn-select" @click="handleSelectDirectory">
          选择照片目录
        </button>
      </div>
    </div>

    <!-- Main app layout -->
    <template v-else>
      <Sidebar
        :current-view="appStore.view"
        :tags="tags"
        :directories="directories"
        :total-count="photoStore.totalCount"
        :selected-tag-id="appStore.selectedTagId"
        :selected-dir-path="appStore.selectedDirPath"
        @view-change="handleViewChange"
        @tag-select="handleTagSelect"
        @dir-select="handleDirSelect"
        @manage-tags="showTagManager = true"
        @add-directory="handleAddDirectory"
      />

      <div class="main-area">
        <div class="top-bar">
          <SearchBar :model-value="appStore.searchQuery" @search="handleSearch" />
          <div v-if="status === 'scanning'" class="scan-indicator">
            <span class="scan-dot" />
            <span>
              扫描中
              {{ scanProgress.found > 0 ? `${scanProgress.indexed}/${scanProgress.found}` : "" }}
            </span>
          </div>
          <div class="top-actions">
            <button class="btn-icon" @click="handleRescanAll" title="重新扫描">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21.5 2v6h-6M2.5 22v-6h6" />
                <path d="M2.5 11.5a10 10 0 0 1 18.8-4.3M21.5 12.5a10 10 0 0 1-18.8 4.3" />
              </svg>
            </button>
          </div>
        </div>

        <div class="content-area">
          <TimelineView v-if="appStore.view === 'timeline'" @photo-click="handlePhotoClick" />

          <PhotoGallery
            v-else-if="appStore.view === 'favorites'"
            key="favorites"
            @photo-click="handlePhotoClick"
          />

          <PhotoGallery
            v-else-if="appStore.view === 'tag-photos'"
            key="tag-photos"
            @photo-click="handlePhotoClick"
          />

          <PhotoGallery
            v-else-if="appStore.view === 'directory'"
            key="directory"
            @photo-click="handlePhotoClick"
          />

          <PhotoGallery
            v-else-if="appStore.view === 'search'"
            key="search"
            @photo-click="handlePhotoClick"
          />

          <SettingsView v-else-if="appStore.view === 'settings'" />

          <div v-else class="empty-state">
            <p>选择左侧目录浏览照片</p>
          </div>
        </div>
      </div>

      <!-- Full-screen viewer -->
      <PhotoViewer
        v-if="viewerIndex !== null && photos.length > 0"
        :photo="photos[viewerIndex]"
        :has-prev="viewerIndex > 0"
        :has-next="viewerIndex < photos.length - 1"
        :current="viewerIndex + 1"
        :total="photos.length"
        @close="photoStore.closeViewer()"
        @prev="photoStore.navigateViewerPrev()"
        @next="photoStore.navigateViewerNext()"
      />

      <!-- Tag manager dialog -->
      <TagManager
        v-if="showTagManager"
        :tags="tags"
        @close="showTagManager = false"
        @tags-changed="appStore.tags = $event"
      />
    </template>
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background: #1a1a2e;
  color: #e0e0e0;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  overflow: hidden;
}

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: #333;
  border-radius: 3px;
}
</style>

<style scoped>
.app-layout {
  display: flex;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
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

.main-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.top-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background: #16213e;
  border-bottom: 1px solid #1a1a2e;
  flex-shrink: 0;
  gap: 12px;
}

.top-actions {
  display: flex;
  gap: 8px;
}

.btn-icon {
  background: none;
  border: 1px solid #333;
  border-radius: 6px;
  color: #888;
  padding: 6px 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.15s, border-color 0.15s;
}

.btn-icon:hover {
  color: #667eea;
  border-color: #667eea;
}

.scan-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #667eea;
  font-size: 0.8rem;
}

.scan-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #667eea;
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.content-area {
  flex: 1;
  overflow: hidden;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #555;
  font-size: 0.9rem;
}
</style>
