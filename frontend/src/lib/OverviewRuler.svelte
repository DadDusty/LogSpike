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

  function getComputedColor(variable: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(variable).trim();
  }

  function redraw() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    // Get colors from CSS variables
    const bgColor = getComputedColor('--color-filter-bar-bg');
    const borderColor = getComputedColor('--color-border');
    const errorColor = getComputedColor('--color-level-error-badge');
    const criticalColor = getComputedColor('--color-level-critical-badge');
    const warningColor = getComputedColor('--color-level-warning-badge');
    const searchColor = getComputedColor('--color-search-highlight-active');
    const followingColor = getComputedColor('--color-following');

    // Background
    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, width, height);

    // Border
    ctx.strokeStyle = borderColor;
    ctx.lineWidth = 1;
    ctx.strokeRect(0, 0, width, height);

    if (lineCount === 0) return;

    // Create maps for quick lookup
    const levelMap = new Map<number, LogLevel>();
    for (const marker of levelMarkers) {
      levelMap.set(marker.line, marker.level);
    }

    // Draw: horizontal strip where each column represents a line
    for (let x = 0; x < width; x++) {
      const lineNum = Math.floor((x / width) * lineCount);
      if (lineNum >= lineCount) break;

      let color = '#666666';

      // Check level
      if (levelMap.has(lineNum)) {
        const level = levelMap.get(lineNum)!;
        if (level === 'fatal') {
          color = criticalColor;
        } else if (level === 'error') {
          color = errorColor;
        } else if (level === 'warn') {
          color = warningColor;
        }
      }

      ctx.fillStyle = color;
      ctx.fillRect(x, 0, 1, height);
    }

    // Draw search matches as highlight overlay
    ctx.fillStyle = searchColor;
    for (const match of searchMatches) {
      const x = (match / lineCount) * width;
      ctx.fillRect(x, 0, 1, height);
    }

    // Draw bookmarks as bright markers
    ctx.fillStyle = followingColor;
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
    border-bottom: 1px solid var(--color-border);
  }
</style>
