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
    referenceMode,
    sequenceMode,
    trace,
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

  <h2>Generation</h2>
  <p class="hint">Shapes the problem. Never the matcher.</p>

  <label for="scenario">Scene</label>
  <select id="scenario" bind:value={$generation.scenario}>
    <option value="asymmetric_room">Asymmetric room</option>
    <option value="ambiguous_corridor">Ambiguous corridor</option>
    <option value="partial_overlap">Partial overlap</option>
  </select>

  <label for="seed">Seed</label>
  <input id="seed" type="number" min="0" step="1" bind:value={$generation.seed} />

  <label for="step">{$sequenceMode ? 'Frames' : 'Frame step'} <output>{$generation.step}</output></label>
  <input id="step" type="range" min="0" max="24" step="1" bind:value={$generation.step} />

  <label for="motion">Motion scale <output>{$generation.motion.toFixed(2)}</output></label>
  <input id="motion" type="range" min="0" max="3" step="0.05" bind:value={$generation.motion} />

  <label for="noise">Range noise, m <output>{$generation.noise.toFixed(3)}</output></label>
  <input id="noise" type="range" min="0" max="0.2" step="0.002" bind:value={$generation.noise} />

  <label for="dropout">Dropout <output>{$generation.dropout.toFixed(2)}</output></label>
  <input id="dropout" type="range" min="0" max="0.8" step="0.01" bind:value={$generation.dropout} />

  <label for="initial">Initial guess error, m/rad <output>{$generation.initial_error.toFixed(2)}</output></label>
  <input
    id="initial"
    type="range"
    min="0"
    max="1.5"
    step="0.01"
    bind:value={$generation.initial_error}
  />

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
  <p class="hint">Moves the sensor further along its path. Less shared geometry is a measured outcome, not a set percentage.</p>

  <h2>Run controls</h2>
  <label for="reference">Reference</label>
  <select id="reference" bind:value={$referenceMode}>
    <option value="fixed">Fixed reference</option>
    <option value="previous_frame">Previous frame</option>
  </select>

  <label class="check">
    <input type="checkbox" bind:checked={$trace} />
    Collect iteration trace
  </label>

  <h2>Import scan pair</h2>
  <p class="hint">Secondary action. Imported data has no ground truth.</p>
  {#if $importMode}
    <button class="wide" onclick={clearImport}>Show generated data</button>
  {/if}
  <textarea
    bind:value={pairText}
    rows="6"
    spellcheck="false"
    placeholder="Paste a csm-rs-scan-pair document"
    aria-label="Scan pair JSON"
  ></textarea>
  <div class="row">
    <button onclick={() => importPair(pairText)}>Load pair</button>
    <button onclick={() => (pairText = JSON.stringify(samplePair(), null, 2))}>Sample</button>
  </div>

  <ExperimentsPanel />
</section>

<style>
  .panel {
    padding: 12px;
    overflow-y: auto;
  }

  h2 {
    margin: 14px 0 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }

  h2:first-child {
    margin-top: 0;
  }

  .hint {
    margin: -2px 0 8px;
    font-size: 12px;
    color: var(--muted);
  }

  .examples {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 4px;
  }

  .example {
    width: 100%;
    text-align: left;
  }

  .example.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, var(--surface-2));
  }

  .observe {
    margin: 8px 0 4px;
    font-size: 12px;
    color: var(--muted);
  }

  label {
    margin-top: 8px;
  }

  label.check {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    margin-top: 10px;
  }

  label.check input {
    width: auto;
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
</style>