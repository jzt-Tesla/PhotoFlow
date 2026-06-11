<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { storeToRefs } from "pinia";
import type { Photo } from "../types";
import { usePhotoStore } from "../stores/photoStore";
import ThumbnailCard from "./ThumbnailCard.vue";

const emit = defineEmits<{
  "photo-click": [index: number];
}>();

const photoStore = usePhotoStore();
const { photos, loading } = storeToRefs(photoStore);

const containerRef = ref<HTMLElement | null>(null);
const scrollTop = ref(0);
const containerHeight = ref(0);

const GAP = 16;
const CARD_SIZE = 200;
const ITEM_SIZE = CARD_SIZE + GAP;
const columns = ref(5);

function updateColumns() {
  if (!containerRef.value) return;
  const width = containerRef.value.clientWidth - GAP * 2;
  columns.value = Math.max(2, Math.floor((width + GAP) / ITEM_SIZE));
}

const totalRows = computed(() =>
  Math.ceil(photos.value.length / columns.value)
);
const totalHeight = computed(() => totalRows.value * ITEM_SIZE + GAP);

const startRow = computed(() =>
  Math.max(0, Math.floor(scrollTop.value / ITEM_SIZE) - 2)
);
const visibleRowCount = computed(() => {
  const rows = Math.ceil(containerHeight.value / ITEM_SIZE) + 4;
  return Math.min(rows, totalRows.value - startRow.value);
});

const visiblePhotos = computed(() => {
  const result: Array<{ photo: Photo; index: number; x: number; y: number }> =
    [];
  const startIdx = startRow.value * columns.value;
  const endIdx = Math.min(
    startIdx + visibleRowCount.value * columns.value,
    photos.value.length
  );

  for (let i = startIdx; i < endIdx; i++) {
    const localIdx = i - startIdx;
    const col = localIdx % columns.value;
    const row = Math.floor(localIdx / columns.value);
    result.push({
      photo: photos.value[i],
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
  if (st + clientHeight >= scrollHeight - ITEM_SIZE * 2) {
    photoStore.loadMore();
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
      <div v-if="loading" class="loading-indicator">加载中…</div>
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

.gallery-scroll {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.gallery-spacer {
  position: relative;
  width: 100%;
}

.gallery-item {
  /* Positioned absolutely by computed style */
}

.loading-indicator {
  text-align: center;
  padding: 16px;
  color: #555;
  font-size: 0.8rem;
}
</style>
