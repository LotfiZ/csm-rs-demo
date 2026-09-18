<script lang="ts">
  import { benchmark, benchmarking, runBenchmark } from '../lib/state';

  function polyline(values: number[], width: number, height: number): string {
    if (values.length === 0) return '';
    const max = Math.max(...values);
    const min = Math.min(...values);
    const span = max - min || 1;
    return values
      .map((value, index) => {
        const x = values.length === 1 ? width / 2 : (index / (values.length - 1)) * width;
        const y = height - ((value - min) / span) * height;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  }

  const W = 220;
  const H = 40;
</script>

<section class="bench" aria-label="A/B benchmark">
  <header>
    <h3>Repeated A/B benchmark</h3>
    <button class="reset" onclick={runBenchmark} disabled={$benchmarking}>
      {$benchmarking ? 'Benchmarking…' : 'Run benchmark'}
    </button>
  </header>

  {#if $benchmark}
    <div class="grid">
      <table>
        <thead>
          <tr>
            <th scope="col">measure</th>
            <th scope="col">A</th>
            <th scope="col">B</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th scope="row">samples / warm-up</th>
            <td class="mono">{$benchmark.a.samples} / {$benchmark.a.warmup}</td>
            <td class="mono">{$benchmark.b.samples} / {$benchmark.b.warmup}</td>
          </tr>
          <tr>
            <th scope="row">prepare (ms)</th>
            <td class="mono">{$benchmark.a.prepare_ms.toFixed(3)}</td>
            <td class="mono">{$benchmark.b.prepare_ms.toFixed(3)}</td>
          </tr>
          <tr>
            <th scope="row">matching median (ms)</th>
            <td class="mono">{$benchmark.a.median_ms.toFixed(3)}</td>
            <td class="mono">{$benchmark.b.median_ms.toFixed(3)}</td>
          </tr>
          <tr>
            <th scope="row">matching mean (ms)</th>
            <td class="mono">{$benchmark.a.mean_ms.toFixed(3)}</td>
            <td class="mono">{$benchmark.b.mean_ms.toFixed(3)}</td>
          </tr>
          <tr>
            <th scope="row">min – max (ms)</th>
            <td class="mono">
              {$benchmark.a.min_ms.toFixed(3)} – {$benchmark.a.max_ms.toFixed(3)}
            </td>
            <td class="mono">
              {$benchmark.b.min_ms.toFixed(3)} – {$benchmark.b.max_ms.toFixed(3)}
            </td>
          </tr>
        </tbody>
      </table>

      <div class="dist">
        <span class="k">matching time distribution</span>
        <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" role="img" aria-label="A matching times">
          <polyline points={polyline($benchmark.a.match_ms, W, H)} />
        </svg>
        <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" role="img" aria-label="B matching times">
          <polyline class="b" points={polyline($benchmark.b.match_ms, W, H)} />
        </svg>
      </div>
    </div>
    <p class="note">
      Conditions: {$benchmark.scenario}, {$benchmark.reference_mode}, step {$benchmark.step},
      {$benchmark.samples} uninstrumented samples after {$benchmark.warmup} warm-up. Preparation is
      reported separately from matching. These timings are machine-specific and are not a
      machine-independent performance claim.
    </p>
  {:else}
    <p class="note">
      Warm up and repeat both configurations on the current shared inputs, with preparation cost
      reported separately from matching.
    </p>
  {/if}
</section>

<style>
  .bench {
    border-top: 1px solid var(--line);
    background: var(--surface);
    padding: 8px 14px;
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }

  .reset {
    font-size: 11px;
    padding: 2px 6px;
  }

  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 20px;
    margin-top: 6px;
  }

  table {
    border-collapse: collapse;
    font-size: 12px;
  }

  th,
  td {
    text-align: left;
    padding: 3px 14px 3px 0;
  }

  tbody th {
    font-weight: 400;
    color: var(--muted);
  }

  thead th {
    color: var(--muted);
    font-weight: 600;
  }

  .dist {
    display: grid;
    gap: 3px;
  }

  .k {
    font-size: 11px;
    color: var(--muted);
  }

  svg {
    width: 220px;
    height: 40px;
    background: var(--plot);
    border: 1px solid var(--line);
    border-radius: var(--radius);
  }

  polyline {
    fill: none;
    stroke: var(--scan-solver);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
  }

  polyline.b {
    stroke: var(--scan-candidate);
  }

  .note {
    margin: 6px 0 0;
    font-size: 11px;
    color: var(--muted);
  }
</style>
