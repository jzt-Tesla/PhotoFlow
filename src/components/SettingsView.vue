<script setup lang="ts">
import { ref, onMounted } from "vue";
import {
  getSettings,
  updateSettings,
  clearThumbnailCache,
  getAppInfo,
  removeScanDirectory,
  listScanDirectories,
} from "../api";
import { useAppStore } from "../stores/appStore";
import type { AppSettings, AppInfo, ScanDirectory } from "../types";

const appStore = useAppStore();

const settings = ref<AppSettings>({ thumbnail_size: 200, thumbnail_quality: 80 });
const info = ref<AppInfo | null>(null);
const directories = ref<ScanDirectory[]>([]);
const message = ref("");

onMounted(async () => {
  try {
    settings.value = await getSettings();
    info.value = await getAppInfo();
    directories.value = await listScanDirectories();
  } catch (e) {
    console.error("Failed to load settings:", e);
  }
});

async function saveSettings() {
  try {
    await updateSettings(settings.value);
    message.value = "设置已保存";
    setTimeout(() => (message.value = ""), 2000);
  } catch (e) {
    message.value = "保存失败";
  }
}

async function handleClearCache() {
  try {
    const removed = await clearThumbnailCache();
    message.value = `已清理 ${removed} 个孤立缩略图`;
    info.value = await getAppInfo();
    setTimeout(() => (message.value = ""), 3000);
  } catch (e) {
    message.value = "清理失败";
  }
}

async function handleRemoveDir(dirId: number) {
  try {
    await removeScanDirectory(dirId);
    directories.value = await listScanDirectories();
    info.value = await getAppInfo();
    await appStore.loadMetadata();
  } catch (e) {
    message.value = "删除失败";
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  return (bytes / (1024 * 1024 * 1024)).toFixed(1) + " GB";
}
</script>

<template>
  <div class="settings">
    <h2 class="settings-title">设置</h2>

    <div class="settings-section">
      <h3>缩略图</h3>
      <div class="setting-row">
        <label>尺寸</label>
        <select v-model.number="settings.thumbnail_size" @change="saveSettings">
          <option :value="100">100px</option>
          <option :value="150">150px</option>
          <option :value="200">200px</option>
          <option :value="250">250px</option>
          <option :value="300">300px</option>
        </select>
      </div>
      <div class="setting-row">
        <label>质量</label>
        <select v-model.number="settings.thumbnail_quality" @change="saveSettings">
          <option :value="60">60%</option>
          <option :value="70">70%</option>
          <option :value="80">80%</option>
          <option :value="90">90%</option>
          <option :value="100">100%</option>
        </select>
      </div>
    </div>

    <div class="settings-section">
      <h3>照片目录</h3>
      <div class="dir-list">
        <div v-for="dir in directories" :key="dir.id" class="dir-row">
          <span class="dir-path">{{ dir.path }}</span>
          <button class="btn-remove" @click="handleRemoveDir(dir.id)" title="移除">×</button>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <h3>缓存</h3>
      <button class="btn-action" @click="handleClearCache">清理孤立缩略图</button>
    </div>

    <div class="settings-section" v-if="info">
      <h3>应用信息</h3>
      <div class="info-row"><span>照片数量</span><span>{{ info.photo_count.toLocaleString() }}</span></div>
      <div class="info-row"><span>缩略图</span><span>{{ info.thumbnail_count.toLocaleString() }}</span></div>
      <div class="info-row"><span>数据库大小</span><span>{{ formatSize(info.db_size) }}</span></div>
      <div class="info-row"><span>数据目录</span><span class="info-path">{{ info.data_dir }}</span></div>
    </div>

    <div v-if="message" class="message">{{ message }}</div>
  </div>
</template>

<style scoped>
.settings {
  padding: 24px;
  overflow-y: auto;
  height: 100%;
  max-width: 600px;
}

.settings-title {
  font-size: 1.2rem;
  font-weight: 500;
  margin-bottom: 24px;
  color: #e0e0e0;
}

.settings-section {
  margin-bottom: 24px;
}

.settings-section h3 {
  font-size: 0.85rem;
  color: #667eea;
  margin-bottom: 12px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
}

.setting-row label {
  color: #aaa;
  font-size: 0.85rem;
}

.setting-row select {
  background: #1a1a2e;
  border: 1px solid #333;
  border-radius: 4px;
  color: #e0e0e0;
  padding: 6px 10px;
  font-size: 0.85rem;
}

.dir-list {
  margin-bottom: 8px;
}

.dir-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px;
  background: #1a1a2e;
  border-radius: 4px;
  margin-bottom: 4px;
}

.dir-path {
  color: #aaa;
  font-size: 0.8rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  margin-right: 8px;
}

.btn-remove {
  background: none;
  border: none;
  color: #e57373;
  cursor: pointer;
  font-size: 1.1rem;
  padding: 0 4px;
}

.btn-action {
  background: #1a1a2e;
  border: 1px solid #333;
  border-radius: 6px;
  color: #888;
  padding: 8px 16px;
  font-size: 0.85rem;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}

.btn-action:hover {
  color: #667eea;
  border-color: #667eea;
}

.info-row {
  display: flex;
  justify-content: space-between;
  padding: 6px 0;
  font-size: 0.85rem;
}

.info-row span:first-child {
  color: #888;
}

.info-row span:last-child {
  color: #bbb;
}

.info-path {
  font-size: 0.75rem;
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.message {
  margin-top: 16px;
  padding: 8px 12px;
  background: rgba(102, 126, 234, 0.15);
  border-radius: 6px;
  color: #667eea;
  font-size: 0.85rem;
}
</style>
