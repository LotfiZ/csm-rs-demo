<script lang="ts">
  import {
    cancelSequence,
    outdated,
    pauseSequence,
    playSequence,
    playing,
    resetSequence,
    sequence,
    sequenceIndex,
    sequenceProgress,
    running,
    stepSequence,
  } from '../lib/state';

  const frameCount = $derived($sequence?.frames.length ?? 0);
  const frame = $derived($sequence ? $sequence.frames[$sequenceIndex] : null);
  const rejected = $derived(frame ? !frame.accepted : false);
</script>

{#if $sequenceProgress}
  <div class="bar running">
    <span>Running sequence… {$sequenceProgress.done} / {$sequenceProgress.total}</span>
    <progress value={$sequenceProgress.done} max={$sequenceProgress.total}></progress>
    <button onclick={cancelSequence}>Cancel</button>
  </div>
{:else if $sequence}
  <div class="bar">
    <span class="label">Sequence</span>
    <button onclick={() => ($playing ? pauseSequence() : playSequence())} disabled={$running}>
      {$playing ? 'Pause' : 'Play'}
    </button>
    <button onclick={() => stepSequence(-1)} disabled={$running || $sequenceIndex === 0}>◀</button>
    <button onclick={() => stepSequence(1)} disabled={$running || $sequenceIndex >= frameCount - 1}>
      ▶
    </button>
    <button onclick={resetSequence} disabled={$running}>Reset</button>

    <input
      class="scrub"
      type="range"
      min="0"
      max={Math.max(0, frameCount - 1)}
      step="1"
      value={$sequenceIndex}
      oninput={(event) => sequenceIndex.set(Number(event.currentTarget.value))}
      aria-label="Sequence frame"
    />

    <span class="readout mono">
      frame {$sequenceIndex + 1} / {frameCount}
      {#if frame}
        · step {frame.step} · {frame.termination}
      {/if}
    </span>

    {#if rejected}
      <span class="rejected">update rejected — trajectory held</span>
    {/if}
    {#if $outdated}
      <span class="rejected">inputs changed — sequence outdated</span>
    {/if}
  </div>
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    border-top: 1px solid var(--line);
    background: var(--surface-2);
    font-size: 12px;
    flex-wrap: wrap;
  }

  .label {
    color: var(--muted);
    font-weight: 600;
  }

  .scrub {
    flex: 1;
    min-width: 120px;
    max-width: 360px;
  }

  .readout {
    color: var(--muted);
  }

  .rejected {
    color: var(--danger);
    font-weight: 500;
  }

  progress {
    width: 160px;
  }
</style>