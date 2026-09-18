<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import BenchmarkPanel from './components/BenchmarkPanel.svelte';
  import BottomPanel from './components/BottomPanel.svelte';
  import DiagnosticsPanel from './components/DiagnosticsPanel.svelte';
  import LeftPanel from './components/LeftPanel.svelte';
  import MatcherPanel from './components/MatcherPanel.svelte';
  import IterationBar from './components/IterationBar.svelte';
  import Plot from './components/Plot.svelte';
  import {
    abMode,
    activeFrame,
    canRunNextStep,
    compare,
    diagnosticsOpen,
    editingSide,
    error,
    generation,
    imported,
    importMode,
    initTheme,
    issues,
    matcher,
    matcherB,
    outdated,
    placed,
    placing,
    refreshExperiments,
    refreshPreview,
    referenceMode,
    resetPlacement,
    result,
    run,
    running,
    setAbMode,
    setPlacing,
    setStepMode,
    stepMode,
    stepTarget,
    theme,
    toggleTheme,
    trace,
  } from './lib/state';

  type StepId = 'prepare' | 'place' | 'match' | 'inspect';

  type Step = {
    id: StepId;
    label: string;
    title: string;
    description: string;
    detail: string;
  };

  const steps: Step[] = [
    {
      id: 'prepare',
      label: 'Prepare',
      title: 'Prepare the scene',
      description: 'Choose an example and shape the generated scans.',
      detail: 'Choose a scene',
    },
    {
      id: 'place',
      label: 'Place',
      title: 'Place the scan',
      description: 'Set the starting pose by moving the sensor on the plot.',
      detail: 'Set the initial pose',
    },
    {
      id: 'match',
      label: 'Match',
      title: 'Run the matcher',
      description: 'Choose a run mode and start the solver when the inputs are ready.',
      detail: 'Run the solver',
    },
    {
      id: 'inspect',
      label: 'Inspect',
      title: 'Inspect the result',
      description: 'Read the outcome, accuracy, and solver diagnostics.',
      detail: 'Read the result',
    },
  ];

  let activeStep = $state<StepId>('prepare');

  const stage = $derived(steps.find((step) => step.id === activeStep) ?? steps[0]);
  const stageNumber = $derived(steps.findIndex((step) => step.id === activeStep) + 1);
  const hasResult = $derived(Boolean($activeFrame || $compare || ($importMode && $imported)));
  const resultLabel = $derived.by(() => {
    if ($activeFrame?.accepted) return 'accepted';
    if ($activeFrame?.valid) return 'valid candidate';
    if ($activeFrame) return 'failed candidate';
    if ($compare) return 'comparison ready';
    if ($imported) return 'imported result';
    return 'waiting for a run';
  });
  const runLabel = $derived.by(() => {
    if ($running) return $stepMode ? 'Running iteration…' : 'Running…';
    if ($stepMode) {
      if (!$activeFrame) return 'Run first iteration';
      if (!$canRunNextStep) return 'Iteration limit reached';
      return `Run iteration ${String($stepTarget + 1).padStart(2, '0')}`;
    }
    if ($abMode) return 'Run A/B';
    return 'Run match';
  });

  function chooseStep(step: StepId) {
    activeStep = step;
    setPlacing(step === 'place');
  }

  function toggleAb() {
    setAbMode(!$abMode);
    activeStep = 'match';
  }

  function toggleStep() {
    setStepMode(!$stepMode);
    activeStep = 'match';
  }

  async function runCurrent() {
    activeStep = 'match';
    await run();
    if (!$stepMode && (get(result) || get(compare))) activeStep = 'inspect';
  }

  onMount(() => {
    initTheme();
    refreshExperiments();
    void refreshPreview();
    // The initial canvas is a generated preview. Matching remains explicit.
  });
</script>

<svelte:head>
  <title>csm-rs · scan matching lab</title>
</svelte:head>

<div class="app">
  <header class="topbar">
    <div class="brand" aria-label="csm-rs scan matching lab">
      <span class="brand-mark">csm</span>
      <span class="brand-slash">/</span>
      <span class="brand-context">scan matching lab</span>
    </div>

    <div class="top-status" aria-live="polite">
      <span class:busy={$running} class="status-dot"></span>
      <span>
        {$running ? 'Working' : $outdated ? 'Inputs changed' : hasResult ? 'Result ready' : 'Ready to run'}
      </span>
    </div>

    <div class="top-actions">
      <button class="utility" class:active={$abMode} aria-pressed={$abMode} onclick={toggleAb}>
        A/B
      </button>
      <button class="utility" class:active={$stepMode} aria-pressed={$stepMode} onclick={toggleStep}>
        Step through
      </button>
      <button
        class="utility theme-button"
        onclick={toggleTheme}
        aria-label={$theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
      >
        {$theme === 'dark' ? 'Light mode' : 'Dark mode'}
      </button>
    </div>
  </header>

  {#if $error}
    <p class="error" role="alert">{$error}</p>
  {/if}

  <div class="workspace">
    <nav class="rail" aria-label="Workflow">
      <div class="rail-heading">
        <span>Workflow</span>
        <span class="rail-count mono">4 steps</span>
      </div>

      <ol class="steps">
        {#each steps as step, index (step.id)}
          <li>
            <button
              class="step-button"
              class:current={activeStep === step.id}
              class:complete={
                (step.id === 'place' && $placed) ||
                (step.id === 'match' && hasResult) ||
                (step.id === 'inspect' && hasResult)
              }
              aria-current={activeStep === step.id ? 'step' : undefined}
              onclick={() => chooseStep(step.id)}
            >
              <span class="step-number mono">0{index + 1}</span>
              <span class="step-copy">
                <strong>{step.label}</strong>
                <small>{step.detail}</small>
              </span>
              {#if step.id === 'place' && $placed}
                <span class="step-check" aria-label="complete">✓</span>
              {:else if step.id === 'match' && hasResult}
                <span class="step-check" aria-label="complete">✓</span>
              {/if}
            </button>
          </li>
        {/each}
      </ol>

      <div class="rail-note">
        <span class="rail-note-mark" aria-hidden="true"></span>
        <p>Move the scan, then decide when to match.</p>
      </div>
    </nav>

    <main class="stage">
      <div class="stage-head">
        <div class="stage-copy">
          <p class="step-caption"><span class="step-marker mono">0{stageNumber}</span>{stage.label}</p>
          <h1>{stage.title}</h1>
          <p>{stage.description}</p>
        </div>
        <div class="stage-state">
          {#if $running}
            <span class="state-pill busy">running</span>
          {:else if $outdated}
            <span class="state-pill warn">needs a run</span>
          {:else if hasResult}
            <span class="state-pill ready">{resultLabel}</span>
          {:else}
            <span class="state-pill">preview</span>
          {/if}
          {#if $placed}
            <span class="pose-state">manual pose set</span>
          {/if}
        </div>
      </div>

      <div class="plot-frame">
        <Plot />
      </div>

      <div class="stage-footer">
        <p><span class="mono">drag</span> pan · <span class="mono">scroll</span> zoom · <span class="mono">0</span> reset</p>
        {#if $outdated}
          <span class="footer-alert">Inputs changed. Run when ready.</span>
        {/if}
      </div>
    </main>

    <aside class="inspector" aria-label="Current workflow controls">
      {#if activeStep === 'prepare'}
        <div class="inspector-scroll">
          <div class="inspector-heading">
            <p class="step-caption">Step 01</p>
            <h2>Prepare the scene</h2>
            <p>Start with a known example, then adjust only what you want to study.</p>
          </div>
          <LeftPanel />
          <div class="next-row">
            <span>Next: set the pose</span>
            <button class="next-button" onclick={() => chooseStep('place')}>Place scan</button>
          </div>
        </div>
      {:else if activeStep === 'place'}
        <section class="action-panel">
          <div class="inspector-heading">
            <p class="step-caption">Step 02</p>
            <h2>Place the scan</h2>
            <p>Give the matcher a starting pose. Moving the scan updates the preview only.</p>
          </div>

          <div class="gesture-card">
            <div class="gesture-row">
              <span class="gesture-key mono">drag</span>
              <span>move the sensor scan</span>
            </div>
            <div class="gesture-row">
              <span class="gesture-key mono">shift + drag</span>
              <span>rotate the sensor</span>
            </div>
          </div>

          {#if $placing}
            <button class="primary" onclick={() => setPlacing(false)}>Validate pose</button>
            <p class="small-note">Move the scan as many times as needed, then validate when it feels right.</p>
          {:else}
            <button class="primary" onclick={() => setPlacing(true)}>Edit pose</button>
          {/if}

          {#if $placed}
            <div class="pose-status">
              <span class="pose-indicator"></span>
              <span>Manual pose saved for the next run.</span>
            </div>
            <button class="secondary wide" onclick={resetPlacement}>Reset to generated guess</button>
          {:else}
            <div class="pose-status pending">
              <span class="pose-indicator"></span>
              <span>Using the generated initial guess.</span>
            </div>
          {/if}

          <div class="next-row">
            <span>Next: choose a run</span>
            <button class="next-button" onclick={() => chooseStep('match')}>Review matcher</button>
          </div>
        </section>
      {:else if activeStep === 'match'}
        <div class="match-shell">
          <section class="run-panel">
            <div class="inspector-heading">
              <p class="step-caption">Step 03</p>
              <h2>Run the matcher</h2>
              <p>Choose a run mode and start the solver when the inputs are ready.</p>
            </div>

            <div class="mode-group" role="group" aria-label="Run mode">
              <button class:active={!$stepMode} onclick={() => setStepMode(false)}>Single frame</button>
              <button class:active={$stepMode} onclick={() => setStepMode(true)}>Step through</button>
            </div>
            <p class="small-note">
              {#if $stepMode}
                Run the matcher one iteration at a time. Each click recomputes the same pair with one more allowed iteration.
              {:else}
                Runs one generated frame with the current pose and settings.
              {/if}
            </p>

            <label for="reference">Reference frame</label>
            <select id="reference" bind:value={$referenceMode}>
              <option value="fixed">Fixed reference</option>
              <option value="previous_frame">Previous frame</option>
            </select>

            {#if $stepMode}
              <p class="trace-note"><span class="trace-mark"></span>Iteration trace is on for this mode.</p>
            {:else}
              <label class="check">
                <input type="checkbox" bind:checked={$trace} />
                Collect iteration trace
              </label>
            {/if}

            <button
              class="primary run-button"
              onclick={runCurrent}
              disabled={$running || $issues.length > 0 || ($stepMode && !$canRunNextStep)}
            >
              {runLabel}
            </button>
            {#if $issues.length > 0}
              <p class="small-note warning-note">Fix the matcher settings below before running.</p>
            {:else if $outdated}
              <p class="small-note">The current result is outdated and will be replaced by this run.</p>
            {/if}
          </section>

          <div class="settings-divider">
            <span>Algorithm settings</span>
            <span class="mono">{$abMode ? 'editing ' + $editingSide : 'A'}</span>
          </div>
          <MatcherPanel target={$abMode && $editingSide === 'B' ? matcherB : matcher} />
        </div>
      {:else}
        <section class="action-panel inspect-panel">
          <div class="inspector-heading">
            <p class="step-caption">Step 04</p>
            <h2>Inspect the result</h2>
            <p>Read the result metrics below, then open the iteration trace here when you need solver detail.</p>
          </div>

          {#if hasResult}
            <div class="result-intro">
              <span class="state-pill ready">{resultLabel}</span>
              <h3>
                {#if $stepMode}
                  Iteration run ready
                {:else if $compare}
                  A/B comparison ready
                {:else}
                  Match ready
                {/if}
              </h3>
              <p>
                {#if $stepMode && $activeFrame?.trace}
                  {$activeFrame.trace.length} iterations are available to inspect below.
                {:else}
                  The solver has finished. The plot shows the returned pose.
                {/if}
              </p>
            </div>

            {#if $activeFrame}
              <dl class="quick-readout">
                <div>
                  <dt>termination</dt>
                  <dd class="mono">{$activeFrame.termination}</dd>
                </div>
                <div>
                  <dt>{$stepMode ? 'iteration target' : 'iterations'}</dt>
                  <dd class="mono">{$stepMode ? $stepTarget : $activeFrame.iterations}</dd>
                </div>
                <div>
                  <dt>correspondences</dt>
                  <dd class="mono">{$activeFrame.nvalid}</dd>
                </div>
              </dl>
            {/if}

            {#if $activeFrame?.trace && $activeFrame.trace.length > 0}
              <div class="diagnostics-control">
                <div class="diagnostics-heading">
                  <span>
                    <strong>Iteration trace</strong>
                    <small>Separate from the scene</small>
                  </span>
                  <button
                    class="inline-button"
                    onclick={() => diagnosticsOpen.update((open) => !open)}
                    aria-expanded={$diagnosticsOpen}
                  >
                    {$diagnosticsOpen ? 'Hide charts' : 'Show charts'}
                  </button>
                </div>
                {#if $diagnosticsOpen}
                  <DiagnosticsPanel />
                {/if}
              </div>
            {/if}
          {:else}
            <div class="empty-result">
              <span class="empty-mark" aria-hidden="true">—</span>
              <h3>No result yet</h3>
              <p>Run the matcher to populate the result strip.</p>
            </div>
          {/if}

          <button class="secondary wide" onclick={() => chooseStep('match')}>Adjust and rerun</button>
        </section>
      {/if}
    </aside>
  </div>

  {#if $stepMode}
    <div class="iteration-dock"><IterationBar /></div>
  {/if}

  {#if hasResult || $abMode}
    <div class="result-dock">
      {#if $abMode}
        <BenchmarkPanel />
      {/if}
      {#if hasResult}<BottomPanel />{/if}
    </div>
  {/if}
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    min-height: 0;
    overflow: hidden;
    background: var(--bg);
  }

  .topbar {
    display: flex;
    align-items: center;
    min-height: 68px;
    padding: 14px 24px;
    gap: 20px;
    border-bottom: 1px solid var(--line);
    background: var(--surface);
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 10px;
    white-space: nowrap;
  }

  .brand-mark {
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.07em;
  }

  .brand-slash {
    color: var(--muted);
  }

  .brand-context {
    color: var(--muted);
    font-size: 12px;
  }

  .top-status {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
  }

  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ink);
  }

  .status-dot.busy {
    animation: pulse 1.1s ease-in-out infinite;
  }

  .top-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .utility {
    min-height: 32px;
    padding: 5px 9px;
    border-color: transparent;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
  }

  .utility:hover,
  .utility.active {
    border-color: var(--line-strong);
    background: var(--surface-2);
    color: var(--text);
  }

  .theme-button {
    margin-left: 4px;
    padding-left: 13px;
    border-left-color: var(--line);
  }

  .error {
    margin: 0;
    padding: 9px 24px;
    border-bottom: 1px solid var(--danger);
    background: color-mix(in srgb, var(--danger) 10%, var(--surface));
    color: var(--text);
    font-size: 12px;
  }

  .workspace {
    display: grid;
    grid-template-columns: 190px minmax(0, 1fr) 316px;
    flex: 1;
    min-height: 0;
  }

  .rail {
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 20px 12px;
    border-right: 1px solid var(--line);
    background: var(--surface);
  }

  .rail-heading {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 0 10px 12px;
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
  }

  .rail-count {
    color: var(--muted);
    font-size: 10px;
  }

  .steps {
    display: grid;
    gap: 3px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .step-button {
    display: grid;
    grid-template-columns: 28px 1fr auto;
    align-items: start;
    width: 100%;
    min-height: 60px;
    padding: 10px;
    border: 0;
    border-left: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    text-align: left;
  }

  .step-button:hover {
    border-color: var(--line-strong);
    background: var(--surface-2);
  }

  .step-button.current {
    border-left-color: var(--ink);
    background: var(--surface-2);
  }

  .step-number {
    padding-top: 1px;
    color: var(--muted);
    font-size: 10px;
  }

  .step-button.current .step-number {
    color: var(--text);
  }

  .step-copy {
    display: grid;
    gap: 2px;
  }

  .step-copy strong {
    font-size: 13px;
    font-weight: 500;
  }

  .step-copy small {
    color: var(--muted);
    font-size: 10px;
    line-height: 1.25;
  }

  .step-check {
    color: var(--muted);
    font-size: 12px;
  }

  .rail-note {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin: auto 10px 0;
    padding-top: 16px;
    border-top: 1px solid var(--line);
  }

  .rail-note-mark {
    flex: 0 0 auto;
    width: 5px;
    height: 5px;
    margin-top: 6px;
    border-radius: 50%;
    background: var(--ink);
  }

  .rail-note p {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
    line-height: 1.45;
  }

  .stage {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg);
  }

  .stage-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
    padding: 22px 28px 16px;
  }

  .stage-copy {
    min-width: 0;
  }

  .step-caption {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px;
    color: var(--muted);
    font-size: 11px;
  }

  .step-marker {
    color: var(--text);
  }

  .stage-copy h1 {
    margin: 0;
    font-size: clamp(22px, 2vw, 30px);
    font-weight: 500;
    letter-spacing: -0.035em;
    line-height: 1.05;
  }

  .stage-copy > p:last-child {
    max-width: 48ch;
    margin: 8px 0 0;
    color: var(--muted);
    font-size: 13px;
  }

  .stage-state {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 7px;
    flex-wrap: wrap;
    padding-top: 25px;
    text-align: right;
  }

  .state-pill {
    display: inline-flex;
    align-items: center;
    min-height: 22px;
    padding: 3px 7px;
    border: 1px solid var(--line-strong);
    color: var(--muted);
    font-size: 10px;
  }

  .state-pill.ready {
    border-color: var(--ink);
    background: var(--ink);
    color: var(--surface);
  }

  .state-pill.warn {
    border-color: var(--ink);
    color: var(--text);
  }

  .state-pill.busy {
    border-style: dashed;
    color: var(--text);
  }

  .pose-state {
    color: var(--muted);
    font-size: 10px;
  }

  .plot-frame {
    position: relative;
    flex: 1;
    min-height: 0;
    margin: 0 16px;
    overflow: hidden;
    border: 1px solid var(--line-strong);
    background: var(--plot);
  }

  .stage-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 40px;
    padding: 8px 28px;
    color: var(--muted);
    font-size: 10px;
  }

  .stage-footer p {
    margin: 0;
  }

  .footer-alert {
    color: var(--text);
  }

  .inspector {
    min-width: 0;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    border-left: 1px solid var(--line);
    background: var(--surface);
  }

  .inspector-scroll,
  .match-shell {
    min-height: 100%;
  }

  .inspector-heading {
    padding: 20px 20px 14px;
  }

  .inspector-heading .step-caption {
    margin-bottom: 7px;
  }

  .inspector-heading h2 {
    margin: 0;
    font-size: 19px;
    font-weight: 500;
    letter-spacing: -0.025em;
  }

  .inspector-heading > p:last-child {
    margin: 7px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }

  .action-panel {
    min-height: 100%;
  }

  .gesture-card {
    display: grid;
    gap: 9px;
    margin: 2px 20px 18px;
    padding: 12px;
    border-top: 1px solid var(--line-strong);
    border-bottom: 1px solid var(--line);
  }

  .gesture-row {
    display: grid;
    grid-template-columns: 92px 1fr;
    gap: 10px;
    align-items: baseline;
    color: var(--text);
    font-size: 12px;
  }

  .gesture-key {
    color: var(--muted);
    font-size: 10px;
  }

  .primary,
  .secondary,
  .next-button {
    min-height: 36px;
    border-radius: 0;
    font-size: 12px;
  }

  .primary {
    width: calc(100% - 40px);
    margin: 0 20px;
    border-color: var(--ink);
    background: var(--ink);
    color: var(--surface);
  }

  .primary:hover {
    border-color: var(--muted);
    background: var(--muted);
  }

  .secondary {
    border-color: var(--line-strong);
    background: transparent;
  }

  .secondary:hover {
    background: var(--surface-2);
  }

  .wide {
    width: calc(100% - 40px);
    margin: 10px 20px 0;
  }

  .small-note {
    margin: 8px 20px 0;
    color: var(--muted);
    font-size: 11px;
    line-height: 1.4;
  }

  .warning-note {
    color: var(--text);
  }

  .pose-status {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin: 20px 20px 0;
    color: var(--text);
    font-size: 11px;
    line-height: 1.35;
  }

  .pose-status.pending {
    color: var(--muted);
  }

  .pose-indicator {
    flex: 0 0 auto;
    width: 7px;
    height: 7px;
    margin-top: 3px;
    border: 1px solid var(--ink);
    border-radius: 50%;
  }

  .pose-status.pending .pose-indicator {
    border-color: var(--muted);
  }

  .next-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin: 26px 20px 20px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 11px;
  }

  .next-button {
    min-height: 30px;
    padding: 4px 9px;
    background: var(--surface-2);
  }

  .run-panel {
    padding-bottom: 18px;
  }

  .mode-group {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1px;
    margin: 0 20px;
    padding: 1px;
    background: var(--line-strong);
  }

  .mode-group button {
    min-height: 32px;
    border: 0;
    border-radius: 0;
    background: var(--surface);
    color: var(--muted);
    font-size: 11px;
  }

  .mode-group button.active {
    background: var(--ink);
    color: var(--surface);
  }

  .run-panel > label:not(.check) {
    margin: 18px 20px 0;
  }

  .run-panel > select {
    width: calc(100% - 40px);
    margin: 4px 20px 0;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 12px 20px 0;
    color: var(--text);
    font-size: 12px;
  }

  .check input {
    width: auto;
    margin: 0;
  }

  .run-button {
    margin-top: 20px;
  }

  .settings-divider {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 11px 20px;
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    color: var(--muted);
    font-size: 11px;
  }

  .settings-divider .mono {
    color: var(--text);
    font-size: 10px;
  }

  .inspect-panel {
    padding-bottom: 20px;
  }

  .result-intro,
  .empty-result {
    margin: 5px 20px 20px;
    padding: 14px 0 16px;
    border-top: 1px solid var(--line-strong);
    border-bottom: 1px solid var(--line);
  }

  .result-intro h3,
  .empty-result h3 {
    margin: 14px 0 4px;
    font-size: 15px;
    font-weight: 500;
  }

  .result-intro p,
  .empty-result p {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }

  .quick-readout {
    display: grid;
    gap: 0;
    margin: 0 20px 18px;
  }

  .quick-readout div {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--line);
  }

  .quick-readout dt {
    color: var(--muted);
    font-size: 11px;
  }

  .quick-readout dd {
    margin: 0;
    color: var(--text);
    font-size: 11px;
    text-align: right;
    overflow-wrap: anywhere;
  }

  .diagnostics-control {
    margin: 20px 20px 0;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }

  .diagnostics-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .diagnostics-heading > span {
    display: grid;
    gap: 2px;
  }

  .diagnostics-heading strong {
    font-size: 12px;
    font-weight: 500;
  }

  .diagnostics-heading small {
    color: var(--muted);
    font-size: 10px;
  }

  .inline-button {
    min-height: 28px;
    padding: 4px 7px;
    background: transparent;
    color: var(--muted);
    font-size: 10px;
    white-space: nowrap;
  }

  .empty-mark {
    color: var(--muted);
    font-size: 22px;
  }

  .iteration-dock,
  .result-dock {
    flex: 0 0 auto;
    min-height: 0;
    max-height: min(32vh, 280px);
    overflow-x: hidden;
    overflow-y: auto;
    border-top: 1px solid var(--line);
    background: var(--surface);
  }

  .result-dock :global(.bottom) {
    border-top: 0;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.3;
    }
    50% {
      opacity: 1;
    }
  }

  @media (max-width: 1120px) {
    .workspace {
      grid-template-columns: 166px minmax(0, 1fr) 288px;
    }

    .topbar {
      padding-inline: 18px;
    }

    .stage-head {
      padding-inline: 20px;
    }

    .stage-footer {
      padding-inline: 20px;
    }
  }

  @media (max-width: 860px) {
    .workspace {
      grid-template-columns: 150px minmax(0, 1fr);
    }

    .inspector {
      grid-column: 1 / -1;
      max-height: 38vh;
      border-top: 1px solid var(--line);
      border-left: 0;
    }

    .stage {
      min-height: 58vh;
    }

    .rail-note {
      display: none;
    }
  }

  @media (max-width: 620px) {
    .app {
      overflow-y: auto;
    }

    .topbar {
      align-items: flex-start;
      flex-wrap: wrap;
      gap: 8px 14px;
      padding: 13px 14px;
    }

    .top-status {
      order: 3;
      width: 100%;
      margin-left: 0;
    }

    .top-actions {
      margin-left: auto;
    }

    .theme-button {
      display: none;
    }

    .workspace {
      display: flex;
      flex-direction: column;
      overflow-y: auto;
    }

    .rail {
      flex: 0 0 auto;
      padding: 12px 10px 8px;
      border-right: 0;
      border-bottom: 1px solid var(--line);
    }

    .rail-heading {
      padding: 0 6px 8px;
    }

    .steps {
      display: flex;
      gap: 3px;
      overflow-x: auto;
    }

    .steps li {
      flex: 1 0 125px;
    }

    .step-button {
      grid-template-columns: 25px 1fr auto;
      min-height: 50px;
      padding: 8px 7px;
      border-top: 2px solid transparent;
      border-left: 0;
    }

    .step-button.current {
      border-top-color: var(--ink);
    }

    .stage {
      flex: 0 0 58vh;
      min-height: 420px;
    }

    .stage-head {
      padding: 16px 14px 12px;
    }

    .stage-copy h1 {
      font-size: 24px;
    }

    .stage-state {
      padding-top: 21px;
    }

    .plot-frame {
      min-height: 260px;
      margin-inline: 10px;
    }

    .stage-footer {
      padding-inline: 14px;
    }

    .stage-footer .footer-alert {
      display: none;
    }

    .inspector {
      flex: 0 0 auto;
      max-height: none;
      overflow: visible;
    }

    .iteration-dock,
    .result-dock {
      max-height: 34vh;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .status-dot.busy {
      animation: none;
    }
  }
</style>
