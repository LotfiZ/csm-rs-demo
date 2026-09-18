<script lang="ts">
  import { EXAMPLE_LIST, exampleById } from '../lib/examples';
  import ExperimentsPanel from './ExperimentsPanel.svelte';
  import {
    clearImport,
    exampleId,
    generation,
    importMode,
    importPair,
    loadExample,
  } from '../lib/state';

  const selected = $derived(exampleById($exampleId));

  const fovDeg = $derived(($generation.half_span * 2 * 180) / Math.PI);

  let pairText = $state('');

  function setFov(degrees: number) {
    generation.update((g) => ({ ...g, half_span: ((degrees / 2) * Math.PI) / 180 }));
  }

  function samplePair() {
    const angles: number[] = [];
    const readings: number[] = [];
    const valid: boolean[] = [];
    for (let i = 0; i < 61; i += 1) {
      const a = -1.5 + i * 0.05;
      angles.push(a);
      readings.push(6 + 0.5 * Math.sin(3 * a));
      valid.push(true);
    }
    return {
      format: 'csm-rs-scan-pair',
      version: 1,
      initial_guess: [0, 0, 0],
      reference: { kind: 'polar', angles, readings, valid },
      sensor: { kind: 'polar', angles, readings, valid },
    };
  }
</script>

<section class="panel">
  <h2>Examples</h2>
  <ul class="examples">
    {#each EXAMPLE_LIST as example (example.id)}
      <li>
        <button
          class="example"
          class:active={example.id === $exampleId}
          onclick={() => loadExample(example.id)}
        >
          {example.name}
        </button>
      </li>
    {/each}
  </ul>
  <p class="observe">{selected.observe}</p>
  <p class="preview-note">Changes update the preview. Run the matcher from step 03.</p>

  <h2>Scene parameters</h2>
  <p class="hint">These shape the generated scan.</p>

  <label for="noise">Range noise, m <output>{$generation.noise.toFixed(3)}</output></label>
  <input id="noise" type="range" min="0" max="0.2" step="0.002" bind:value={$generation.noise} />

  <label for="initial">Initial guess error, m/rad <output>{$generation.initial_error.toFixed(2)}</output></label>
  <input
    id="initial"
    type="range"
    min="0"
    max="1.5"
    step="0.01"
    bind:value={$generation.initial_error}
  />
  <p class="hint">Ignored once you place the scan by hand on the plot.</p>

  <label for="motion">Motion scale <output>{$generation.motion.toFixed(2)}</output></label>
  <input id="motion" type="range" min="0" max="3" step="0.05" bind:value={$generation.motion} />

  <details class="advanced">
    <summary>More scene settings</summary>

    <label for="seed">Seed</label>
    <input id="seed" type="number" min="0" step="1" bind:value={$generation.seed} />

    <label for="step">Scene sample <output>{$generation.step}</output></label>
    <input id="step" type="range" min="0" max="24" step="1" bind:value={$generation.step} />

    <label for="dropout">Dropout <output>{$generation.dropout.toFixed(2)}</output></label>
    <input id="dropout" type="range" min="0" max="0.8" step="0.01" bind:value={$generation.dropout} />

    <label for="rays">Sensor rays</label>
    <input id="rays" type="number" min="3" step="1" bind:value={$generation.ray_count} />

    <label for="fov">Field of view, °</label>
    <input
      id="fov"
      type="number"
      min="1"
      max="360"
      step="1"
      value={fovDeg.toFixed(1)}
      oninput={(event) => setFov(Number(event.currentTarget.value))}
    />

    <label for="overlap">Overlap separation, m <output>{$generation.overlap.toFixed(2)}</output></label>
    <input id="overlap" type="range" min="0" max="3" step="0.05" bind:value={$generation.overlap} />
    <p class="hint">Moves the sensor further along its path. This selects a generated scene sample; it does not select matcher iterations.</p>
  </details>

  <details class="advanced">
    <summary>Advanced tools</summary>
    <p class="hint">Bring your own scans or save a generated run for later.</p>

    <section class="tool-section" aria-labelledby="import-scan-heading">
      <h3 id="import-scan-heading">Import scan pair</h3>
      <p class="hint">Paste reference and sensor scans as JSON. Imported scans have no ground truth.</p>
      {#if $importMode}
        <button class="wide" onclick={clearImport}>Use generated example</button>
      {/if}
      <textarea
        bind:value={pairText}
        rows="6"
        spellcheck="false"
        placeholder="Paste scan-pair JSON here"
        aria-label="Scan pair JSON"
      ></textarea>
      <div class="row">
        <button onclick={() => importPair(pairText)}>Load scan pair</button>
        <button onclick={() => (pairText = JSON.stringify(samplePair(), null, 2))}>Use sample</button>
      </div>
    </section>

    <ExperimentsPanel />
  </details>
</section>

<style>
  .panel {
    padding: 0 20px 4px;
  }

  h2 {
    margin: 24px 0 7px;
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }

  h2:first-child {
    margin-top: 0;
  }

  .hint {
    margin: -2px 0 9px;
    font-size: 11px;
    color: var(--muted);
  }

  .examples {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  .example {
    width: 100%;
    padding: 8px 9px;
    border-color: transparent;
    border-left: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--text);
    font-size: 12px;
    text-align: left;
  }

  .example:hover,
  .example.active {
    border-left-color: var(--ink);
    background: var(--surface-2);
  }

  .example.active {
    font-weight: 500;
  }

  .advanced {
    margin-top: 18px;
    border-top: 1px solid var(--line);
    padding-top: 11px;
  }

  .advanced summary {
    font-size: 11px;
    color: var(--muted);
    cursor: pointer;
    list-style: none;
  }

  .advanced summary::-webkit-details-marker {
    display: none;
  }

  .advanced summary::before {
    content: '+';
    display: inline-block;
    width: 15px;
    color: var(--text);
  }

  .advanced[open] summary::before {
    content: '–';
  }

  .advanced[open] summary {
    margin-bottom: 4px;
    color: var(--text);
  }

  .observe {
    margin: 10px 0 5px;
    font-size: 11px;
    line-height: 1.45;
    color: var(--muted);
  }

  .preview-note {
    margin: 0;
    color: var(--text);
    font-size: 10px;
    line-height: 1.45;
  }

  label {
    margin-top: 8px;
  }

  output {
    font-family: var(--font-mono);
    color: var(--text);
    float: right;
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

  .row {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }

  .wide {
    width: 100%;
    margin-top: 6px;
  }

  .tool-section {
    margin-top: 13px;
    border-top: 1px solid var(--line);
    padding-top: 12px;
  }

  .tool-section h3 {
    margin: 0 0 4px;
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
  }
</style>
