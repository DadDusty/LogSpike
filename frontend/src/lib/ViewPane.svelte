<script lang="ts">
  import OverviewRuler from "./OverviewRuler.svelte";
  import FilterBar from "./FilterBar.svelte";
  import VirtualLogList from "./VirtualLogList.svelte";
  import type { FileMeta, LogLevel } from "./types";

  interface PaneState {
    id: number;
    pattern: string;
    regex: boolean;
    caseSensitive: boolean;
    selectedLevels: LogLevel[];
    searchMatches: number[];
    activeMatchIdx: number;
    bookmarks: number[];
    bookmarkMode: boolean;
    levelMarkers: { line: number; level: LogLevel }[];
    viewId: number | null;
    viewLineCount: number;
  }

  let {
    pane,
    openFiles = [],
    viewMode = "table",
    isActive = false,
    onLevelMarkers = (_markers: { line: number; level: LogLevel }[]) => {},
    onSearch = () => {},
    onBookmarkToggle = (_line: number) => {},
    onPrevMatch = () => {},
    onNextMatch = () => {},
    onToggleBookmarkMode = () => {},
    onPatternChange = (_pattern: string) => {},
    onRegexChange = (_regex: boolean) => {},
    onCaseSensitiveChange = (_case: boolean) => {},
    onLevelsChange = (_levels: LogLevel[]) => {},
    onBookmarkModeChange = (_mode: boolean) => {},
  }: {
    pane: PaneState;
    openFiles?: FileMeta[];
    viewMode?: string;
    isActive?: boolean;
    onLevelMarkers?: (markers: { line: number; level: LogLevel }[]) => void;
    onSearch?: () => void;
    onBookmarkToggle?: (line: number) => void;
    onPrevMatch?: () => void;
    onNextMatch?: () => void;
    onToggleBookmarkMode?: () => void;
    onPatternChange?: (pattern: string) => void;
    onRegexChange?: (regex: boolean) => void;
    onCaseSensitiveChange?: (cs: boolean) => void;
    onLevelsChange?: (levels: LogLevel[]) => void;
    onBookmarkModeChange?: (mode: boolean) => void;
  } = $props();

  let listRef: any = $state();

  function seekToLine(line: number) {
    listRef?.scrollToLine(line);
  }
</script>

<div class="view-pane" class:active={isActive}>
  {#if pane.viewId !== null}
    <OverviewRuler
      lineCount={pane.viewLineCount}
      searchMatches={pane.searchMatches}
      bookmarks={pane.bookmarks}
      levelMarkers={pane.levelMarkers}
      scrollFraction={0}
      viewFraction={0.1}
      onSeek={seekToLine}
    />
  {/if}

  <FilterBar
    bind:pattern={pane.pattern}
    bind:regex={pane.regex}
    bind:caseSensitive={pane.caseSensitive}
    bind:selectedLevels={pane.selectedLevels}
    bind:bookmarkMode={pane.bookmarkMode}
    matchCount={pane.searchMatches.length}
    activeMatchIdx={pane.activeMatchIdx + 1}
    bookmarkCount={pane.bookmarks.length}
    onSearch={onSearch}
    onPrevMatch={onPrevMatch}
    onNextMatch={onNextMatch}
    onToggleBookmarkMode={onToggleBookmarkMode}
  />

  <div class="pane-content">
    {#if pane.viewId !== null}
      <VirtualLogList
        bind:this={listRef}
        fileId={pane.viewId}
        lineCount={pane.viewLineCount}
        {viewMode}
        isView={true}
        {openFiles}
        searchMatches={pane.searchMatches}
        activeMatchIdx={pane.activeMatchIdx}
        bookmarks={pane.bookmarks}
        bookmarkMode={pane.bookmarkMode}
        onBookmarkToggle={onBookmarkToggle}
        onLinesLoaded={onLevelMarkers}
      />
    {:else}
      <div class="empty">
        <div class="empty-text">No view selected</div>
      </div>
    {/if}
  </div>
</div>

<style>
  .view-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg);
  }
  .pane-content {
    flex: 1;
    position: relative;
    min-height: 0;
  }
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-dim);
    font-size: 13px;
  }
</style>
