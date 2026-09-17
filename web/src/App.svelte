<script lang="ts">
  import BottomPanel from './components/BottomPanel.svelte';
  import BenchmarkPanel from './components/BenchmarkPanel.svelte';
  import DiagnosticsPanel from './components/DiagnosticsPanel.svelte';
  import LeftPanel from './components/LeftPanel.svelte';
  import MatcherPanel from './components/MatcherPanel.svelte';
  import Plot from './components/Plot.svelte';
  import SequenceBar from './components/SequenceBar.svelte';
  import {
    abMode,
    activeFrame,
    editingSide,
    error,
    initLayout,
    initTheme,
    issues,
    layout,
    matcher,
    matcherB,
    outdated,
    refreshExperiments,
    run,
    running,
    sequenceMode,
    setAbMode,
    setSequenceMode,
    theme,
    toggleTheme,
  } from './lib/state';
  import { onMount } from 'svelte';

  onMount(() => {
    initTheme();
    initLayout();
    refreshExperiments();
    // Show a working example immediately; Run stays explicit thereafter.
    run();
  });

  type Axis = 'left' | 'right' | 'bottom';

  function nudge(axis: Axis, delta: number) {
    layout.update((current) => {
      if (axis === 'left') return { ...current, left: clamp(current.left + delta, 180, 520) };
      if (axis === 'right') return { ...current, right: clamp(current.right - delta, 180, 520) };
      return { ...current, bottom: clamp(current.bottom - delta, 140, 640) };
    });
  }

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value));
  }

  let resizing: Axis | null = null;
  let start = 0;
  let startSize = 0;

  function startResize(axis: Axis, event: PointerEvent) {
    event.preventDefault();
    resizing = axis;
    start = axis === 'bottom' ? event.clientY : event.clientX;
    startSize = axis === 'left' ? $layout.left : axis === 'right' ? $layout.right : $layout.bottom;
    window.addEventListener('pointermove', onResize);
    window.addEventListener('pointerup', endResize);
  }

  function onResize(event: PointerEvent) {
    if (!resizing) return;
    const delta = (resizing === 'bottom' ? event.clientY : event.clientX) - start;
    layout.update((current) => {
      if (resizing === 'left') return { ...current, left: clamp(startSize + delta, 180, 520) };
      if (resizing === 'right') return { ...current, right: clamp(startSize - delta, 180, 520) };
      return { ...current, bottom: clamp(startSize - delta, 140, 640) };
    });
  }

  function endResize() {
    resizing = null;
    window.removeEventListener('pointermove', onResize);
    window.removeEventListener('pointerup', endResize);
  }

  function handleKey(axis: Axis, event: KeyboardEvent) {
    const step = 16;
    if (event.key === 'ArrowLeft') nudge(axis, -step);
    else if (event.key === 'ArrowRight') nudge(axis, step);
    else if (event.key === 'ArrowUp') nudge(axis, -step);
    else if (event.key === 'ArrowDown') nudge(axis, step);
    else return;
    event.preventDefault();
  }
</script>

<div class="app">
  <header>
    <div class="title">
      <h1>csm-rs workbench</h1>
      <span class="sub">scan-matching experiments on the real library</span>
    </div>

    <div class="spacer"></div>

    {#if $outdated}
      <span class="badge">inputs changed — results outdated</span>
    {/if}

    <button
      class="toggle"
      class:active={!$layout.leftCollapsed}
      aria-pressed={!$layout.leftCollapsed}
      onclick={() => layout.update((l) => ({ ...l, leftCollapsed: !l.leftCollapsed }))}
    >
      Examples
    </button>
    <button
      class="toggle"
      class:active={!$layout.rightCollapsed}
      aria-pressed={!$layout.rightCollapsed}
      onclick={() => layout.update((l) => ({ ...l, rightCollapsed: !l.rightCollapsed }))}
    >
      Settings
    </button>
    <button
      class="toggle"
      class:active={$abMode}
      aria-pressed={$abMode}
      onclick={() => setAbMode(!$abMode)}
    >
      A/B
    </button>
    <button
      class="toggle"
      class:active={$sequenceMode}
      aria-pressed={$sequenceMode}
      onclick={() => setSequenceMode(!$sequenceMode)}
    >
      Sequence
    </button>
    <button class="toggle" onclick={toggleTheme} aria-label="Toggle light and dark theme">
      {$theme === 'dark' ? 'Light' : 'Dark'}
    </button>
    <button class="run" onclick={run} disabled={$running || $issues.length > 0}>
      {$running ? 'Running…' : $abMode ? 'Run A/B' : $sequenceMode ? 'Run sequence' : 'Run match'}
    </button>
  </header>

  {#if $error}
    <p class="error" role="alert">{$error}</p>
  {/if}

  <div class="body">
    {#if !$layout.leftCollapsed}
      <div class="side" style="width: {$layout.left}px"><LeftPanel /></div>
    {/if}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
    <div
      class="vhandle"
      role="separator"
      tabindex="0"
      aria-orientation="vertical"
      aria-label="Resize examples panel"
      onpointerdown={(event) => startResize('left', event)}
      onkeydown={(event) => handleKey('left', event)}
    ></div>

    <div class="plot-cell"><Plot /></div>

    <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
    <div
      class="vhandle"
      role="separator"
      tabindex="0"
      aria-orientation="vertical"
      aria-label="Resize settings panel"
      onpointerdown={(event) => startResize('right', event)}
      onkeydown={(event) => handleKey('right', event)}
    ></div>
    {#if !$layout.rightCollapsed}
      <div class="side" style="width: {$layout.right}px">
        <MatcherPanel target={$abMode && $editingSide === 'B' ? matcherB : matcher} />
      </div>
    {/if}
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
  <div
    class="hhandle"
    role="separator"
    tabindex="0"
    aria-orientation="horizontal"
    aria-label="Resize diagnostics"
    onpointerdown={(event) => startResize('bottom', event)}
    onkeydown={(event) => handleKey('bottom', event)}
  ></div>

  <div class="bottom-area" style="max-height: {$layout.bottom}px">
    {#if $sequenceMode}<SequenceBar />{/if}
    {#if $abMode}<BenchmarkPanel />{/if}
    {#if $activeFrame?.trace && $activeFrame.trace.length > 0}<DiagnosticsPanel />{/if}
    <BottomPanel />
  </div>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    min-height: 0;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
    flex-wrap: wrap;
  }

  .title h1 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  .sub {
    font-size: 12px;
    color: var(--muted);
  }

  .spacer {
    flex: 1;
  }

  .badge {
    font-size: 12px;
    color: var(--accent-contrast);
    background: var(--raw);
    border-radius: var(--radius);
    padding: 2px 8px;
  }

  .toggle.active {
    border-color: var(--line-strong);
    background: var(--surface-2);
    font-weight: 600;
  }

  .run {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }

  .error {
    margin: 0;
    padding: 6px 14px;
    background: color-mix(in srgb, var(--danger) 18%, var(--surface));
    color: var(--text);
    border-bottom: 1px solid var(--line);
    font-size: 13px;
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .side {
    min-height: 0;
    overflow-y: auto;
  }

  .plot-cell {
    flex: 1;
    min-width: 0;
    min-height: 0;
    background: var(--plot);
  }

  .vhandle {
    flex: 0 0 6px;
    cursor: col-resize;
    background: var(--surface-2);
    border-left: 1px solid var(--line);
    border-right: 1px solid var(--line);
  }

  .hhandle {
    flex: 0 0 6px;
    cursor: row-resize;
    background: var(--surface-2);
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }

  .vhandle:hover,
  .hhandle:hover,
  .vhandle:focus-visible,
  .hhandle:focus-visible {
    background: var(--accent);
    outline: none;
  }

  .bottom-area {
    flex: 0 0 auto;
    overflow-y: auto;
  }

  @media (max-width: 900px) {
    .body {
      flex-direction: column;
      overflow-y: auto;
    }

    .side {
      width: auto !important;
      max-height: 40vh;
      overflow-y: auto;
      border: none;
    }

    .plot-cell {
      height: 50vh;
      flex: 0 0 auto;
      order: -1;
    }

    .vhandle {
      display: none;
    }
  }
</style>