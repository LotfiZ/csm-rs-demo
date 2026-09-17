<script lang="ts">
  import { diagnosticsOpen, result, view } from '../lib/state';

  function wrap(angle: number): number {
    return ((angle + Math.PI) % (2 * Math.PI) + 2 * Math.PI) % (2 * Math.PI) - Math.PI;
  }

  const pairErrors = $derived.by(() => {
    const data = $result;
    if (!data) return null;
    const dx = data.relative_truth_pose[0] - data.relative_estimated_pose[0];
    const dy = data.relative_truth_pose[1] - data.relative_estimated_pose[1];
    return {
      translation: Math.hypot(dx, dy),
      rotationDeg: Math.abs((wrap(data.relative_estimated_pose[2] - data.relative_truth_pose[2]) * 180) / Math.PI),
    };
  });

  const status = $derived.by(() => {
    const data = $result;
    if (!data) return { label: 'no result', tone: 'muted' as const };
    if (data.accepted) return { label: 'accepted', tone: 'ok' as const };
    if (data.valid) return { label: 'valid candidate', tone: 'warn' as const };
    return { label: 'failed candidate', tone: 'bad' as const };
  });

  // Share of reference geometry the sensor also observes. Measured from the
  // generated points, never the requested separation.
  const observedOverlap = $derived.by(() => {
    const data = $view;
    if (!data || data.reference.length === 0 || data.sensor_true.length === 0) return null;
    const tolerance = 0.15;
    let hits = 0;
    for (const [rx, ry] of data.reference) {
      for (const [sx, sy] of data.sensor_true) {
        if (Math.hypot(rx - sx, ry - sy) <= tolerance) {
          hits += 1;
          break;
        }
      }
    }
    return hits / data.reference.length;
  });
</script>

<section class="bottom" aria-label="Metrics and diagnostics">
  <div class="bar">
    <div class="metric">
      <span class="k">translation error</span>
      <span class="v mono">{pairErrors ? pairErrors.translation.toFixed(4) : '—'} <em>m</em></span>
    </div>
    <div class="metric">
      <span class="k">rotation error</span>
      <span class="v mono">{pairErrors ? pairErrors.rotationDeg.toFixed(3) : '—'} <em>°</em></span>
    </div>
    <div class="metric">
      <span class="k">status</span>
      <span class="v {status.tone}">{status.label}</span>
    </div>
    <div class="metric">
      <span class="k">termination</span>
      <span class="v mono">{$result?.termination ?? '—'}</span>
    </div>
    <div class="metric">
      <span class="k">runtime</span>
      <span class="v mono">{$result ? $result.normal_ms.toFixed(3) : '—'} <em>ms</em></span>
    </div>
    <button class="toggle" onclick={() => diagnosticsOpen.update((open) => !open)}>
      {diagnosticsOpen ? 'hide diagnostics' : 'show diagnostics'}
    </button>
  </div>

  {#if $diagnosticsOpen}
    <div class="diagnostics">
      <div class="metric">
        <span class="k">iterations</span>
        <span class="v mono">{$result?.iterations ?? '—'}</span>
      </div>
      <div class="metric">
        <span class="k">correspondences</span>
        <span class="v mono">{$result?.nvalid ?? '—'}</span>
      </div>
      <div class="metric">
        <span class="k">fitting error</span>
        <span class="v mono">{$result ? $result.error.toFixed(4) : '—'}</span>
      </div>
      <div class="metric">
        <span class="k">uncertainty</span>
        <span class="v mono">{$result?.covariance_status ?? '—'}</span>
      </div>
      <div class="metric">
        <span class="k">overlap (measured)</span>
        <span class="v mono">{observedOverlap === null ? '—' : `${(observedOverlap * 100).toFixed(1)} %`}</span>
      </div>
      <div class="metric">
        <span class="k">reference</span>
        <span class="v mono">{$result?.reference_mode ?? '—'}</span>
      </div>
      <div class="metric">
        <span class="k">traced duration</span>
        <span class="v mono">{$result && $result.instrumented_ms > 0 ? `${$result.instrumented_ms.toFixed(3)} ms` : '—'}</span>
      </div>
    </div>
    <p class="note">
      Translation and rotation error compare the estimate with ground truth. Fitting error
      is the matcher's internal correspondence residual and is not accuracy.
    </p>
  {/if}
</section>

<style>
  .bottom {
    border-top: 1px solid var(--line);
    background: var(--surface);
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: stretch;
    gap: 0;
  }

  .metric {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 8px 14px;
    border-right: 1px solid var(--line);
    min-width: 130px;
  }

  .k {
    font-size: 11px;
    color: var(--muted);
  }

  .v {
    font-size: 14px;
    font-weight: 500;
  }

  .v em {
    font-style: normal;
    font-size: 11px;
    color: var(--muted);
  }

  .v.ok { color: var(--aligned); }
  .v.warn { color: var(--raw); }
  .v.bad { color: var(--danger); }
  .v.muted { color: var(--muted); }

  .toggle {
    margin: 8px 14px 8px auto;
    align-self: center;
    font-size: 12px;
  }

  .diagnostics {
    display: flex;
    flex-wrap: wrap;
    border-top: 1px solid var(--line);
  }

  .diagnostics .metric {
    background: var(--surface-2);
  }

  .note {
    margin: 0;
    padding: 8px 14px;
    font-size: 12px;
    color: var(--muted);
    border-top: 1px solid var(--line);
  }
</style>