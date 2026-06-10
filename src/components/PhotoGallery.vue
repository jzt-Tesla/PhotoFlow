<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import type { Photo } from "../types";
import ThumbnailCard from "./ThumbnailCard.vue";

const props = defineProps<{
  photos: Photo[];
  totalCount: number;
  hasMore: boolean;
}>();

const emit = defineEmits<{
  "load-more": [];
  "photo-click": [index: number];
  rescan: [];
}>();

const containerRef = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const containerHeight = ref(0);

const GAP = 16;
const CARD_SIZE = 200;
const ITEM_SIZE = CARD_SIZE + GAP; // 216px per row
const columns = ref(5);

function updateColumns() {
  if (!containerRef.value) return;
  const width = containerRef.value.clientWidth - GAP * 2; // padding
  columns.value = Math.max(2, Math.floor((width + GAP) / ITEM_SIZE));
}

const totalRows = computed(() => Math.ceil(props.photos.length / columns.value));
const totalHeight = computed(() => totalRows.value * ITEM_SIZE + GAP);

const startRow = computed(() => Math.max(0, Math.floor(scrollTop.value / ITEM_SIZE) - 2));
const visibleRowCount = computed(() => {
  const rows = Math.ceil(containerHeight.value / ITEM_SIZE) + 4;
  return Math.min(rows, totalRows.value - startRow.value);
});

const visiblePhotos = computed(() => {
  const result: Array<{ photo: Photo; index: number; x: number; y: number }> = [];
  const startIdx = startRow.value * columns.value;
  const endIdx = Math.min(startIdx + visibleRowCount.value * columns.value, props.photos.length);

  for (let i = startIdx; i < endIdx; i++) {
    const localIdx = i - startIdx;
    const col = localIdx % columns.value;
    const row = Math.floor(localIdx / columns.value);
    result.push({
      photo: props.photos[i],
      index: i,
      x: GAP + col * ITEM_SIZE,
      y: (startRow.value + row) * ITEM_SIZE + GAP,
    });
  }
  return result;
});

function onScroll() {
  if (!containerRef.value) return;
  scrollTop.value = containerRef.value.scrollTop;

  const { scrollTop: st, scrollHeight, clientHeight } = containerRef.value;
  if (st + clientHeight >= scrollHeight - ITEM_SIZE * 2 && props.hasMore) {
    emit("load-more");
  }
}

let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  if (containerRef.value) {
    containerHeight.value = containerRef.value.clientHeight;
    updateColumns();
    resizeObserver = new ResizeObserver(() => {
      if (containerRef.value) {
        containerHeight.value = containerRef.value.clientHeight;
        updateColumns();
      }
    });
    resizeObserver.observe(containerRef.value);
  }
});

onUnmounted(() => {
  resizeObserver?.disconnect();
});
</script>

<template>
  <div class="gallery">
    <header class="gallery-header">
      <h2>PhotoFlow</h2>
      <div class="header-right">
        <span class="count">{{ totalCount.toLocaleString() }} 张照片</span>
        <button class="btn-rescan" @click="emit('rescan')" title="重新扫描目录">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21.5 2v6h-6M2.5 22v-6h6" />
            <path d="M2.5 11.5a10 10 0 0 1 18.8-4.3M21.5 12.5a10 10 0 0 1-18.8 4.3" />
          </svg>
        </button>
      </div>
    </header>
    <div ref="containerRef" class="gallery-scroll" @scroll="onScroll">
      <div class="gallery-spacer" :style="{ height: totalHeight + 'px' }">
        <div
          v-for="item in visiblePhotos"
          :key="item.photo.id"
          class="gallery-item"
          :style="{
            position: 'absolute',
            left: item.x + 'px',
            top: item.y + 'px',
            width: CARD_SIZE + 'px',
            height: CARD_SIZE + 'px',
          }"
        >
          <ThumbnailCard
            :photo="item.photo"
            @click="emit('photo-click', item.index)"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.gallery {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
}

.gallery-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 24px;
  background: #16213e;
  border-bottom: 1px solid #1a1a2e;
  flex-shrink: 0;
}

.gallery-header h2 {
  font-size: 1.2rem;
  font-weight: 500;
  background: linear-gradient(135deg, #667eea, #764ba2);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.count {
  color: #888;
  font-size: 0.85rem;
}

.btn-rescan {
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

.btn-rescan:hover {
  color: #667eea;
  border-color: #667eea;
}

.gallery-scroll {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.gallery-scroll::-webkit-scrollbar {
  width: 6px;
}

.gallery-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.gallery-scroll::-webkit-scrollbar-thumb {
  background: #333;
  border-radius: 3px;
}

.gallery-spacer {
  position: relative;
  width: 100%;
}

.gallery-item {
  /* Positioned absolutely by computed style */
}
</style>
