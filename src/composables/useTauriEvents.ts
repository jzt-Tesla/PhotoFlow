import { onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { Photo } from "../types";
import { usePhotoStore } from "../stores/photoStore";
import { useAppStore } from "../stores/appStore";

/**
 * Encapsulates Tauri event listeners for scan-progress and photo-stream.
 * Manages micro-batch buffering (5 photos / 50ms) and writes to stores.
 */
export function useTauriEvents() {
  const photoStore = usePhotoStore();
  const appStore = useAppStore();

  let unlistenScan: (() => void) | null = null;
  let unlistenBatch: (() => void) | null = null;

  // Micro-batch buffer for streaming photos
  let streamBuffer: Photo[] = [];
  let streamTimer: ReturnType<typeof setTimeout> | null = null;

  function flushStreamBuffer() {
    if (streamBuffer.length === 0) return;
    const batch = streamBuffer.splice(0);
    if (appStore.view === "timeline") {
      photoStore.appendStreamed(batch);
    }
    // Show gallery on first photo arrival
    if (appStore.status === "scanning") {
      appStore.status = "ready";
    }
  }

  onMounted(async () => {
    unlistenScan = await listen<{
      found: number;
      indexed: number;
      errors: number;
    }>("scan-progress", (event) => {
      appStore.scanProgress = event.payload;
    });

    unlistenBatch = await listen<Photo>("photo-stream", (event) => {
      streamBuffer.push(event.payload);
      // Flush every 5 photos or every 50ms, whichever comes first
      if (streamBuffer.length >= 5) {
        if (streamTimer) {
          clearTimeout(streamTimer);
          streamTimer = null;
        }
        flushStreamBuffer();
      } else if (!streamTimer) {
        streamTimer = setTimeout(() => {
          streamTimer = null;
          flushStreamBuffer();
        }, 50);
      }
    });
  });

  onUnmounted(() => {
    unlistenScan?.();
    unlistenBatch?.();
    if (streamTimer) {
      clearTimeout(streamTimer);
      streamTimer = null;
    }
    flushStreamBuffer();
  });
}
