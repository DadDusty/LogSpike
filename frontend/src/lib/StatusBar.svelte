<script lang="ts">
  let {
    lineCount,
    status,
    following,
    onToggleFollow,
    onJumpToEnd,
  }: {
    lineCount: number;
    status: string;
    following: boolean;
    onToggleFollow: () => void;
    onJumpToEnd: () => void;
  } = $props();
</script>

<footer class="status-bar">
  <div class="left">
    <span class="status-msg">{status || 'Ready'}</span>
  </div>

  <div class="right">
    <span class="stat">UTF-8</span>
    <span class="stat">{lineCount.toLocaleString()} lines</span>
    <div class="divider"></div>
    <button class="action-btn" class:active={following} onclick={onToggleFollow}>
      {following ? 'Following ●' : 'Follow'}
    </button>
    <button class="action-btn" onclick={onJumpToEnd}>Jump to End</button>
  </div>
</footer>

<style>
  .status-bar {
    height: 28px;
    background: var(--bg-elevated);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    font-size: 11px;
    color: var(--text-dim);
    flex-shrink: 0;
  }
  .left, .right {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .status-msg {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 400px;
  }
  .stat {
    font-weight: 500;
  }
  .divider {
    width: 1px;
    height: 14px;
    background: var(--border);
  }
  .action-btn {
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 1px 8px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    color: var(--text);
  }
  .action-btn:hover {
    background: var(--border-dim);
  }
  .action-btn.active {
    color: var(--accent);
    background: var(--accent-dim);
  }
</style>
