export type MediaType = "video" | "audio";
export type VideoFormat = "mp4" | "mkv" | "webm";
export type VideoQuality = "best" | "p1080" | "p720" | "p480";
export type AudioFormat = "mp3" | "m4a" | "flac" | "wav" | "opus";
export type AudioQuality = "best" | "high" | "medium" | "low";

export interface StartDownloadPayload {
  url: string;
  media_type?: MediaType;
  video_format?: VideoFormat;
  video_quality?: VideoQuality;
  audio_format?: AudioFormat;
  audio_quality?: AudioQuality;
  custom_output_dir?: string;
}

export interface ProgressData {
  download_id: string;
  percent: number;
  speed: string;
  eta: string;
  total_bytes: number;
  downloaded_bytes: number;
  status: string;
}

export interface CompletedData {
  download_id: string;
  file_path: string;
  file_name: string;
  file_size_bytes: number;
  extension: string;
  fallback_to_audio: boolean;
}

export interface EngineError {
  code: string;
  message: string;
}

export interface EngineResponse<T = unknown> {
  id: string;
  version: string;
  type: string;
  success: boolean;
  data?: T;
  error?: EngineError;
}

// Internal messages between Popup/Content script and Service Worker
export type InternalMessageType =
  | "START_DOWNLOAD"
  | "GET_ACTIVE_DOWNLOADS"
  | "CANCEL_DOWNLOAD"
  | "PING_ENGINE"
  | "ENGINE_STATUS";

export interface InternalMessage {
  type: InternalMessageType;
  payload?: any;
}
