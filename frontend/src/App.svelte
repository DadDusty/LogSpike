<script lang="ts">
  import "./lib/themes.css";
  import Sidebar from "./lib/Sidebar.svelte";
  import Toolbar from "./lib/Toolbar.svelte";
  import FilterBar from "./lib/FilterBar.svelte";
  import StatusBar from "./lib/StatusBar.svelte";
  import VirtualLogList from "./lib/VirtualLogList.svelte";
  import ViewPane from "./lib/ViewPane.svelte";
  import {
    closeFile,
    createSession,
    createView,
    fileMeta,
    onTail,
    openFile,
    openFolder as openFolderByPath,
    pickAndOpen,
    pickFolder,
    search as runSearch,
    startTail,
    stopTail,
    viewMeta,
  } from "./lib/api";
  import type { FileMeta, LogLevel, TailEvent } from "./lib/types";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import OverviewRuler from "./lib/OverviewRuler.svelte";

  interface PaneState {
    id: number;
    fileId: number | null;
    sessionId: number | null;
    viewId: number | null;
    viewLineCount: number;
    activeSessionType: 'file' | 'all';
    pattern: string;
    regex: boolean;
    caseSensitive: boolean;
    selectedLevels: LogLevel[];
    searchMatches: number[];
    activeMatchIdx: number;
    bookmarks: number[];
    bookmarkMode: boolean;
    levelMarkers: { line: number; level: LogLevel }[];
    liveMode: boolean;
  }

  function newPane(id: number): PaneState {
    return {
      id,
      fileId: null,
      sessionId: null,
      viewId: null,
      viewLineCount: 0,
      activeSessionType: 'all',
      pattern: "",
      regex: false,
      caseSensitive: false,
      selectedLevels: [],
      searchMatches: [],
      activeMatchIdx: 0,
      bookmarks: [],
      bookmarkMode: false,
      levelMarkers: [],
      liveMode: false,
    };
  }

  let openFiles = $state<FileMeta[]>([]);
  let folderPath = $state<string | null>(null);

  let panes = $state<PaneState[]>([newPane(0)]);
  let activePaneId = $state(0);
  let splitView = $state(false);
  let splitRatio = $state(0.5);

  let status = $state<string>("");
  let viewMode = $state<"raw" | "table">("table");

  let theme = $state<'light' | 'dark' | 'system'>('system');

  const tailListeners = new Map<number, UnlistenFn>();

  // Derived state for active pane (read-only reference)
  const activePane = $derived(panes.find(p => p.id === activePaneId) || panes[0]);

  let listRef = $state<{ scrollToLine: (line: number) => void } | undefined>(undefined);

  async function open() {
    try {
      const next = await pickAndOpen();
      if (!next) return;
      openFiles = [...openFiles, next];
      await selectFile(next.id);
    } catch (err) {
      status = String(err);
    }
  }

  async function pickAndOpenFolder() {
    try {
      const info = await pickFolder();
      if (!info) return;
      folderPath = info.path;
      openFiles = [...openFiles, ...info.files];
      await selectAll();
    } catch (err) {
      status = String(err);
    }
  }

  async function selectFileInPane(paneId: number, fileId: number) {
    const pane = panes.find(p => p.id === paneId);
    if (!pane) return;

    pane.fileId = fileId;
    pane.activeSessionType = 'file';
    status = `Loading ${openFiles.find(f => f.id === fileId)?.path ?? ''}...`;

    pane.levelMarkers = [];
    pane.bookmarks = loadBookmarksForPane(fileId);

    const sid = await createSession([fileId]);
    pane.sessionId = sid;
    await updateViewInPane(pane);
    status = "";

    if (pane.liveMode) await ensureTailing(fileId);
  }

  async function selectAllInPane(paneId: number) {
    const pane = panes.find(p => p.id === paneId);
    if (!pane) return;

    pane.activeSessionType = 'all';
    pane.fileId = null;
    status = "Merging all logs...";

    pane.levelMarkers = [];
    pane.bookmarks = loadBookmarksForPane(null);

    const sid = await createSession(openFiles.map(f => f.id));
    pane.sessionId = sid;
    await updateViewInPane(pane);
    status = "";

    if (pane.liveMode) {
      for (const f of openFiles) await ensureTailing(f.id);
    }
  }

  async function selectFile(id: number) {
    activePaneId = activePane.id;
    await selectFileInPane(activePaneId, id);
  }

  async function selectAll() {
    activePaneId = activePane.id;
    await selectAllInPane(activePaneId);
  }

  async function updateViewInPane(pane: PaneState) {
    if (pane.sessionId === null) return;
    const vid = await createView(pane.sessionId, pane.selectedLevels);
    pane.viewId = vid;
    const vmeta = await viewMeta(vid);
    pane.viewLineCount = vmeta.line_count;
  }

  async function updateView() {
    await updateViewInPane(activePane);
  }

  async function close(id: number) {
    await closeFile(id);
    const listener = tailListeners.get(id);
    if (listener) {
      listener();
      tailListeners.delete(id);
    }
    const remaining = openFiles.filter(f => f.id !== id);
    openFiles = remaining;

    // Clear view if closing the active file or if no files remain
    if (activePane.fileId === id || remaining.length === 0) {
      activePane.fileId = null;
      activePane.sessionId = null;
      activePane.viewId = null;
      activePane.viewLineCount = 0;
      activePane.searchMatches = [];
      activePane.activeMatchIdx = 0;
    }

    // If in 'all' mode and files remain, rebuild session
    if (activePane.activeSessionType === 'all' && remaining.length > 0) {
      await selectAll();
    }
  }

  async function closeAll() {
    await stopAllTailing();
    for (const file of openFiles) {
      await closeFile(file.id);
    }
    openFiles = [];
    activePane.fileId = null;
    activePane.sessionId = null;
    activePane.viewId = null;
    activePane.viewLineCount = 0;
    activePane.searchMatches = [];
    activePane.activeMatchIdx = 0;
  }

  function clearAllFilters() {
    activePane.pattern = "";
    activePane.regex = false;
    activePane.caseSensitive = false;
    activePane.bookmarkMode = false;
    activePane.searchMatches = [];
    activePane.activeMatchIdx = 0;
  }

  async function ensureTailing(id: number) {
    if (tailListeners.has(id)) return;

    const unlisten = await onTail(id, (ev: TailEvent) => {
      if (ev.kind === "appended") {
        refreshFileAndSession(id);
      }
    });
    tailListeners.set(id, unlisten);
    await startTail(id);
  }

  async function stopAllTailing() {
    for (const [id, unlisten] of tailListeners.entries()) {
      await stopTail(id);
      unlisten();
    }
    tailListeners.clear();
  }

  async function refreshFileAndSession(id: number) {
    const meta = await fileMeta(id);
    openFiles = openFiles.map(f => f.id === id ? meta : f);

    // Refresh current view if it contains this file
    if (activePane.activeSessionType === 'all' || activePane.fileId === id) {
      if (activePane.activeSessionType === 'all') await selectAll();
      else await selectFile(id);
    }
  }

  async function toggleLiveMode() {
    activePane.liveMode = !activePane.liveMode;
    if (activePane.liveMode) {
      if (activePane.activeSessionType === 'file' && activePane.fileId) {
        await ensureTailing(activePane.fileId);
      } else {
        for (const f of openFiles) await ensureTailing(f.id);
      }
      listRef?.scrollToLine(activePane.viewLineCount - 1);
    } else {
      await stopAllTailing();
    }
  }

  async function doSearch() {
    // Clear if pattern is empty
    if (!activePane.pattern) {
      activePane.searchMatches = [];
      activePane.activeMatchIdx = 0;
      status = "";
      return;
    }

    if (!activePane.fileId) {
      status = "select a file to search";
      return;
    }

    try {
      const hits = await runSearch(activePane.fileId, {
        pattern: activePane.pattern,
        regex: activePane.regex,
        case_sensitive: activePane.caseSensitive,
      });

      if (hits.length === 0) {
        status = "no matches";
        activePane.searchMatches = [];
        activePane.activeMatchIdx = 0;
        return;
      }
      // Deduplicate to one match per line (keep first occurrence)
      const uniqueLines = new Set<number>();
      activePane.searchMatches = hits
        .map(h => h.line_number)
        .filter(lineNum => {
          if (uniqueLines.has(lineNum)) return false;
          uniqueLines.add(lineNum);
          return true;
        });
      activePane.activeMatchIdx = 0;
      status = `${activePane.searchMatches.length.toLocaleString()} lines match (${hits.length.toLocaleString()} total occurrences)`;
      listRef?.scrollToLine(hits[0].line_number);
    } catch (err) {
      status = String(err);
      activePane.searchMatches = [];
    }
  }

  function prevMatch() {
    if (activePane.searchMatches.length === 0) return;
    activePane.activeMatchIdx = (activePane.activeMatchIdx - 1 + activePane.searchMatches.length) % activePane.searchMatches.length;
    listRef?.scrollToLine(activePane.searchMatches[activePane.activeMatchIdx]);
  }

  function nextMatch() {
    if (activePane.searchMatches.length === 0) return;
    activePane.activeMatchIdx = (activePane.activeMatchIdx + 1) % activePane.searchMatches.length;
    listRef?.scrollToLine(activePane.searchMatches[activePane.activeMatchIdx]);
  }

  function toggleBookmark(line: number) {
    if (activePane.bookmarks.includes(line)) {
      activePane.bookmarks = activePane.bookmarks.filter(b => b !== line);
    } else {
      activePane.bookmarks = [...activePane.bookmarks, line];
    }
    saveBookmarks();
  }

  function saveBookmarks() {
    try {
      const key = `logspike:bookmarks:${activePane.fileId ?? 'all'}`;
      localStorage.setItem(key, JSON.stringify(activePane.bookmarks));
    } catch (e) {
      console.warn('failed to save bookmarks:', e);
    }
  }

  function loadBookmarks(fileId: number | null) {
    try {
      const key = `logspike:bookmarks:${fileId ?? 'all'}`;
      const saved = localStorage.getItem(key);
      activePane.bookmarks = saved ? JSON.parse(saved) : [];
    } catch (e) {
      console.warn('failed to load bookmarks:', e);
      activePane.bookmarks = [];
    }
  }

  function loadBookmarksForPane(fileId: number | null): number[] {
    try {
      const key = `logspike:bookmarks:${fileId ?? 'all'}`;
      const saved = localStorage.getItem(key);
      return saved ? JSON.parse(saved) : [];
    } catch (e) {
      console.warn('failed to load bookmarks:', e);
      return [];
    }
  }

  function saveBookmarksForPane(fileId: number | null, bookmarks: number[]) {
    try {
      const key = `logspike:bookmarks:${fileId ?? 'all'}`;
      localStorage.setItem(key, JSON.stringify(bookmarks));
    } catch (e) {
      console.warn('failed to save bookmarks:', e);
    }
  }

  function handleLinesLoaded(lines: { line: number; level: LogLevel }[]) {
    activePane.levelMarkers = [...activePane.levelMarkers, ...lines];
  }

  function toggleTheme() {
    theme = theme === 'system' ? 'light' : theme === 'light' ? 'dark' : 'system';
  }

  function applyTheme() {
    const elem = document.documentElement;
    if (theme === 'system') {
      elem.removeAttribute('data-theme');
    } else {
      elem.setAttribute('data-theme', theme);
    }
    localStorage.setItem('logspike:theme', theme);
  }

  onMount(() => {
    const saved = localStorage.getItem('logspike:theme');
    if (saved === 'light' || saved === 'dark') {
      theme = saved;
    }
    applyTheme();
    restoreSession();

    // Disable right-click context menu
    document.addEventListener('contextmenu', (e) => e.preventDefault());
  });

  async function restoreSession() {
    try {
      const saved = localStorage.getItem('logspike:session');
      if (!saved) return;
      const { paths, folder } = JSON.parse(saved);

      try {
        if (folder) {
          const info = await openFolderByPath(folder);
          folderPath = info.path;
          openFiles = info.files;
        } else if (Array.isArray(paths) && paths.length > 0) {
          for (const path of paths) {
            const meta = await openFile(path);
            openFiles = [...openFiles, meta];
          }
        }

        if (openFiles.length > 0) {
          await selectAll();
        }
      } catch (e) {
        console.warn('failed to restore files:', e);
      }
    } catch (e) {
      console.warn('failed to restore session:', e);
    }
  }

  async function onOpenFileChange() {
    localStorage.setItem('logspike:session', JSON.stringify({
      paths: openFiles.map(f => f.path),
      folder: folderPath,
    }));
  }

  $effect(() => {
    void openFiles;
    void folderPath;
    onOpenFileChange();
  });

  $effect(() => {
    applyTheme();
  });

  $effect(() => {
    if (activePane.sessionId !== null) {
      updateView();
    }
  });

  $effect(() => {
    // Scroll to first bookmark when bookmark mode is enabled
    if (activePane.bookmarkMode && activePane.bookmarks.length > 0) {
      const firstBookmark = Math.min(...activePane.bookmarks);
      queueMicrotask(() => listRef?.scrollToLine(firstBookmark));
    }
  });

  $effect(() => {
    // Live search on pattern change
    void activePane.pattern;
    void activePane.regex;
    void activePane.caseSensitive;
    doSearch().catch(err => console.error("search error:", err));
  });

  $effect(() => {
    return () => {
      stopAllTailing();
    };
  });
</script>

<div class="shell">
  <Toolbar
    liveMode={activePane.liveMode}
    {viewMode}
    {theme}
    onOpen={open}
    onOpenFolder={pickAndOpenFolder}
    onToggleLiveMode={toggleLiveMode}
    onViewModeChange={(mode) => (viewMode = mode)}
    onThemeChange={toggleTheme}
  />

  <div class="container">
    <Sidebar
      {openFiles}
      activeFileId={activePane.fileId}
      activeSessionType={activePane.activeSessionType}
      {folderPath}
      onSelect={selectFile}
      onSelectAll={selectAll}
      onClose={close}
      onCloseAll={closeAll}
    />

    <main>
      {#if activePane.viewId !== null}
        <OverviewRuler
          lineCount={activePane.viewLineCount}
          searchMatches={activePane.searchMatches}
          bookmarks={activePane.bookmarks}
          levelMarkers={activePane.levelMarkers}
          {theme}
          scrollFraction={0}
          viewFraction={0.1}
          onSeek={(line) => listRef?.scrollToLine(line)}
        />
      {/if}

      <FilterBar
        bind:pattern={activePane.pattern}
        bind:regex={activePane.regex}
        bind:caseSensitive={activePane.caseSensitive}
        bind:selectedLevels={activePane.selectedLevels}
        bind:bookmarkMode={activePane.bookmarkMode}
        matchCount={activePane.searchMatches.length}
        activeMatchIdx={activePane.activeMatchIdx + 1}
        bookmarkCount={activePane.bookmarks.length}
        onSearch={doSearch}
        onPrevMatch={prevMatch}
        onNextMatch={nextMatch}
      />

      <div class="content">
        {#if activePane.viewId !== null}
          <VirtualLogList
            bind:this={listRef}
            fileId={activePane.viewId}
            lineCount={activePane.viewLineCount}
            {viewMode}
            isView={true}
            {openFiles}
            searchMatches={activePane.searchMatches}
            activeMatchIdx={activePane.activeMatchIdx}
            bookmarks={activePane.bookmarks}
            bookmarkMode={activePane.bookmarkMode}
            onBookmarkToggle={toggleBookmark}
            onLinesLoaded={handleLinesLoaded}
            onClearFilters={clearAllFilters}
          />
        {:else}
          <div class="empty">
            <div class="empty-content">
              <div class="empty-icon">📋</div>
              <h2>No logs selected</h2>
              <p>Open a file or folder to view logs</p>
              <div class="empty-ctas">
                <button onclick={open} class="empty-cta">Open File</button>
                <button onclick={pickAndOpenFolder} class="empty-cta">Open Folder</button>
              </div>
            </div>
          </div>
        {/if}
      </div>
    </main>
  </div>

  <StatusBar
    lineCount={activePane.viewLineCount}
    {status}
    following={activePane.liveMode}
    onToggleFollow={toggleLiveMode}
    onJumpToEnd={() => listRef?.scrollToLine(activePane.viewLineCount - 1)}
  />
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-window-bg);
  }
  .container {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .content {
    flex: 1;
    position: relative;
    min-height: 0;
  }
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 20px;
  }
  .empty-content {
    text-align: center;
    max-width: 300px;
  }
  .empty-icon {
    font-size: 48px;
    margin-bottom: 16px;
    opacity: 0.5;
  }
  .empty h2 {
    margin: 0 0 8px 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--color-text-primary);
  }
  .empty p {
    margin: 0 0 24px 0;
    color: var(--color-text-secondary);
    font-size: 13px;
  }
  .empty-ctas {
    display: flex;
    gap: 8px;
  }
  .empty-cta {
    background: var(--color-accent);
    color: #ffffff;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
  }
  .empty-cta:hover {
    opacity: 0.9;
  }
</style>
