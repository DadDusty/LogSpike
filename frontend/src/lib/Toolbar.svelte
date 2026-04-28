<script lang="ts">
  let {
    liveMode,
    viewMode = "table",
    theme = "system",
    onOpen,
    onOpenFolder,
    onToggleLiveMode,
    onViewModeChange = (_: "raw" | "table") => {},
    onThemeChange = () => {},
  }: {
    liveMode: boolean;
    viewMode?: "raw" | "table";
    theme?: "light" | "dark" | "system";
    onOpen: () => void;
    onOpenFolder: () => void;
    onToggleLiveMode: () => void;
    onViewModeChange?: (mode: "raw" | "table") => void;
    onThemeChange?: () => void;
  } = $props();
</script>

<header class="toolbar">
  <div class="left">
    <div class="btn-group">
      <button onclick={onOpen}>Open File</button>
      <button onclick={onOpenFolder}>Open Folder</button>
    </div>
  </div>

  <div class="middle">
    <div class="segmented-control">
      <button 
        class:active={viewMode === 'table'} 
        onclick={() => onViewModeChange('table')}
      >Table</button>
      <button 
        class:active={viewMode === 'raw'} 
        onclick={() => onViewModeChange('raw')}
      >Raw</button>
    </div>
  </div>

  <div class="right">
    <button
      class="theme-btn"
      title={theme === 'system' ? 'System' : theme === 'light' ? 'Light' : 'Dark'}
      onclick={onThemeChange}
    >
      {#if theme === 'system'}
        ◑
      {:else if theme === 'light'}
        ☀
      {:else}
        ☾
      {/if}
    </button>
    <button class="live-btn" class:active={liveMode} onclick={onToggleLiveMode}>
      <span class="indicator"></span>
      {liveMode ? 'Live Mode' : 'Go Live'}
    </button>
  </div>
</header>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 16px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .left, .right { display: flex; align-items: center; gap: 8px; }
  
  .btn-group {
    display: flex;
    gap: 1px;
    background: var(--border);
    padding: 1px;
    border-radius: 6px;
    overflow: hidden;
  }
  .btn-group button {
    border-radius: 0;
    border: none;
    margin: 0;
  }
  .btn-group button:first-child { border-top-left-radius: 5px; border-bottom-left-radius: 5px; }
  .btn-group button:last-child { border-top-right-radius: 5px; border-bottom-right-radius: 5px; }

  .primary {
    background: var(--accent);
    color: white;
  }
  .primary:hover { background: #0077ed; }

  .segmented-control {
    display: flex;
    background: var(--border-dim);
    padding: 2px;
    border-radius: 8px;
  }
  .segmented-control button {
    background: none;
    border: none;
    border-radius: 6px;
    padding: 4px 16px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-dim);
  }
  .segmented-control button.active {
    background: var(--bg);
    color: var(--text);
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  }

  .theme-btn {
    background: none;
    border: none;
    font-size: 16px;
    cursor: pointer;
    padding: 4px 8px;
    color: var(--text-dim);
    border-radius: 4px;
  }
  .theme-btn:hover {
    color: var(--text);
    background: var(--border-dim);
  }

  .live-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-dim);
  }
  .live-btn.active {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-dim);
  }
  .live-btn.active .indicator {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0% { opacity: 1; }
    50% { opacity: 0.4; }
    100% { opacity: 1; }
  }
</style>
