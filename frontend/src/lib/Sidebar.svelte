<script lang="ts">
  import type { FileMeta } from "./types";

  let {
    openFiles,
    activeFileId,
    activeSessionType, // 'file' | 'all'
    folderPath,
    onSelect,
    onSelectAll,
    onClose,
    onCloseAll = () => {},
  }: {
    openFiles: FileMeta[];
    activeFileId: number | null;
    activeSessionType: 'file' | 'all';
    folderPath: string | null;
    onSelect: (id: number) => void;
    onSelectAll: () => void;
    onClose: (id: number) => void;
    onCloseAll?: () => void;
  } = $props();

  let folderExpanded = $state(true);

  function handleCloseAll() {
    onCloseAll();
    folderExpanded = false;
  }

  function getFileName(path: string) {
    return path.split(/[/\\]/).pop() || path;
  }

  function getFolderName(path: string) {
    return path.split(/[/\\]/).filter(Boolean).pop() || path;
  }
</script>

<aside class="sidebar">
  <div class="sidebar-content">
    {#if folderPath && openFiles.length > 0}
      <div class="section">
        <button
          class="section-item"
          class:active={activeSessionType === 'all'}
          onclick={onSelectAll}
        >
          <span class="icon">∞</span>
          <div class="item-details">
            <span class="name">All Logs</span>
            <span class="detail">{openFiles.length} files</span>
          </div>
        </button>
      </div>
    {/if}

    {#if folderPath && openFiles.length > 0}
      <div class="section">
        <div class="section-header-wrapper">
          <button
            class="section-header"
            type="button"
            onclick={() => folderExpanded = !folderExpanded}
            aria-expanded={folderExpanded}
          >
            <span class="chevron" class:expanded={folderExpanded}>▸</span>
            <span class="section-title">{getFolderName(folderPath)}</span>
          </button>
          {#if openFiles.length > 0}
            <button
              class="close-all-btn"
              type="button"
              title="Close all files"
              onclick={handleCloseAll}
            >
              ✕
            </button>
          {/if}
        </div>
        
        {#if folderExpanded}
          <ul class="file-list">
            {#each openFiles as file}
              <li class:active={activeSessionType === 'file' && file.id === activeFileId}>
                <button class="file-item" onclick={() => onSelect(file.id)}>
                  <span class="icon-file">📄</span>
                  <div class="file-details">
                    <span class="name" title={file.path}>{getFileName(file.path)}</span>
                    {#if file.last_timestamp}
                      <span class="detail">{file.last_timestamp}</span>
                    {/if}
                  </div>
                </button>
                <button class="close-btn" onclick={() => onClose(file.id)}>×</button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {:else}
      <div class="section">
        <div class="section-title-static">Open Files</div>
        <ul class="file-list">
          {#each openFiles as file}
            <li class:active={activeSessionType === 'file' && file.id === activeFileId}>
              <button class="file-item" onclick={() => onSelect(file.id)}>
                <span class="icon-file">📄</span>
                <div class="file-details">
                  <span class="name" title={file.path}>{getFileName(file.path)}</span>
                  {#if file.last_timestamp}
                    <span class="detail">{file.last_timestamp}</span>
                  {/if}
                </div>
              </button>
              <button class="close-btn" onclick={() => onClose(file.id)}>×</button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    width: 240px;
    background: var(--bg-elevated);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    user-select: none;
  }
  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px 0;
    scrollbar-gutter: stable;
  }
  .section {
    margin-bottom: 16px;
  }
  .section-header-wrapper {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 28px;
  }
  .section-header {
    display: flex;
    align-items: center;
    padding: 0 12px;
    cursor: pointer;
    gap: 6px;
    background: none;
    border: none;
    flex: 1;
    text-align: left;
    font: inherit;
    color: inherit;
    height: 100%;
  }
  .section-header:hover {
    background: var(--border-dim);
  }
  .close-all-btn {
    background: none;
    border: none;
    padding: 0 8px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text-dim);
    opacity: 0;
    transition: opacity 0.1s;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .close-all-btn:hover {
    color: var(--lv-error);
    opacity: 1;
  }
  .section-header-wrapper:hover .close-all-btn {
    opacity: 0.6;
  }
  .chevron {
    font-size: 16px;
    color: var(--text);
    transition: transform 0.2s ease;
    min-width: 16px;
    min-height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transform-origin: center;
    font-weight: 600;
  }
  .chevron.expanded {
    transform: rotate(90deg);
  }
  .section-title {
    font-size: 11px;
    font-weight: 700;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .section-title-static {
    padding: 4px 16px;
    font-size: 11px;
    font-weight: 700;
    color: var(--text-dim);
    text-transform: uppercase;
  }
  .section-item {
    width: calc(100% - 16px);
    margin: 0 8px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: none;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    color: var(--text);
  }
  .section-item:hover:not(.active) {
    background: var(--border-dim);
  }
  .section-item.active {
    background: var(--accent);
    color: white;
  }
  .icon { font-size: 18px; line-height: 1; }
  .icon-file { font-size: 14px; opacity: 0.6; margin-top: 2px; }

  .item-details, .file-details {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.3;
  }

  .file-list {
    list-style: none;
    padding: 0 8px;
    margin: 0;
  }
  li {
    display: flex;
    align-items: center;
    border-radius: 4px;
    margin-bottom: 1px;
    position: relative;
  }
  li.active {
    background: rgba(0, 113, 227, 0.25);
    border-radius: 8px;
    backdrop-filter: blur(10px);
    border: 1px solid rgba(0, 113, 227, 0.4);
  }
  li.active .name,
  li.active .detail,
  li.active .icon-file {
    color: var(--accent);
    opacity: 1;
    font-weight: 500;
  }
  li:not(.active):hover {
    background: var(--border-dim);
  }
  .file-item {
    flex: 1;
    background: none;
    border: none;
    padding: 4px 8px;
    text-align: left;
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    cursor: pointer;
    color: inherit;
  }
  .name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 400;
  }
  .detail {
    font-size: 9px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: none;
  }
  .close-btn {
    background: none;
    border: none;
    padding: 2px 8px;
    cursor: pointer;
    font-size: 16px;
    color: inherit;
    opacity: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 24px;
    flex-shrink: 0;
  }
  li:hover .close-btn {
    opacity: 0.5;
  }
  .close-btn:hover {
    opacity: 1 !important;
  }
  li.active .close-btn {
    opacity: 0.7;
  }
</style>
