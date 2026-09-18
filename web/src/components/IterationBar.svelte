<script lang="ts">
  import {
    activeFrame,
    canRunNextStep,
    matcher,
    resetStep,
    result,
    run,
    running,
    issues,
    stepMode,
    stepTarget,
    traceIteration,
  } from '../lib/state';

  const iterations = $derived($activeFrame?.trace ?? []);
  const count = $derived(iterations.length);
  const selected = $derived(Math.min($traceIteration, Math.max(0, count - 1)));
  const iteration = $derived(iterations[selected] ?? null);
  const nextIteration = $derived($stepTarget + 1);
  const nextLabel = $derived(
    !$result ? 'Run first iteration' : $canRunNextStep ? `Run iteration ${String(nextIteration).padStart(2, '0')}` : 'Iteration limit reached',
  );
  let open = $state(true);

  function inspect(delta: number) {
    traceIteration.set(Math.min(Math.max(0, selected + delta), Math.max(0, count - 1)));
  }
</script>

{#if $stepMode}
  <section class="bar" aria-label="Solver iteration stepper">
    <div class="header">
      <div class="copy">
        <span class="eyebrow">Solver stepper</span>
        <strong>
          {#if iteration}
            Iteration {selected + 1} of {count}
          {:else}
            No iterations yet
          {/if}
        </strong>
        {#if open}
          <p>Ask for one more solver iteration when you are ready. The plot follows the selected iteration.</p>
        {/if}
      </div>

      <div class="actions">
        <button class="primary" onclick={run} disabled={$running || $issues.length > 0 || !$canRunNextStep}>{nextLabel}</button>
        <button class="secondary" onclick={resetStep} disabled={$running || !$result}>Reset</button>
        <button
          class="collapse"
          onclick={() => (open = !open)}
          aria-expanded={open}
          aria-controls="solver-stepper-details"
        >
          {open ? 'Collapse' : 'Expand'}
        </button>
      </div>
    </div>

    {#if open}
      <div id="solver-stepper-details" class="details">
        {#if count > 0}
          <div class="inspect">
            <button
              class="nudge"
              onclick={() => inspect(-1)}
              disabled={selected === 0}
              aria-label="Inspect previous iteration"
            >
              ←
            </button>
            <input
              type="range"
              min="0"
              max={Math.max(0, count - 1)}
              step="1"
              value={selected}
              oninput={(event) => traceIteration.set(Number(event.currentTarget.value))}
              aria-label="Inspect matcher iteration"
            />
            <button
              class="nudge"
              onclick={() => inspect(1)}
              disabled={selected >= count - 1}
              aria-label="Inspect next iteration"
            >
              →
            </button>
            <span class="readout mono">
              inspect {selected + 1}/{count}
              {#if iteration} · residual {iteration.error.toFixed(4)} · {iteration.valid_correspondences} correspondences{/if}
            </span>
          </div>
        {/if}

        {#if $result && !$canRunNextStep}
          <p class="stop-note">
            {$result.termination === 'IterationLimit'
              ? `Reached the ${Math.max(1, Math.floor($matcher.max_iterations))}-iteration limit.`
              : `The matcher stopped after ${$result.iterations} iteration${$result.iterations === 1 ? '' : 's'}.`}
          </p>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<style>
  .bar {
    padding: 11px 20px;
    background: var(--surface);
  }

  .header {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) auto;
    align-items: center;
    gap: 10px 24px;
  }

  .copy {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .eyebrow {
    color: var(--muted);
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  strong {
    font-size: 13px;
    font-weight: 500;
  }

  p {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
    line-height: 1.4;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 7px;
    min-width: 0;
  }

  button {
    min-height: 32px;
    font-size: 11px;
  }

  .primary {
    border-color: var(--ink);
    background: var(--ink);
    color: var(--surface);
  }

  .secondary,
  .collapse {
    background: transparent;
  }

  .collapse {
    color: var(--muted);
  }

  .details {
    display: grid;
    gap: 8px;
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--line);
  }

  .inspect {
    display: grid;
    grid-template-columns: auto minmax(120px, 360px) auto minmax(0, 1fr);
    align-items: center;
    gap: 8px;
  }

  .nudge {
    width: 32px;
    padding: 4px 7px;
    background: transparent;
  }

  input[type='range'] {
    width: 100%;
  }

  .readout {
    min-width: 0;
    color: var(--muted);
    font-size: 10px;
    overflow-wrap: anywhere;
  }

  .stop-note {
    padding-top: 0;
  }

  @media (max-width: 620px) {
    .bar {
      padding-inline: 14px;
    }

    .header {
      grid-template-columns: 1fr;
    }

    .actions {
      justify-content: flex-start;
    }

    .inspect {
      grid-template-columns: auto minmax(0, 1fr) auto;
    }

    .readout {
      grid-column: 1 / -1;
    }
  }
</style>
