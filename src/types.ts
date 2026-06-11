export interface Photo {
  id: number;
  path: string;
  filename: string;
  created_time: string;
  width: number;
  height: number;
  thumbnail_path: string;
  file_size: number;
  modified_time: string;
  taken_time: string | null;
  camera_model: string | null;
  lens_model: string | null;
  latitude: number | null;
  longitude: number | null;
  favorite: boolean;
  tags: string[];
}

export interface Tag {
  id: number;
  name: string;
  color: string;
  count: number;
}

export interface ScanDirectory {
  id: number;
  path: string;
  added_time: string;
}

export interface TimelineGroup {
  year_month: string;
  count: number;
}

export interface PhotoFilter {
  favoriteOnly?: boolean;
  tagId?: number;
  directoryPath?: string;
  yearMonth?: string;
  searchQuery?: string;
}

export interface ScanResult {
  message: string;
  found: number;
  indexed: number;
  errors: number;
  cleanup_removed: number;
}

export interface AppSettings {
  thumbnail_size: number;
  thumbnail_quality: number;
}

export interface AppInfo {
  photo_count: number;
  db_size: number;
  thumbnail_count: number;
  scan_directories: ScanDirectory[];
  data_dir: string;
}

export type ViewMode =
  | "timeline"
  | "favorites"
  | "tags"
  | "tag-photos"
  | "directory"
  | "search"
  | "settings";

export interface AppState {
  view: ViewMode;
  searchQuery: string;
  selectedTagId: number | null;
  selectedDirPath: string | null;
  selectedYearMonth: string | null;
}
