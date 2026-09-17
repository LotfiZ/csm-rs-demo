<script lang="ts">
  import BottomPanel from './components/BottomPanel.svelte';
  import BenchmarkPanel from './components/BenchmarkPanel.svelte';
  import DiagnosticsPanel from './components/DiagnosticsPanel.svelte';
  import LeftPanel from './components/LeftPanel.svelte';
  import MatcherPanel from './components/MatcherPanel.svelte';
  import Plot from './components/Plot.svelte';
  import SequenceBar from './components/SequenceBar.svelte';
  import { error, initTheme, issues, outdated, run, running, theme, toggleTheme, abMode, setAbMode, sequenceMode, setSequenceMode, matcher, matcherB, editingSide, activeFrame } from './lib/state';
  import { onMount } from 'svelte';

  onMount(() => {
    initTheme();
    // Show a working example immediately; Run stays explicit thereafter.
    run();
  });
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
      class="ab"
      class:active={$abMode}
      aria-pressed={$abMode}
      onclick={() => setAbMode(!$abMode)}
    >
      A/B
    </button>
    <button
      class="ab"
      class:active={$sequenceMode}
      aria-pressed={$sequenceMode}
      onclick={() => setSequenceMode(!$sequenceMode)}
    >
      Sequence
    </button>
    <button class="theme" onclick={toggleTheme} aria-label="Toggle theme">
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
    <LeftPanel />
    <div class="plot-cell"><Plot /></div>
    <MatcherPanel target={$abMode && $editingSide === 'B' ? matcherB : matcher} />
  </div>

  <div class="bottom-area">
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
    gap: 10px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
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

  .ab.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 18%, var(--surface-2));
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

  .bottom-area {
    flex: 0 0 auto;
  }

  .body {
    flex: 1;
    display: grid;
    grid-template-columns: 264px minmax(0, 1fr) 264px;
    min-height: 0;
  }

  .plot-cell {
    min-width: 0;
    min-height: 0;
    background: var(--plot);
  }

  @media (max-width: 900px) {
    .body {
      grid-template-columns: 1fr;
      grid-auto-rows: min-content;
      overflow-y: auto;
    }

    .plot-cell {
      height: 60vh;
      order: -1;
    }
  }
</style>