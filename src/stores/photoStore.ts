import { defineStore } from "pinia";
import type { Photo, PhotoFilter } from "../types";
import {
  loadPhotosFiltered,
  photoCountFiltered,
  loadPhotos,
  getPhotoCount,
} from "../api";

const PAGE_SIZE = 200;

interface PhotoStoreState {
  // 照片数据
  photos: Photo[];
  totalCount: number;
  hasMore: boolean;
  offset: number;
  loading: boolean;
  currentFilter: PhotoFilter | null;
  // 查看器（语义上属于"当前照片上下文"）
  viewerIndex: number | null;
}

export const usePhotoStore = defineStore("photo", {
  state: (): PhotoStoreState => ({
    photos: [],
    totalCount: 0,
    hasMore: false,
    offset: 0,
    loading: false,
    currentFilter: null,
    viewerIndex: null,
  }),

  actions: {
    /** 切换视图/目录时调用：异步加载新数据，旧照片保留到新数据到位 */
    async loadPhotos(filter: PhotoFilter | null) {
      this.currentFilter = filter;
      this.loading = true;
      try {
        const count = filter
          ? await photoCountFiltered(filter)
          : await getPhotoCount();
        const limit = Math.min(PAGE_SIZE, count);
        const loaded = filter
          ? await loadPhotosFiltered({ ...filter, offset: 0, limit })
          : await loadPhotos(0, limit);
        // 一次性替换：新数据到位后才替换旧数据，避免空白闪烁
        this.$patch({
          photos: loaded,
          totalCount: count,
          offset: loaded.length,
          hasMore: loaded.length < count,
          viewerIndex: null,
        });
      } catch (e) {
        console.error("[photoStore] loadPhotos error:", e);
        // 出错时清空，避免显示过期数据
        this.$patch({ photos: [], totalCount: 0, offset: 0, hasMore: false });
      } finally {
        this.loading = false;
      }
    },

    /** 无限滚动追加 */
    async loadMore() {
      if (this.loading || !this.hasMore) return;
      this.loading = true;
      try {
        const more = this.currentFilter
          ? await loadPhotosFiltered({
              ...this.currentFilter,
              offset: this.offset,
              limit: PAGE_SIZE,
            })
          : await loadPhotos(this.offset, PAGE_SIZE);
        this.$patch((state) => {
          state.photos.push(...more);
        });
        this.offset += more.length;
        this.hasMore = this.offset < this.totalCount;
      } finally {
        this.loading = false;
      }
    },

    /** photo-stream 流式追加（$patch 避免数组重建） */
    appendStreamed(batch: Photo[]) {
      this.$patch((state) => {
        state.photos.push(...batch);
      });
      this.totalCount = this.photos.length;
      this.offset = this.photos.length;
    },

    /** 就地更新收藏状态 */
    updateFavorite(photoId: number, isFav: boolean) {
      const photo = this.photos.find((p) => p.id === photoId);
      if (photo) photo.favorite = isFav;
    },

    /** 扫描结束后从 DB 重新加载（获得真实 ID） */
    async reload() {
      await this.loadPhotos(this.currentFilter);
    },

    /** 清空所有状态（包括查看器） */
    reset() {
      this.photos = [];
      this.totalCount = 0;
      this.hasMore = false;
      this.offset = 0;
      this.currentFilter = null;
      this.viewerIndex = null;
    },

    // ── 查看器操作 ──
    openViewer(index: number) {
      this.viewerIndex = index;
    },
    closeViewer() {
      this.viewerIndex = null;
    },
    navigateViewerPrev() {
      if (this.viewerIndex !== null && this.viewerIndex > 0)
        this.viewerIndex--;
    },
    navigateViewerNext() {
      if (
        this.viewerIndex !== null &&
        this.viewerIndex < this.photos.length - 1
      )
        this.viewerIndex++;
    },
  },
});
