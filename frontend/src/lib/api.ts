// Thin wrappers around `invoke()` so components don't sprinkle string command
// names everywhere. Keeping the wrapper layer also means the rest of the
// frontend doesn't depend on @tauri-apps/api directly — easy to swap for a
// mock during component development with `npm run dev` outside Tauri.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

import type {
  FileId,
  FileMeta,
  LogLevel,
  LogLine,
  Match,
  SearchOptions,
  TailEvent,
} from "./types";

export type SessionId = number;
export type ViewId = number;

export interface ViewMeta {
  id: ViewId;
  line_count: number;
}

export interface FolderInfo {
  path: string;
  files: FileMeta[];
}

export async function pickAndOpen(): Promise<FileMeta | null> {
  const selected = await openDialog({
    multiple: false,
    title: "Open log file",
    filters: [
      { name: "Logs", extensions: ["log", "txt", "out", "err", "trace"] },
      { name: "All files", extensions: ["*"] },
    ],
  });
  if (!selected || Array.isArray(selected)) return null;
  return openFile(selected);
}

export async function pickFolder(): Promise<FolderInfo | null> {
  const selected = await openDialog({
    multiple: false,
    directory: true,
    title: "Open log folder",
  });
  if (!selected || Array.isArray(selected)) return null;
  return openFolder(selected);
}

export function openFolder(path: string): Promise<FolderInfo> {
  return invoke("open_folder", { path });
}

export function openFile(path: string): Promise<FileMeta> {
  return invoke("open_file", { path });
}

export function closeFile(id: FileId): Promise<void> {
  return invoke("close_file", { id });
}

export function fileMeta(id: FileId): Promise<FileMeta> {
  return invoke("file_meta", { id });
}

export function readRange(id: FileId, start: number, end: number): Promise<LogLine[]> {
  return invoke("read_range", { id, start, end });
}

export function createSession(fileIds: FileId[]): Promise<SessionId> {
  return invoke("create_session", { fileIds });
}

export function createView(sessionId: SessionId, levels: LogLevel[]): Promise<ViewId> {
  return invoke("create_view", { sessionId, levels });
}

export function sortView(id: ViewId, column: string, direction: string): Promise<void> {
  return invoke("sort_view", { id, column, direction });
}

export function viewMeta(id: ViewId): Promise<ViewMeta> {
  return invoke("view_meta", { id });
}

export function readViewRange(id: ViewId, start: number, end: number): Promise<any[]> {
  return invoke("read_view_range", { id, start, end });
}

export function search(id: FileId, options: SearchOptions): Promise<Match[]> {
  return invoke("search", { id, options });
}

export function startTail(id: FileId): Promise<void> {
  return invoke("start_tail", { id });
}

export function stopTail(id: FileId): Promise<void> {
  return invoke("stop_tail", { id });
}

export function onTail(id: FileId, handler: (ev: TailEvent) => void): Promise<UnlistenFn> {
  return listen<TailEvent>(`tail:${id}`, (e) => handler(e.payload));
}
