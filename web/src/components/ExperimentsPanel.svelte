<script lang="ts">
  import {
    experiments,
    experimentWarnings,
    exportLoadedExperiment,
    importPortable,
    loadedExperiment,
    openExperiment,
    rerunLoadedExperiment,
    rerunResult,
    saveCurrentExperiment,
  } from '../lib/state';

  let name = $state('');
  let portableText = $state('');

  const delta = $derived.by(() => {
    const doc = $loadedExperiment;
    const rerun = $rerunResult;
    if (!doc || !rerun) return null;
    const saved = doc.session.result.estimated_pose;
    const fresh = rerun.estimated_pose;
    return Math.hypot(saved[0] - fresh[0], saved[1] - fresh[1]);
  });
</script>

<h2>Experiments</h2>
<p class="hint">Saved on the local backend; survives restarts and browser cleanup.</p>

<div class="row">
  <input type="text" bind:value={name} placeholder="experiment name" aria-label="Experiment name" />
  <button onclick={() => saveCurrentExperiment(name)} disabled={name.trim() === ''}>Save</button>
</div>

{#if $experiments.length > 0}
  <ul class="list">
    {#each $experiments as saved (saved)}
      <li>
        <button
          class:active={$loadedExperiment?.name === saved}
          onclick={() => openExperiment(saved)}
        >
          {saved}
        </button>
      </li>
    {/each}
  </ul>
{:else}
  <p class="hint">No saved experiments yet.</p>
{/if}

{#if $loadedExperiment}
  <div class="loaded">
    <div class="stat">
      <span>loaded</span><span class="mono">{$loadedExperiment.name}</span>
    </div>
    <div class="stat">
      <span>app / library</span>
      <span class="mono">{$loadedExperiment.versions.app} / {$loadedExperiment.versions.library.slice(0, 7)}</span>
    </div>
    <div class="stat">
      <span>saved observation</span>
      <span class="mono"
        >{$loadedExperiment.session.result.termination} ·
        {($loadedExperiment.session.result.estimated_pose[0]).toFixed(3)},
        {($loadedExperiment.session.result.estimated_pose[1]).toFixed(3)}</span
      >
    </div>

    <button class="wide" onclick={rerunLoadedExperiment}>Rerun from stored scans</button>

    {#if $rerunResult && delta !== null}
      <div class="stat">
        <span>rerun</span>
        <span class="mono">{$rerunResult.termination} · pose Δ {delta.toExponential(2)} m</span>
      </div>
      <p class="hint">
        The saved observation and this rerun are kept separate. A nonzero Δ or a different
        termination means the rerun does not reproduce the stored snapshot.
      </p>
    {/if}

    {#if $experimentWarnings.length > 0}
      <ul class="warnings" role="alert">
        {#each $experimentWarnings as warning (warning)}
          <li>{warning}</li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

  <h2>Portable document</h2>
  <div class="row">
    <button
      onclick={() => (portableText = exportLoadedExperiment())}
      disabled={!$loadedExperiment}
    >
      Export
    </button>
    <button onclick={() => importPortable(portableText)} disabled={portableText.trim() === ''}>
      Import
    </button>
  </div>
  <textarea
    bind:value={portableText}
    rows="5"
    spellcheck="false"
    placeholder="Paste a portable experiment document"
    aria-label="Portable experiment document"
  ></textarea>

<style>
  h2 {
    margin: 14px 0 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }

  .hint {
    margin: 2px 0 6px;
    font-size: 12px;
    color: var(--muted);
  }

  .row {
    display: flex;
    gap: 6px;
  }

  .row input {
    flex: 1;
  }

  .list {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: grid;
    gap: 4px;
  }

  .list button {
    width: 100%;
    text-align: left;
  }

  .list button.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, var(--surface-2));
  }

  .loaded {
    margin-top: 8px;
    border-top: 1px solid var(--line);
    padding-top: 6px;
  }

  .stat {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
    padding: 2px 0;
  }

  .stat .mono {
    color: var(--text);
    text-align: right;
  }

  .wide {
    width: 100%;
    margin: 6px 0;
  }

  .warnings {
    margin: 6px 0 0;
    padding: 6px 8px 6px 22px;
    list-style: disc;
    border: 1px solid var(--raw);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--raw) 12%, var(--surface));
    font-size: 11px;
    color: var(--text);
  }

  textarea {
    width: 100%;
    margin-top: 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius);
    padding: 6px;
    resize: vertical;
  }
</style>