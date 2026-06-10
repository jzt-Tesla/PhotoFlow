export interface Photo {
  id: number;
  path: string;
  filename: string;
  created_time: string;
  width: number;
  height: number;
  thumbnail_path: string;
}

export interface ScanResult {
  message: string;
  found: number;
  indexed: number;
  errors: number;
  cleanup_removed: number;
}
