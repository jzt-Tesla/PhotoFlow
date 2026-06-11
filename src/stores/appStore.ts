import { defineStore } from "pinia";
import type { Tag, ScanDirectory, ViewMode, PhotoFilter } from "../types";
import {
  listTags,
  listScanDirectories,
  selectAndScanDirectory,
  rescanAllDirectories,
  pickAndAddDirectory,
  repairThumbnails,
} from "../api";
import { usePhotoStore } from "./photoStore";

interface ScanProgress {
  found: number;
  indexed: number;
  errors: number;
}

interface AppStoreState {
  // 应用生命周期
  status: "idle" | "scanning" | "ready";
  scanProgress: ScanProgress;

  // 元数据
  tags: Tag[];
  directories: ScanDirectory[];

  // 导航状态
  view: ViewMode;
  searchQuery: string;
  selectedTagId: number | null;
  selectedDirPath: string | null;
  selectedYearMonth: string | null;

  // UI 状态
  showTagManager: boolean;
}

export const useAppStore = defineStore("app", {
  state: (): AppStoreState => ({
    status: "idle",
    scanProgress: { found: 0, indexed: 0, errors: 0 },
    tags: [],
    directories: [],
    view: "timeline",
    searchQuery: "",
    selectedTagId: null,
    selectedDirPath: null,
    selectedYearMonth: null,
    showTagManager: false,
  }),

  getters: {
    /** 当前 filter 对象（由 view + 参数决定） */
    currentFilter(state): PhotoFilter | null {
      switch (state.view) {
        case "favorites":
          return { favoriteOnly: true };
        case "tag-photos":
          return state.selectedTagId ? { tagId: state.selectedTagId } : null;
        case "directory":
          return state.selectedDirPath
            ? { directoryPath: state.selectedDirPath }
            : null;
        case "search":
          return state.searchQuery
            ? { searchQuery: state.searchQuery }
            : null;
        case "timeline":
          return null;
        default:
          return null;
      }
    },
  },

  actions: {
    /** 统一导航入口：设置参数 → 显式调用 photoStore 加载 */
    navigateTo(
      view: ViewMode,
      params?: { tagId?: number; dirPath?: string; searchQuery?: string }
    ) {
      this.view = view;
      this.selectedTagId = params?.tagId ?? null;
      this.selectedDirPath = params?.dirPath ?? null;
      this.searchQuery = params?.searchQuery ?? "";
      // 显式触发加载（非 watch 隐式）
      const photoStore = usePhotoStore();
      photoStore.loadPhotos(this.currentFilter);
    },

    /** 首次选择目录并扫描 */
    async startScan() {
      this.status = "scanning";
      this.scanProgress = { found: 0, indexed: 0, errors: 0 };
      try {
        await selectAndScanDirectory();
      } finally {
        await this.afterScan();
      }
    },

    /** 重新扫描所有目录 */
    async rescanAll() {
      this.status = "scanning";
      this.scanProgress = { found: 0, indexed: 0, errors: 0 };
      try {
        await rescanAllDirectories();
      } finally {
        await this.afterScan();
      }
    },

    /** 从侧边栏添加新目录：选择文件夹 → 注册 → 扫描 */
    async addDirectory() {
      try {
        this.status = "scanning";
        this.scanProgress = { found: 0, indexed: 0, errors: 0 };
        await pickAndAddDirectory();
        await this.rescanAll();
      } catch {
        // User cancelled picker — restore status
        this.status = "ready";
      }
    },

    /** 扫描完成后的通用处理 */
    async afterScan() {
      await repairThumbnails(); // 修复缺失的缩略图
      const photoStore = usePhotoStore();
      await photoStore.reload();
      await this.loadMetadata();
      this.status = "ready";
    },

    /** 刷新 tags + directories */
    async loadMetadata() {
      try {
        this.tags = await listTags();
        this.directories = await listScanDirectories();
      } catch {
        /* ignore */
      }
    },
  },
});
