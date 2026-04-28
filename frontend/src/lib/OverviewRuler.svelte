<script lang="ts">
  import { onMount } from "svelte";
  import type { LogLevel } from "./types";

  let {
    lineCount,
    searchMatches = [],
    bookmarks = [],
    levelMarkers = [],
    scrollFraction = 0,
    viewFraction = 0.1,
    theme = 'system',
    onSeek = (_line: number) => {},
  }: {
    lineCount: number;
    searchMatches?: number[];
    bookmarks?: number[];
    levelMarkers?: { line: number; level: LogLevel }[];
    scrollFraction?: number;
    viewFraction?: number;
    theme?: 'light' | 'dark' | 'system';
    onSeek?: (line: number) => void;
  } = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let width = $state(0);
  let height = $state(24);
  let resizeObserver: ResizeObserver | undefined;

  function redraw() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    // Determine theme for colors
    let isDark = theme === 'dark';
    if (theme === 'system') {
      isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    }

    // Background
    ctx.fillStyle = 'var(--bg-elevated)';
    ctx.fillRect(0, 0, width, height);

    // Border
    ctx.strokeStyle = 'var(--border)';
    ctx.lineWidth = 1;
    ctx.strokeRect(0, 0, width, height);

    if (lineCount === 0) return;

    // Create maps for quick lookup
    const levelMap = new Map<number, LogLevel>();
    for (const marker of levelMarkers) {
      levelMap.set(marker.line, marker.level);
    }

    const bookmarkSet = new Set(bookmarks);
    const matchSet = new Set(searchMatches);

    // Draw: horizontal strip where each column represents a line
    for (let x = 0; x < width; x++) {
      const lineNum = Math.floor((x / width) * lineCount);
      if (lineNum >= lineCount) break;

      let color = isDark ? 'rgb(51, 51, 51)' : 'rgb(200, 200, 200)'; // default dim

      // Check level
      if (levelMap.has(lineNum)) {
        const level = levelMap.get(lineNum)!;
        if (level === 'error') {
          color = isDark ? 'rgb(255, 59, 48)' : 'rgb(255, 95, 80)'; // red
        } else if (level === 'fatal') {
          color = isDark ? 'rgb(191, 90, 242)' : 'rgb(200, 130, 255)'; // purple
        } else if (level === 'warn') {
          color = isDark ? 'rgb(255, 159, 10)' : 'rgb(255, 180, 60)'; // orange
        }
      }

      ctx.fillStyle = color;
      ctx.fillRect(x, 0, 1, height); // full height bar
    }

    // Draw search matches as blue overlay
    ctx.fillStyle = isDark ? 'rgba(0, 113, 227, 0.6)' : 'rgba(0, 113, 227, 0.4)';
    for (const match of searchMatches) {
      const x = (match / lineCount) * width;
      ctx.fillRect(x, 0, 1, height);
    }

    // Draw bookmarks as gold markers
    ctx.fillStyle = isDark ? 'rgb(255, 215, 0)' : 'rgb(255, 195, 0)';
    for (const line of bookmarks) {
      const x = (line / lineCount) * width;
      ctx.fillRect(x, 0, 2, height);
    }
  }

  $effect(() => {
    redraw();
  });

  function handleClick(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const line = Math.floor((x / width) * lineCount);
    onSeek(Math.max(0, Math.min(line, lineCount - 1)));
  }

  function handleResize() {
    if (canvas) {
      width = canvas.offsetWidth;
      redraw();
    }
  }

  onMount(() => {
    if (!canvas) return;

    // Initialize width
    width = canvas.offsetWidth;
    redraw();

    // Watch for size changes
    resizeObserver = new ResizeObserver(() => {
      if (canvas) {
        width = canvas.offsetWidth;
        redraw();
      }
    });
    resizeObserver.observe(canvas);

    return () => {
      resizeObserver?.disconnect();
    };
  });

  $effect(() => {
    // Redraw when props change
    void lineCount;
    void searchMatches;
    void bookmarks;
    void levelMarkers;
    void theme;
    redraw();
  });
</script>

<canvas
  bind:this={canvas}
  {height}
  class="overview-ruler"
  onclick={handleClick}
  onmousemove={handleResize}
></canvas>

<style>
  .overview-ruler {
    width: 100%;
    height: 24px;
    display: block;
    cursor: pointer;
    border-bottom: 1px solid var(--border);
  }
</style>
