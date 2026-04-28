<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { CHUNK_SIZE, LineCache } from "./lineCache";
  import { readRange, readViewRange, sortView } from "./api";
  import type { FileId, LogLine } from "./types";

  let {
    fileId,
    lineCount,
    viewMode = "table",
    isView = false,
    rowHeight = 24,
    onActiveChange = (_first: number, _last: number) => {},
    openFiles = undefined,
    searchMatches = [],
    activeMatchIdx = 0,
    bookmarks = [],
    bookmarkMode = false,
    matchesOnly = false,
    onBookmarkToggle = () => {},
    onLinesLoaded = () => {},
    onClearFilters = () => {},
  }: {
    fileId: number;
    lineCount: number;
    viewMode?: "raw" | "table";
    isView?: boolean;
    rowHeight?: number;
    onActiveChange?: (first: number, last: number) => void;
    openFiles?: any[];
    searchMatches?: number[];
    activeMatchIdx?: number;
    bookmarks?: number[];
    bookmarkMode?: boolean;
    matchesOnly?: boolean;
    onBookmarkToggle?: (line: number) => void;
    onLinesLoaded?: (lines: { line: number; level: string }[]) => void;
    onClearFilters?: () => void;
  } = $props();

  import type { FileMeta, LogLevel } from "./types";

  const LEVEL_ABBR: Record<LogLevel, string> = {
    trace: 'TRC',
    debug: 'DBG',
    info: 'NFO',
    warn: 'WRN',
    error: 'ERR',
    fatal: 'CRT',
  };

  type ColumnDef = {
    key: string;
    label: string;
    width: number | '1fr';
    minWidth: number;
    visible: boolean;
    resizable: boolean;
  };

  const DEFAULT_COLUMNS: ColumnDef[] = [
    { key: 'bookmark',  label: '⭐',        width: 20,   minWidth: 20,  visible: true, resizable: false },
    { key: 'number',    label: '#',         width: 48,   minWidth: 32,  visible: true, resizable: false },
    { key: 'timestamp', label: 'Time',       width: 200,  minWidth: 80,  visible: true, resizable: true  },
    { key: 'level',     label: 'Lvl',        width: 40,   minWidth: 32,  visible: true, resizable: false },
    { key: 'component', label: 'Component',  width: 140,  minWidth: 60,  visible: true, resizable: true  },
    { key: 'source',    label: 'Source',     width: 120,  minWidth: 60,  visible: true, resizable: true  },
    { key: 'message',   label: 'Message',    width: '1fr',minWidth: 100, visible: true, resizable: false },
  ];

  const MAX_VIRTUAL_PX = 30_000_000;
  const OVERSCAN = 12;
  const FETCH_DEBOUNCE_MS = 16;

  let viewport: HTMLDivElement | undefined = $state();
  let headerRow: HTMLDivElement | undefined = $state();
  let viewportHeight = $state(0);
  let scrollTop = $state(0);
  const cache = new LineCache();

  let firstLine = $state(0);
  let lastLine = $state(0);
  let rendered = $state<(any | null)[]>([]);
  let renderOffsetPx = $state(0);

  let sortCol = $state<string | null>(null);
  let sortDir = $state<'asc' | 'desc'>('asc');

  let columns = $state<ColumnDef[]>(DEFAULT_COLUMNS);
  let showSettings = $state(false);
  let resizing = $state<{ key: string; startX: number; startWidth: number } | null>(null);
  let contextMenu = $state<{ x: number; y: number; lineNumber: number } | null>(null);

  function loadColumnConfig() {
    try {
      const stored = localStorage.getItem('logspike:columns');
      if (stored) {
        const parsed = JSON.parse(stored) as ColumnDef[];
        const merged = DEFAULT_COLUMNS.map(def => {
          const found = parsed.find(c => c.key === def.key);
          return found ? { ...def, ...found } : def;
        });
        columns = merged;
      }
    } catch (e) {
      console.error('failed to load column config', e);
    }
  }

  function saveColumnConfig() {
    try {
      localStorage.setItem('logspike:columns', JSON.stringify(columns));
    } catch (e) {
      console.error('failed to save column config', e);
    }
  }

  function toggleColumnVisibility(key: string) {
    const col = columns.find(c => c.key === key);
    if (col && col.key !== 'message') {
      col.visible = !col.visible;
      columns = columns;
      saveColumnConfig();
    }
  }

  function resetColumns() {
    columns = JSON.parse(JSON.stringify(DEFAULT_COLUMNS));
    saveColumnConfig();
  }

  function showRowMenu(e: MouseEvent, lineNumber: number) {
    e.preventDefault();
    e.stopPropagation();
    contextMenu = { x: e.clientX, y: e.clientY, lineNumber };
  }

  function goToRow(lineNumber: number, onClearFilters?: () => void) {
    // Clear all filters before jumping
    onClearFilters?.();

    if (!viewport) return;
    if (!remappingActive) {
      viewport.scrollTop = lineNumber * rowHeight;
    } else {
      const visibleLines = Math.max(1, Math.floor(viewportHeight / rowHeight));
      const ratio = lineNumber / Math.max(1, lineCount - visibleLines);
      viewport.scrollTop = ratio * Math.max(1, spacerHeight - viewportHeight);
    }
    contextMenu = null;
  }

  function onResizeStart(e: PointerEvent, key: string) {
    const col = columns.find(c => c.key === key);
    if (!col || col.width === '1fr') return;
    e.preventDefault();
    resizing = { key, startX: e.clientX, startWidth: col.width as number };

    const handleMove = (e: PointerEvent) => {
      if (!resizing) return;
      const delta = e.clientX - resizing.startX;
      const colDef = columns.find(c => c.key === resizing.key)!;
      const newWidth = Math.max(resizing.startWidth + delta, colDef.minWidth);
      colDef.width = newWidth;
      columns = columns;
    };

    const handleEnd = () => {
      if (resizing) {
        saveColumnConfig();
        resizing = null;
      }
      window.removeEventListener('pointermove', handleMove);
      window.removeEventListener('pointerup', handleEnd);
    };

    window.addEventListener('pointermove', handleMove);
    window.addEventListener('pointerup', handleEnd);
  }

  const fileNameMap = $derived(
    new Map((openFiles ?? []).map(f => [f.id, f.path.split('/').pop() ?? f.path]))
  );

  const searchMatchSet = $derived(new Set(searchMatches ?? []));
  const bookmarkSet = $derived(new Set(bookmarks ?? []));

  const gridTemplateColumns = $derived(
    columns
      .filter(c => c.visible)
      .map(c => typeof c.width === 'number' ? `${c.width}px` : c.width)
      .join(' ')
  );

  $effect(() => {
    void fileId;
    cache.clear();
    const maxScroll = Math.max(0, spacerHeight - viewportHeight);
    if (viewport && viewport.scrollTop > maxScroll) {
      viewport.scrollTop = maxScroll;
    }
    refresh();
  });

  $effect(() => {
    // Re-render when matchesOnly or bookmark mode changes to apply filters
    void matchesOnly;
    void bookmarkMode;
    refresh();
  });

  $effect(() => {
    // Fetch initial chunks when lineCount is set to populate minimap
    if (lineCount > 0) {
      const numChunks = Math.ceil(lineCount / CHUNK_SIZE);
      const chunkCount = Math.min(Math.max(20, Math.ceil(numChunks * 0.1)), numChunks);
      const chunks = [];
      for (let i = 0; i < chunkCount; i++) {
        chunks.push(i);
      }
      if (chunks.length > 0) {
        fetchChunks(chunks).catch(err => console.error("initial fetch failed:", err));
      }
    }
  });

  const spacerHeight = $derived(Math.min(lineCount * rowHeight, MAX_VIRTUAL_PX));
  const remappingActive = $derived(lineCount * rowHeight > MAX_VIRTUAL_PX);

  function topLineForScroll(scrollTop: number): number {
    if (!remappingActive) return Math.floor(scrollTop / rowHeight);
    const visibleLines = Math.max(1, Math.floor(viewportHeight / rowHeight));
    const max = Math.max(1, spacerHeight - viewportHeight);
    const ratio = scrollTop / max;
    return Math.floor(ratio * Math.max(0, lineCount - visibleLines));
  }

  let pendingFetchTimer: number | undefined;
  function scheduleFetch(missing: number[]) {
    if (missing.length === 0) return;
    if (pendingFetchTimer !== undefined) clearTimeout(pendingFetchTimer);
    pendingFetchTimer = window.setTimeout(() => {
      pendingFetchTimer = undefined;
      fetchChunks(missing).catch((err) => console.error("fetch failed", err));
    }, FETCH_DEBOUNCE_MS);
  }

  let scrollRAFId: number | undefined;
  function onScroll() {
    scrollTop = viewport.scrollTop;
    if (scrollRAFId !== undefined) cancelAnimationFrame(scrollRAFId);
    scrollRAFId = requestAnimationFrame(refresh);
  }

  async function fetchChunks(chunkIds: number[]) {
    chunkIds.sort((a, b) => a - b);
    let i = 0;
    while (i < chunkIds.length) {
      let j = i;
      while (j + 1 < chunkIds.length && chunkIds[j + 1] === chunkIds[j] + 1) j++;
      const start = chunkIds[i] * CHUNK_SIZE;
      const end = Math.min(lineCount, (chunkIds[j] + 1) * CHUNK_SIZE);
      try {
        const lines = isView 
          ? await readViewRange(fileId, start, end)
          : await readRange(fileId, start, end);
        
        for (let c = chunkIds[i]; c <= chunkIds[j]; c++) {
          const cstart = c * CHUNK_SIZE;
          const cend = Math.min(lineCount, cstart + CHUNK_SIZE);
          const slice = lines.slice(cstart - start, cend - start);
          cache.put(c, slice);

          // Report level markers to parent
          const levelData = slice
            .map((line, idx) => ({
              line: cstart + idx,
              level: line.parsed?.level ?? 'plain',
            }))
            .filter(item => item.level !== 'plain');
          if (levelData.length > 0) {
            onLinesLoaded(levelData);
          }
        }
        refresh();
      } catch (e) {
        console.error("fetch range failed", e);
      }
      i = j + 1;
    }
  }

  function refresh() {
    if (lineCount === 0 || viewportHeight === 0) {
      rendered = [];
      firstLine = 0;
      lastLine = 0;
      return;
    }
    const top = topLineForScroll(scrollTop);
    const visible = Math.max(1, Math.ceil(viewportHeight / rowHeight));
    const start = Math.max(0, top - OVERSCAN);
    const end = Math.min(lineCount, top + visible + OVERSCAN);
    firstLine = start;
    lastLine = end;
    renderOffsetPx = remappingActive
      ? scrollTop - (top - start) * rowHeight
      : start * rowHeight;
    const { lines, missing } = cache.slice(start, end);
    rendered = lines;
    onActiveChange(top, top + visible);
    if (missing.length) scheduleFetch(missing);
  }


  function onResize() {
    viewportHeight = viewport.clientHeight;
    refresh();
  }

  async function toggleSort(col: string) {
    if (!isView) return;
    if (sortCol === col) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortCol = col;
      sortDir = 'asc';
    }
    await sortView(fileId, sortCol, sortDir);
    cache.clear();
    refresh();
  }

  onMount(() => {
    loadColumnConfig();
    viewportHeight = viewport.clientHeight;
    const ro = new ResizeObserver(onResize);
    ro.observe(viewport);
    refresh();

    // Fetch initial chunks to populate levelMarkers for minimap
    queueMicrotask(() => {
      const chunks = [];
      for (let i = 0; i < Math.min(3, Math.ceil(lineCount / CHUNK_SIZE)); i++) {
        chunks.push(i);
      }
      if (chunks.length > 0) {
        fetchChunks(chunks).catch(err => console.error("initial fetch failed:", err));
      }
    });

    return () => ro.disconnect();
  });

  onDestroy(() => {
    if (pendingFetchTimer !== undefined) clearTimeout(pendingFetchTimer);
    if (scrollRAFId !== undefined) cancelAnimationFrame(scrollRAFId);
  });

  export function scrollToLine(line: number) {
    if (!viewport) return;
    if (!remappingActive) {
      viewport.scrollTop = line * rowHeight;
    } else {
      const visibleLines = Math.max(1, Math.floor(viewportHeight / rowHeight));
      const ratio = line / Math.max(1, lineCount - visibleLines);
      viewport.scrollTop = ratio * Math.max(1, spacerHeight - viewportHeight);
    }
  }
</script>

<div class="list-container">
  {#if viewMode === "table"}
    <div class="header-row" bind:this={headerRow}>
      <div class="header-columns" style:grid-template-columns={gridTemplateColumns}>
        {#each columns as col (col.key)}
          {#if col.visible}
            <div class="h-col-wrapper">
              {#if col.key === 'bookmark'}
                <span class="h-col h-bookmark">
                  <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
                    <path d="M2 1h10v11L7 9l-5 3V1z" />
                  </svg>
                </span>
              {:else if col.key === 'number' || col.key === 'level'}
                <span class="h-col">{col.label}</span>
              {:else}
                <button class="h-col" onclick={() => toggleSort(col.key)}>
                  {col.label} {sortCol === col.key ? (sortDir === 'asc' ? '▴' : '▾') : ''}
                </button>
              {/if}
              {#if col.resizable && col.key !== 'message'}
                <div
                  class="resize-handle"
                  role="button"
                  tabindex="0"
                  onpointerdown={(e) => onResizeStart(e, col.key)}
                  aria-label="Resize {col.label} column"
                ></div>
              {/if}
            </div>
          {/if}
        {/each}
      </div>
      <button class="settings-btn" onclick={() => (showSettings = true)}>⚙</button>
    </div>
  {/if}

  <div bind:this={viewport} class="viewport" onscroll={onScroll}>
    <div class="spacer" style:height="{spacerHeight}px">
      <div class="rows" style:transform="translate3d(0,{renderOffsetPx}px,0)">
        {#each rendered as line, i (firstLine + i)}
          {#if line && (!bookmarkMode || bookmarkSet.has(line.number))}
            {#if !matchesOnly || searchMatchSet.has(line.number)}
              {@const isMatch = searchMatchSet.has(line.number)}
              {@const activeMatchLine = searchMatches?.[activeMatchIdx]}
              {@const isActive = activeMatchLine !== undefined && line.number === activeMatchLine}
              <div
                class="row {viewMode === 'table' ? 'row-table' : ''} level-{line.parsed?.level ?? 'plain'}"
                class:row-match={isMatch}
                class:row-match-active={isActive}
                role="option"
                oncontextmenu={(e) => showRowMenu(e, line.number)}
                style:height="{rowHeight}px"
                style:line-height="{rowHeight}px"
                style:grid-template-columns={viewMode === 'table' ? gridTemplateColumns : undefined}
              >
                {#if viewMode === "table"}
                  {#each columns as col (col.key)}
                    {#if col.visible}
                      {#if col.key === 'bookmark'}
                        <button
                          class="bookmark-btn"
                          class:bookmarked={bookmarkSet.has(line.number)}
                          onclick={() => onBookmarkToggle(line.number)}
                          title="Bookmark this line"
                        >
                          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
                            {#if bookmarkSet.has(line.number)}
                              <path d="M2 1h10v11L7 9l-5 3V1z" />
                            {:else}
                              <path d="M2 1h10v11L7 9l-5 3V1z" opacity="0.4" />
                            {/if}
                          </svg>
                        </button>
                      {:else if col.key === 'number'}
                        <span class="ln">{line.number + 1}</span>
                      {:else if col.key === 'timestamp'}
                        <span class="col timestamp">{line.parsed?.timestamp ?? ""}</span>
                      {:else if col.key === 'level'}
                        <span class="col level-badge" title={line.parsed?.level ?? ""}>
                          {line.parsed?.level ? LEVEL_ABBR[line.parsed.level] : "—"}
                        </span>
                      {:else if col.key === 'component'}
                        <span class="col component">{line.parsed?.component ?? ""}</span>
                      {:else if col.key === 'source'}
                        <span class="col source">{fileNameMap.get(line.source_id ?? 0) ?? ''}</span>
                      {:else if col.key === 'message'}
                        <span class="col message">{line.parsed?.message ?? line.text}</span>
                      {/if}
                    {/if}
                  {/each}
                {:else}
                  <span class="ln">{line.number + 1}</span>
                  <span class="text">{line.text}</span>
                {/if}
              </div>
            {:else if !matchesOnly}
              <div class="row placeholder" style:height="{rowHeight}px"></div>
            {/if}
          {/if}
        {/each}
      </div>
    </div>
  </div>

  {#if contextMenu}
    <button class="context-menu-backdrop" aria-label="Close context menu" onclick={() => (contextMenu = null)}></button>
    <div class="context-menu" role="menu" style:left="{contextMenu.x}px" style:top="{contextMenu.y}px">
      <button role="menuitem" onclick={() => goToRow(contextMenu!.lineNumber, onClearFilters)}>
        Go to row {contextMenu.lineNumber + 1}
      </button>
    </div>
  {/if}

  {#if showSettings}
    <button
      class="modal-backdrop"
      type="button"
      aria-label="Close settings"
      onclick={() => (showSettings = false)}
    ></button>
    <dialog open class="settings-modal">
      <h2>Column Settings</h2>
      <div class="column-list">
        {#each columns as col (col.key)}
          <label class="column-toggle">
            <input
              type="checkbox"
              checked={col.visible}
              disabled={col.key === 'message'}
              onchange={() => toggleColumnVisibility(col.key)}
            />
            <span>{col.label}</span>
          </label>
        {/each}
      </div>
      <div class="modal-actions">
        <button onclick={resetColumns} class="btn-reset">Reset to Defaults</button>
        <button onclick={() => (showSettings = false)} class="btn-close">Close</button>
      </div>
    </dialog>
  {/if}
</div>

<style>
  .list-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    position: relative;
  }
  .header-row {
    display: flex;
    align-items: center;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border);
    z-index: 10;
    gap: 8px;
    padding: 0 12px;
    height: 28px;
  }

  .header-columns {
    display: grid;
    gap: 12px;
    flex: 1;
    min-width: 0;
  }

  .h-col-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
  }
  .h-col {
    background: none;
    border: none;
    padding: 3px 0;
    text-align: left;
    font-size: 11px;
    font-weight: 700;
    color: var(--text-dim);
    text-transform: uppercase;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    height: 100%;
  }
  .h-col:hover {
    color: var(--text);
  }
  span.h-col { cursor: default; }
  .h-bookmark {
    justify-content: center;
    color: var(--text-muted);
  }
  .h-bookmark svg {
    width: 14px;
    height: 14px;
  }

  .resize-handle {
    position: absolute;
    right: -6px;
    top: 50%;
    transform: translateY(-50%);
    width: 12px;
    height: 16px;
    cursor: col-resize;
    user-select: none;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .resize-handle::before {
    content: '';
    display: block;
    width: 2px;
    height: 12px;
    background: var(--border-dim);
    border-radius: 1px;
    margin: 0 1px;
    box-shadow: 2px 0 0 var(--border-dim);
  }
  .resize-handle:hover::before {
    background: var(--text-dim);
    box-shadow: 2px 0 0 var(--text-dim);
  }

  .settings-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 14px;
    padding: 6px 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 24px;
    width: 24px;
  }
  .settings-btn:hover {
    color: var(--text);
  }

  .viewport {
    position: relative;
    flex: 1;
    overflow: auto;
    background: var(--bg);
    font-family: var(--mono);
    font-size: 12px;
    contain: strict;
  }
  .viewport::-webkit-scrollbar {
    width: 10px;
  }
  .viewport::-webkit-scrollbar-track {
    background: var(--bg);
  }
  .viewport::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 5px;
  }
  .viewport::-webkit-scrollbar-thumb:hover {
    background: var(--border-dim);
  }
  .viewport {
    scrollbar-color: var(--border) var(--bg);
    scrollbar-width: thin;
  }
  .spacer { position: relative; width: 100%; }
  .rows {
    position: absolute;
    top: 0; left: 0; right: 0;
    will-change: transform;
  }
  .row {
    display: flex;
    gap: 12px;
    padding: 0 12px;
    white-space: pre;
    overflow: hidden;
    border-bottom: 1px solid var(--border-dim);
    background: var(--bg);
    contain: layout style paint;
    animation: fadeIn 100ms ease-out;
  }
  @keyframes fadeIn {
    from { opacity: 0.8; }
    to { opacity: 1; }
  }
  .row:nth-child(even):not(.level-warn):not(.level-error):not(.level-fatal) {
    background: var(--bg-row-alt);
  }

  .ln {
    color: var(--text-muted);
    text-align: right;
    min-width: 5ch;
    user-select: none;
    flex: 0 0 auto;
    font-size: 11px;
  }
  .text { flex: 1 1 auto; min-width: 0; }
  .placeholder { background: var(--bg-elevated); opacity: 0.2; }

  .level-trace { color: var(--lv-trace); }
  .level-debug { color: var(--lv-debug); }
  .level-info  { color: var(--lv-info); }
  .level-warn  {
    color: var(--lv-warn);
    background: rgba(255, 159, 10, 0.12);
    border-left: 2px solid rgba(255, 159, 10, 0.4);
  }
  .level-error {
    color: var(--lv-error);
    background: rgba(255, 59, 48, 0.15);
    border-left: 2px solid var(--lv-error);
  }
  .level-fatal {
    color: var(--lv-fatal);
    background: rgba(191, 90, 242, 0.18);
    border-left: 2px solid var(--lv-fatal);
    font-weight: 600;
  }

  .row-table {
    display: grid;
    gap: 12px;
    align-items: center;
  }
  
  .col {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .col.timestamp { color: var(--text-dim); }
  .col.level-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 8px;
    font-weight: 800;
    height: 18px;
    width: 32px;
    border-radius: 3px;
    background: var(--border-dim);
    color: inherit;
    letter-spacing: 0.03em;
  }
  .col.component { color: var(--text-dim); }
  .col.source { color: var(--text-muted); font-size: 11px; }
  .col.message { color: var(--text); }

  .bookmark-btn {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: var(--text-muted);
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }
  .bookmark-btn:hover {
    color: var(--accent);
  }
  .bookmark-btn.bookmarked {
    color: var(--accent);
  }

  .row-match-active {
    background: rgba(224, 175, 0, 0.2) !important;
    box-shadow: inset 0 0 0 1px rgba(224, 175, 0, 0.4);
  }

  .row-match {
    background: rgba(0, 113, 227, 0.08) !important;
  }

  .context-menu-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 999;
  }

  .context-menu {
    position: fixed;
    z-index: 1000;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    min-width: 160px;
  }

  .context-menu button {
    display: block;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
    color: var(--text);
  }

  .context-menu button:hover {
    background: var(--accent-dim);
    color: var(--accent);
  }

  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 99;
    border: none;
    padding: 0;
    cursor: pointer;
  }

  .settings-modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 100;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
    min-width: 280px;
    max-width: 320px;
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.3);
  }

  .settings-modal h2 {
    margin: 0 0 10px 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }

  .column-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 12px;
    max-height: 300px;
    overflow-y: auto;
  }

  .column-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    padding: 3px 6px;
    border-radius: 3px;
    font-size: 12px;
  }

  .column-toggle:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .column-toggle input {
    cursor: pointer;
  }

  .column-toggle input:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .column-toggle span {
    color: var(--text);
    font-size: 12px;
  }

  .modal-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .btn-reset,
  .btn-close {
    padding: 6px 12px;
    border: none;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-reset {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text);
  }

  .btn-reset:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .btn-close {
    background: var(--accent, #0066ff);
    color: white;
  }

  .btn-close:hover {
    opacity: 0.8;
  }
</style>
