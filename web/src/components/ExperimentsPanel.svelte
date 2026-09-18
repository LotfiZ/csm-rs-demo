<script lang="ts">
  import {
    experiments,
    experimentWarnings,
    exportLoadedExperiment,
    importMode,
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

<section class="saved-section" aria-labelledby="saved-runs-heading">
  <h3 id="saved-runs-heading">Saved runs</h3>
  <p class="hint">Save a generated setup and its result so you can load or replay it later.</p>

  {#if $importMode}
    <p class="notice">Saved runs currently support generated examples. Choose an example above before saving.</p>
  {:else}
    <div class="row">
      <input
        type="text"
        bind:value={name}
        placeholder="name, e.g. noisy-room"
        aria-label="Saved run name"
      />
      <button onclick={() => saveCurrentExperiment(name)} disabled={name.trim() === ''}>Save run</button>
    </div>
  {/if}

  {#if $experiments.length > 0}
    <ul class="list" aria-label="Saved runs">
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
    <p class="hint">No saved runs yet.</p>
  {/if}

  {#if $loadedExperiment}
    <div class="loaded">
      <div class="loaded-heading">
        <span>Loaded run</span>
        <strong class="mono">{$loadedExperiment.name}</strong>
      </div>
      <div class="stat">
        <span>termination</span>
        <span class="mono">{$loadedExperiment.session.result.termination}</span>
      </div>
      <div class="stat">
        <span>estimated position</span>
        <span class="mono"
          >{($loadedExperiment.session.result.estimated_pose[0]).toFixed(3)},
          {($loadedExperiment.session.result.estimated_pose[1]).toFixed(3)} m</span
        >
      </div>

      <button class="wide" onclick={rerunLoadedExperiment}>Replay stored scans</button>

      {#if $rerunResult && delta !== null}
        <div class="stat">
          <span>replay result</span>
          <span class="mono">{$rerunResult.termination} · position change {delta.toExponential(2)} m</span>
        </div>
        <p class="hint">
          Replay uses the stored scans again. A position difference or different termination means
          the saved result did not reproduce exactly.
        </p>
      {/if}

      {#if $experimentWarnings.length > 0}
        <ul class="warnings" role="alert">
          {#each $experimentWarnings as warning (warning)}
            <li>{warning}</li>
          {/each}
        </ul>
      {/if}

      <details class="metadata">
        <summary>Technical details</summary>
        <div class="stat">
          <span>app / library</span>
          <span class="mono">{$loadedExperiment.versions.app} / {$loadedExperiment.versions.library.slice(0, 7)}</span>
        </div>
      </details>
    </div>
  {/if}
</section>

<section class="portable" aria-labelledby="portable-heading">
  <h3 id="portable-heading">Share or back up</h3>
  <p class="hint">Export a saved run as JSON, or paste one from another machine.</p>
  <div class="row">
    <button
      onclick={() => (portableText = exportLoadedExperiment())}
      disabled={!$loadedExperiment}
    >
      Export JSON
    </button>
    <button onclick={() => importPortable(portableText)} disabled={portableText.trim() === ''}>
      Import JSON
    </button>
  </div>
  <textarea
    bind:value={portableText}
    rows="5"
    spellcheck="false"
    placeholder="Paste saved run JSON here"
    aria-label="Saved run JSON"
  ></textarea>
</section>

<style>
  h3 {
    margin: 0 0 4px;
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
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
    min-width: 0;
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

  .loaded-heading {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 0 4px;
    color: var(--muted);
    font-size: 11px;
  }

  .loaded-heading strong {
    overflow: hidden;
    color: var(--text);
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .notice {
    margin: 7px 0 0;
    border-left: 2px solid var(--line-strong);
    padding: 3px 0 3px 8px;
    color: var(--muted);
    font-size: 11px;
    line-height: 1.4;
  }

  .warnings {
    margin: 6px 0 0;
    padding: 6px 8px 6px 22px;
    list-style: disc;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--line-strong) 12%, var(--surface));
    font-size: 11px;
    color: var(--text);
  }

  .metadata {
    margin-top: 8px;
    border-top: 1px solid var(--line);
    padding-top: 7px;
  }

  .metadata summary {
    color: var(--muted);
    cursor: pointer;
    font-size: 11px;
  }

  .metadata .stat {
    margin-top: 4px;
  }

  .portable {
    margin-top: 16px;
    border-top: 1px solid var(--line);
    padding-top: 12px;
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
