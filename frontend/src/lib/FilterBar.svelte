<script lang="ts">
  import type { LogLevel } from "./types";

  let {
    pattern = $bindable(""),
    regex = $bindable(false),
    caseSensitive = $bindable(false),
    selectedLevels = $bindable([]),
    matchCount = 0,
    activeMatchIdx = 0,
    bookmarkCount = 0,
    bookmarkMode = $bindable(false),
    matchesOnly = $bindable(false),
    onSearch,
    onPrevMatch = () => {},
    onNextMatch = () => {},
  }: {
    pattern?: string;
    regex?: boolean;
    caseSensitive?: boolean;
    selectedLevels?: LogLevel[];
    matchCount?: number;
    activeMatchIdx?: number;
    bookmarkCount?: number;
    bookmarkMode?: boolean;
    matchesOnly?: boolean;
    onSearch: () => void;
    onPrevMatch?: () => void;
    onNextMatch?: () => void;
  } = $props();

  const levels: LogLevel[] = ["trace", "debug", "info", "warn", "error", "fatal"];
  const LEVEL_ABBR: Record<LogLevel, string> = {
    trace: 'TRC',
    debug: 'DBG',
    info: 'NFO',
    warn: 'WRN',
    error: 'ERR',
    fatal: 'CRT',
  };

  function toggleLevel(level: LogLevel) {
    if (selectedLevels.includes(level)) {
      selectedLevels = selectedLevels.filter((l) => l !== level);
    } else {
      selectedLevels = [...selectedLevels, level];
    }
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    onSearch();
  }
</script>

<div class="filter-bar">
  <form class="search-form" onsubmit={submit}>
    <div class="input-group">
      <input
        type="search"
        placeholder="Search logs..."
        bind:value={pattern}
        spellcheck="false"
        autocomplete="off"
      />
      <div class="toggles">
        {#if matchCount > 0}
          <div class="match-count">
            {activeMatchIdx} / {matchCount}
          </div>
          <button
            type="button"
            class="nav-btn"
            onclick={onPrevMatch}
            title="Previous match"
            disabled={matchCount === 0}
          >
            ▲
          </button>
          <button
            type="button"
            class="nav-btn"
            onclick={onNextMatch}
            title="Next match"
            disabled={matchCount === 0}
          >
            ▼
          </button>
        {/if}
        <button
          type="button"
          class="toggle-btn"
          class:active={regex}
          onclick={() => (regex = !regex)}
          title="Regular Expression (.*)"
        >
          .*
        </button>
        <button
          type="button"
          class="toggle-btn"
          class:active={caseSensitive}
          onclick={() => (caseSensitive = !caseSensitive)}
          title="Case Sensitive (Ab)"
        >
          Aa
        </button>
        {#if pattern}
          <button
            type="button"
            class="toggle-btn"
            class:active={matchesOnly}
            onclick={() => (matchesOnly = !matchesOnly)}
            title="Show matches only (auto-enabled)"
          >
            ◆
          </button>
        {/if}
      </div>
    </div>
  </form>

  <div class="divider"></div>

  <div class="level-pills">
    {#each levels as level}
      <button
        class="pill level-{level}"
        class:active={selectedLevels.includes(level)}
        onclick={() => toggleLevel(level)}
      >
        {LEVEL_ABBR[level]}
      </button>
    {/each}
    <button
      class="pill bookmark-pill"
      class:active={bookmarkMode}
      title={`Show only bookmarks (${bookmarkCount})`}
      onclick={() => (bookmarkMode = !bookmarkMode)}
    >
      <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
        <path d="M2 1h8v9L6 8l-4 2V1z" />
      </svg>
    </button>
  </div>
</div>

<style>
  .filter-bar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 16px;
    background: var(--color-filter-bar-bg);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }
  .search-form {
    flex: 1;
    max-width: 600px;
  }
  .input-group {
    position: relative;
    display: flex;
    align-items: center;
  }
  .input-group input {
    width: 100%;
    padding-right: 180px;
    height: 32px;
  }
  .input-group input::-webkit-search-cancel-button {
    margin-right: 8px;
  }
  .toggles {
    position: absolute;
    right: 20px;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    gap: 4px;
    align-items: center;
    z-index: 5;
    pointer-events: auto;
  }
  .match-count {
    font-size: 11px;
    color: var(--color-text-secondary);
    font-weight: 600;
    white-space: nowrap;
  }
  .nav-btn {
    background: none;
    border: none;
    border-radius: 3px;
    padding: 2px 4px;
    font-size: 10px;
    color: var(--color-text-secondary);
    cursor: pointer;
  }
  .nav-btn:hover:not(:disabled) {
    background: var(--color-border-subtle);
    color: var(--color-text-primary);
  }
  .nav-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .toggle-btn {
    background: none;
    border: none;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-secondary);
    cursor: pointer;
  }
  .toggle-btn:hover {
    background: var(--color-border-subtle);
    color: var(--color-text-primary);
  }
  .toggle-btn.active {
    background: rgba(91, 163, 255, 0.15);
    color: var(--color-accent);
    border: 1px solid var(--color-accent);
  }

  .divider {
    width: 1px;
    height: 20px;
    background: var(--color-border);
  }

  .level-pills {
    display: flex;
    gap: 6px;
  }
  .pill {
    padding: 4px 10px;
    border: none;
    border-radius: 4px;
    background: var(--color-border-subtle);
    color: var(--color-text-secondary);
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.1s ease;
  }
  .pill.active {
    color: white;
  }
  .pill.level-trace.active { background: #636366; }
  .pill.level-debug.active { background: var(--color-accent); }
  .pill.level-info.active  { background: #7eb6ff; }
  .pill.level-warn.active  { background: #ffd60a; color: #1a1a1e; }
  .pill.level-error.active { background: #ff453a; }
  .pill.level-fatal.active { background: #ff453a; }

  .pill.bookmark-pill {
    padding: 4px 8px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .pill.bookmark-pill.active {
    background: var(--color-following);
    color: #000;
  }

  .pill:not(.active):hover {
    background: var(--color-border);
    color: var(--color-text-primary);
  }
</style>
