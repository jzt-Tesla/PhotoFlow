<script setup lang="ts">
import type { Tag, ScanDirectory, ViewMode } from "../types";

defineProps<{
  currentView: ViewMode;
  tags: Tag[];
  directories: ScanDirectory[];
  totalCount: number;
  selectedTagId: number | null;
  selectedDirPath: string | null;
}>();

const emit = defineEmits<{
  "view-change": [view: ViewMode];
  "tag-select": [tagId: number];
  "dir-select": [dirPath: string];
  "manage-tags": [];
  "add-directory": [];
}>();

const navItems: { view: ViewMode; label: string; icon: string }[] = [
  { view: "favorites", label: "收藏", icon: "⭐" },
  { view: "tags", label: "标签", icon: "🏷️" },
  { view: "settings", label: "设置", icon: "⚙️" },
];

function dirName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <h1 class="logo">PhotoFlow</h1>
      <span class="count">{{ totalCount.toLocaleString() }}</span>
    </div>

    <nav class="nav-section">
      <!-- 所有照片 + 添加目录按钮 -->
      <div class="nav-row">
        <button
          class="nav-item nav-item-main"
          :class="{ active: currentView === 'timeline' || currentView === 'directory' }"
          @click="emit('view-change', 'timeline')"
        >
          <span class="nav-icon">📷</span>
          <span class="nav-label">所有照片</span>
        </button>
        <button
          class="btn-inline-add"
          @click.stop="emit('add-directory')"
          title="添加照片目录"
        >
          ＋
        </button>
      </div>

      <!-- 目录树（常驻显示） -->
      <div v-if="directories.length > 0" class="dir-tree">
        <button
          v-for="dir in directories"
          :key="dir.id"
          class="nav-item dir-item"
          :class="{ active: currentView === 'directory' && selectedDirPath === dir.path }"
          @click="emit('dir-select', dir.path)"
        >
          <span class="dir-icon">📁</span>
          <span class="nav-label dir-name">{{ dirName(dir.path) }}</span>
        </button>
      </div>

      <button
        v-for="item in navItems"
        :key="item.view"
        class="nav-item"
        :class="{ active: currentView === item.view }"
        @click="emit('view-change', item.view)"
      >
        <span class="nav-icon">{{ item.icon }}</span>
        <span class="nav-label">{{ item.label }}</span>
      </button>
    </nav>

    <!-- Tags section -->
    <div class="nav-section" v-if="tags.length > 0">
      <div class="section-header">
        <span class="section-title">标签</span>
        <button class="btn-manage" @click="emit('manage-tags')" title="管理标签">⚙</button>
      </div>
      <button
        v-for="tag in tags"
        :key="tag.id"
        class="nav-item tag-item"
        :class="{ active: currentView === 'tag-photos' && selectedTagId === tag.id }"
        @click="emit('tag-select', tag.id)"
      >
        <span class="tag-dot" :style="{ background: tag.color }" />
        <span class="nav-label">{{ tag.name }} <span class="tag-count">{{ tag.count }}</span></span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 220px;
  background: #0f0f23;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow-y: auto;
  border-right: 1px solid #1a1a2e;
}

.sidebar-header {
  padding: 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.logo {
  font-size: 1.1rem;
  font-weight: 500;
  background: linear-gradient(135deg, #667eea, #764ba2);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.count {
  color: #555;
  font-size: 0.75rem;
}

.nav-section {
  padding: 4px 8px;
}

.nav-row {
  display: flex;
  align-items: center;
}

.nav-item-main {
  flex: 1;
}

.btn-inline-add {
  background: none;
  border: none;
  color: #555;
  font-size: 1rem;
  cursor: pointer;
  padding: 6px 8px;
  border-radius: 4px;
  transition: color 0.15s, background 0.15s;
}

.btn-inline-add:hover {
  color: #667eea;
  background: rgba(102, 126, 234, 0.08);
}

.dir-tree {
  padding-left: 12px;
  border-left: 2px solid #1a1a2e;
  margin: 2px 0 2px 8px;
}

.dir-item {
  padding: 5px 10px;
  font-size: 0.8rem;
}

.dir-icon {
  font-size: 0.85rem;
  flex-shrink: 0;
}

.dir-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 8px 4px;
}

.section-title {
  color: #555;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.btn-manage {
  background: none;
  border: none;
  color: #555;
  cursor: pointer;
  font-size: 0.75rem;
  padding: 2px;
}

.btn-manage:hover {
  color: #667eea;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 12px;
  background: none;
  border: none;
  border-radius: 6px;
  color: #888;
  font-size: 0.85rem;
  cursor: pointer;
  text-align: left;
  transition: background 0.15s, color 0.15s;
}

.nav-item:hover {
  background: rgba(102, 126, 234, 0.08);
  color: #bbb;
}

.nav-item.active {
  background: rgba(102, 126, 234, 0.15);
  color: #667eea;
}

.nav-icon {
  font-size: 1rem;
  width: 20px;
  text-align: center;
  flex-shrink: 0;
}

.tag-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.tag-count {
  color: #555;
  font-size: 0.7rem;
}
</style>
