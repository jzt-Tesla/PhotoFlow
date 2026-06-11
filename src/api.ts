import { invoke } from "@tauri-apps/api/core";
import type {
  Photo,
  Tag,
  ScanDirectory,
  TimelineGroup,
  ScanResult,
  AppSettings,
  AppInfo,
} from "./types";

// ────────── Directory scanning ──────────

export async function selectAndScanDirectory(): Promise<ScanResult> {
  return await invoke<ScanResult>("select_and_scan_directory");
}

export async function rescanAllDirectories(): Promise<ScanResult> {
  return await invoke<ScanResult>("rescan_all_directories");
}

export async function rescanDirectoryById(dirId: number): Promise<ScanResult> {
  return await invoke<ScanResult>("rescan_directory_by_id", { dirId });
}

// ────────── Photo loading ──────────

export async function loadPhotos(
  offset: number,
  limit: number,
): Promise<Photo[]> {
  return await invoke<Photo[]>("load_photos", { offset, limit });
}

export async function getPhotoCount(): Promise<number> {
  return await invoke<number>("get_photo_count");
}

export async function loadPhotosFiltered(params: {
  favoriteOnly?: boolean;
  tagId?: number;
  directoryPath?: string;
  yearMonth?: string;
  searchQuery?: string;
  offset: number;
  limit: number;
}): Promise<Photo[]> {
  const { offset, limit, ...filter } = params;
  return await invoke<Photo[]>("load_photos_filtered", {
    filterJson: JSON.stringify(filter),
    offset,
    limit,
  });
}

export async function photoCountFiltered(params: {
  favoriteOnly?: boolean;
  tagId?: number;
  directoryPath?: string;
  yearMonth?: string;
  searchQuery?: string;
}): Promise<number> {
  return await invoke<number>("photo_count_filtered", {
    filterJson: JSON.stringify(params),
  });
}

// ────────── Favorites ──────────

export async function toggleFavorite(photoId: number): Promise<boolean> {
  return await invoke<boolean>("toggle_favorite", { photoId });
}

// ────────── Tags ──────────

export async function createTag(
  name: string,
  color: string,
): Promise<number> {
  return await invoke<number>("create_tag", { name, color });
}

export async function deleteTag(tagId: number): Promise<void> {
  await invoke("delete_tag", { tagId });
}

export async function renameTag(
  tagId: number,
  newName: string,
): Promise<void> {
  await invoke("rename_tag", { tagId, newName });
}

export async function updateTagColor(
  tagId: number,
  color: string,
): Promise<void> {
  await invoke("update_tag_color", { tagId, color });
}

export async function listTags(): Promise<Tag[]> {
  return await invoke<Tag[]>("list_tags");
}

export async function addPhotoTag(
  photoId: number,
  tagId: number,
): Promise<void> {
  await invoke("add_photo_tag", { photoId, tagId });
}

export async function removePhotoTag(
  photoId: number,
  tagId: number,
): Promise<void> {
  await invoke("remove_photo_tag", { photoId, tagId });
}

export async function getPhotoTags(photoId: number): Promise<Tag[]> {
  return await invoke<Tag[]>("get_photo_tags", { photoId });
}

// ────────── Timeline ──────────

export async function loadTimelineGroups(): Promise<TimelineGroup[]> {
  return await invoke<TimelineGroup[]>("load_timeline_groups");
}

// ────────── Multi-directory ──────────

export async function addScanDirectory(path: string): Promise<number> {
  return await invoke<number>("add_scan_directory", { path });
}

export async function removeScanDirectory(dirId: number): Promise<void> {
  await invoke("remove_scan_directory", { dirId });
}

export async function listScanDirectories(): Promise<ScanDirectory[]> {
  return await invoke<ScanDirectory[]>("list_scan_directories");
}

export async function pickAndAddDirectory(): Promise<number> {
  return await invoke<number>("pick_and_add_directory");
}

// ────────── Settings ──────────

export async function getSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>("get_settings");
}

export async function updateSettings(settings: AppSettings): Promise<void> {
  await invoke("update_settings", { settings });
}

export async function clearThumbnailCache(): Promise<number> {
  return await invoke<number>("clear_thumbnail_cache");
}

export async function getAppInfo(): Promise<AppInfo> {
  return await invoke<AppInfo>("get_app_info");
}

// ────────── URL builders ──────────

export function getThumbnailUrl(thumbPath: string): string {
  const filename = thumbPath.split(/[\\/]/).pop() ?? "";
  const hash = filename.replace(/\.jpg$/i, "");
  return `http://photoflow.localhost/thumb/${hash}`;
}

export function getPhotoUrl(photoPath: string): string {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(photoPath);
  const binary = Array.from(bytes, (b) => String.fromCharCode(b)).join("");
  const base64 = btoa(binary)
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");
  return `http://photoflow.localhost/photo/${base64}`;
}

export async function repairThumbnails(): Promise<number> {
  return await invoke<number>("repair_thumbnails");
}
