<script lang="ts">
  import { activeFrame, traceIteration } from '../lib/state';

  const iterations = $derived($activeFrame?.trace ?? []);
  const count = $derived(iterations.length);
  const selected = $derived(Math.min($traceIteration, Math.max(0, count - 1)));
  const iteration = $derived(iterations[selected] ?? null);

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

  function markerX(index: number, total: number, width: number): number {
    return total <= 1 ? width / 2 : (index / (total - 1)) * width;
  }

  const CHART_W = 320;
  const CHART_H = 56;

  const uncertainty = $derived.by(() => {
    const frame = $activeFrame;
    if (!frame) return null;
    if (frame.covariance_status === 'Disabled') {
      return {
        tone: 'muted',
        text: 'Not requested. Turn on “Compute covariance” to see pose uncertainty.',
      };
    }
    if (frame.covariance_status === 'Failed' || !frame.covariance) {
      return {
        tone: 'warn',
        text: 'Requested, but the library could not compute it — the geometry may be degenerate. The pose is still reported; this is not zero uncertainty.',
      };
    }
    const [vx, vy, vt] = frame.covariance;
    const sigmaX = Math.sqrt(Math.max(vx, 0));
    const sigmaY = Math.sqrt(Math.max(vy, 0));
    const sigmaTheta = (Math.sqrt(Math.max(vt, 0)) * 180) / Math.PI;
    return {
      tone: 'ok',
      text: `σx ${sigmaX.toFixed(4)} m · σy ${sigmaY.toFixed(4)} m · σθ ${sigmaTheta.toFixed(3)}°`,
    };
  });
</script>

{#if count > 0}
  <div class="diagnostics">
    <div class="charts">
      <div class="chart">
        <span class="k">residual per iteration</span>
        <svg viewBox="0 0 {CHART_W} {CHART_H}" preserveAspectRatio="none" role="img" aria-label="Residual per iteration">
          <polyline points={polyline(iterations.map((i) => i.error), CHART_W, CHART_H)} />
          <circle
            cx={markerX(selected, count, CHART_W)}
            cy={CHART_H - ((iteration?.error ?? 0) / (Math.max(...iterations.map((i) => i.error)) || 1)) * CHART_H}
            r="3"
          />
        </svg>
      </div>
      <div class="chart">
        <span class="k">valid correspondences per iteration</span>
        <svg viewBox="0 0 {CHART_W} {CHART_H}" preserveAspectRatio="none" role="img" aria-label="Correspondences per iteration">
          <polyline
            class="count"
            points={polyline(iterations.map((i) => i.valid_correspondences), CHART_W, CHART_H)}
          />
          <circle
            class="count"
            cx={markerX(selected, count, CHART_W)}
            cy={CHART_H - ((iteration?.valid_correspondences ?? 0) / (Math.max(...iterations.map((i) => i.valid_correspondences)) || 1)) * CHART_H}
            r="3"
          />
        </svg>
      </div>
    </div>

    <div class="step">
      <input
        type="range"
        min="0"
        max={Math.max(0, count - 1)}
        step="1"
        value={selected}
        oninput={(event) => traceIteration.set(Number(event.currentTarget.value))}
        aria-label="Trace iteration"
      />
      <span class="readout mono">
        iteration {selected + 1} / {count}
        {#if iteration}
          · residual {iteration.error.toFixed(4)} · correspondences {iteration.valid_correspondences}
          {#if iteration.restart}· restart{/if}
        {/if}
      </span>
    </div>

    {#if uncertainty}
      <p class="uncertainty {uncertainty.tone}"><strong>Uncertainty:</strong> {uncertainty.text}</p>
    {/if}

    <p class="note">
      Residual is the matcher's internal fitting error, not accuracy. Truth-based translation and
      rotation error above are the accuracy. Tracing is opt-in and does not change the match result.
    </p>
  </div>
{/if}

<style>
  .diagnostics {
    border-top: 1px solid var(--line);
    background: var(--surface-2);
    padding: 8px 14px 4px;
    display: grid;
    gap: 8px;
  }

  .charts {
    display: flex;
    flex-wrap: wrap;
    gap: 20px;
  }

  .chart {
    display: grid;
    gap: 2px;
  }

  .k {
    font-size: 11px;
    color: var(--muted);
  }

  svg {
    width: 320px;
    height: 56px;
    background: var(--plot);
    border: 1px solid var(--line);
    border-radius: var(--radius);
  }

  polyline {
    fill: none;
    stroke: var(--aligned);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
  }

  polyline.count {
    stroke: var(--raw);
  }

  circle {
    fill: var(--aligned);
  }

  circle.count {
    fill: var(--raw);
  }

  .step {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .step input {
    flex: 1;
    min-width: 160px;
    max-width: 420px;
  }

  .readout {
    font-size: 12px;
    color: var(--muted);
  }

  .uncertainty {
    margin: 0;
    font-size: 12px;
  }

  .uncertainty.ok { color: var(--text); }
  .uncertainty.warn { color: var(--raw); }
  .uncertainty.muted { color: var(--muted); }

  .note {
    margin: 0;
    font-size: 11px;
    color: var(--muted);
  }
</style>