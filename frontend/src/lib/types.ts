// Mirror of the structs serialised by the Rust backend. Keep in sync with
// crates/core/src/{index,parser,search,tail}.rs.
//
// We intentionally keep these as plain types (not classes) so they cross
// the IPC boundary as cheap JSON.

export type FileId = number;

export type LogLevel =
  | "trace"
  | "debug"
  | "info"
  | "warn"
  | "error"
  | "fatal";

export type FileFormat = "plain" | "json" | "cmtrace" | "nginx";

export interface ParsedLine {
  level: LogLevel | null;
  timestamp: string | null;
  component: string | null;
  source: string | null;
  message: string | null;
}

export interface LogLine {
  number: number;
  byte_offset: number;
  text: string;
  parsed: ParsedLine | null;
  source_id?: number;
}

export interface FileMeta {
  id: FileId;
  path: string;
  byte_len: number;
  line_count: number;
  format: FileFormat;
  last_timestamp?: string | null;
}

export interface SearchOptions {
  pattern: string;
  case_sensitive?: boolean;
  regex?: boolean;
  max_results?: number | null;
}

export interface Match {
  line_number: number;
  byte_offset: number;
  match_start: number;
  match_end: number;
}

export type TailEvent =
  | { kind: "appended"; count: number; total_lines: number }
  | { kind: "rotated" }
  | { kind: "removed" };
