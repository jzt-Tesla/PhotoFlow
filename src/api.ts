import { invoke } from "@tauri-apps/api/core";
import type { Photo, ScanResult } from "./types";

/**
 * Let user pick a directory, then scan and index all photos.
 */
export async function selectAndScanDirectory(): Promise<ScanResult> {
  return await invoke<ScanResult>("select_and_scan_directory");
}

/**
 * Re-scan the previously selected directory.
 */
export async function rescanDirectory(): Promise<ScanResult> {
  return await invoke<ScanResult>("rescan_directory");
}

/**
 * Load a page of photos from the database.
 */
export async function loadPhotos(offset: number, limit: number): Promise<Photo[]> {
  return await invoke<Photo[]>("load_photos", { offset, limit });
}

/**
 * Get the total number of indexed photos.
 */
export async function getPhotoCount(): Promise<number> {
  return await invoke<number>("get_photo_count");
}

/**
 * Build a URL for a thumbnail using the custom protocol.
 * Uses the http://photoflow.localhost/ workaround format required by
 * WebView2 on Windows (wry converts this back to photoflow:// internally).
 */
export function getThumbnailUrl(thumbPath: string): string {
  // thumbPath is the hash filename like ".../thumbs/{hash}.jpg"
  // Extract just the hash (filename without extension)
  const filename = thumbPath.split(/[\\/]/).pop() ?? "";
  const hash = filename.replace(/\.jpg$/i, "");
  return `http://photoflow.localhost/thumb/${hash}`;
}

/**
 * Build a URL for the original photo using the custom protocol.
 * Uses the http://photoflow.localhost/ workaround format required by
 * WebView2 on Windows (wry converts this back to photoflow:// internally).
 */
export function getPhotoUrl(photoPath: string): string {
  // Encode the file path as URL-safe base64
  const encoder = new TextEncoder();
  const bytes = encoder.encode(photoPath);
  // Use Array.from to avoid call-stack overflow with very long paths
  const binary = Array.from(bytes, (b) => String.fromCharCode(b)).join("");
  const base64 = btoa(binary)
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");
  return `http://photoflow.localhost/photo/${base64}`;
}
