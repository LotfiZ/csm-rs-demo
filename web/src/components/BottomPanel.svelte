<script lang="ts">
  import { matcher, matcherB, abMode, compare, diagnosticsOpen, result, view } from '../lib/state';
  import { matcherDiff } from '../lib/matcherFields';
  import type { CompareSide } from '../lib/api';

  function wrap(angle: number): number {
    return ((angle + Math.PI) % (2 * Math.PI) + 2 * Math.PI) % (2 * Math.PI) - Math.PI;
  }

  function sideError(side: CompareSide): { translation: number; rotationDeg: number } {
    const dx = side.relative_truth_pose[0] - side.estimated_pose[0];
    const dy = side.relative_truth_pose[1] - side.estimated_pose[1];
    return {
      translation: Math.hypot(dx, dy),
      rotationDeg: Math.abs(
        (wrap(side.estimated_pose[2] - side.relative_truth_pose[2]) * 180) / Math.PI,
      ),
    };
  }

  function sideStatus(side: CompareSide): { label: string; tone: string } {
    if (side.accepted) return { label: 'accepted', tone: 'ok' };
    if (side.valid) return { label: 'valid candidate', tone: 'warn' };
    return { label: 'failed candidate', tone: 'bad' };
  }

  const pairErrors = $derived.by(() => {
    const data = $result;
    if (!data) return null;
    const dx = data.relative_truth_pose[0] - data.relative_estimated_pose[0];
    const dy = data.relative_truth_pose[1] - data.relative_estimated_pose[1];
    return {
      translation: Math.hypot(dx, dy),
      rotationDeg: Math.abs(
        (wrap(data.relative_estimated_pose[2] - data.relative_truth_pose[2]) * 180) / Math.PI,
      ),
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

  const differences = $derived(matcherDiff($matcher, $matcherB));
</script>

<section class="bottom" aria-label="Metrics and diagnostics">
  {#if $abMode}
    {#if $compare}
      <div class="compare">
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
              <th scope="row">translation error (m)</th>
              <td class="mono">{sideError($compare.a).translation.toFixed(4)}</td>
              <td class="mono">{sideError($compare.b).translation.toFixed(4)}</td>
            </tr>
            <tr>
              <th scope="row">rotation error (°)</th>
              <td class="mono">{sideError($compare.a).rotationDeg.toFixed(3)}</td>
              <td class="mono">{sideError($compare.b).rotationDeg.toFixed(3)}</td>
            </tr>
            <tr>
              <th scope="row">status</th>
              <td class={sideStatus($compare.a).tone}>{sideStatus($compare.a).label}</td>
              <td class={sideStatus($compare.b).tone}>{sideStatus($compare.b).label}</td>
            </tr>
            <tr>
              <th scope="row">termination</th>
              <td class="mono">{$compare.a.termination}</td>
              <td class="mono">{$compare.b.termination}</td>
            </tr>
            <tr>
              <th scope="row">iterations</th>
              <td class="mono">{$compare.a.iterations}</td>
              <td class="mono">{$compare.b.iterations}</td>
            </tr>
            <tr>
              <th scope="row">correspondences</th>
              <td class="mono">{$compare.a.nvalid}</td>
              <td class="mono">{$compare.b.nvalid}</td>
            </tr>
            <tr>
              <th scope="row">runtime (ms)</th>
              <td class="mono">{$compare.a.normal_ms.toFixed(3)}</td>
              <td class="mono">{$compare.b.normal_ms.toFixed(3)}</td>
            </tr>
          </tbody>
        </table>

        <div class="diff">
          <h3>Parameters that differ</h3>
          {#if differences.length === 0}
            <p class="note">A and B are identical; any result difference is not from parameters.</p>
          {:else}
            <ul>
              {#each differences as difference (difference.label)}
                <li>
                  <span class="param">{difference.label}</span>
                  <span class="mono a">{difference.a}</span>
                  <span class="arrow">→</span>
                  <span class="mono b">{difference.b}</span>
                </li>
              {/each}
            </ul>
          {/if}
          <p class="note">
            Both sides matched the same scans, initial guess, reference mode, and frame. Only the
            matcher parameters differ.
          </p>
        </div>
      </div>
    {:else}
      <p class="note">Run to compare A and B on the same inputs.</p>
    {/if}
  {:else}
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
      </div>
      <p class="note">
        Translation and rotation error compare the estimate with ground truth. Fitting error is the
        matcher's internal correspondence residual and is not accuracy.
      </p>
    {/if}
  {/if}
</section>

<style>
  .bottom {
    border-top: 1px solid var(--line);
    background: var(--surface);
    max-height: 40vh;
    overflow-y: auto;
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: stretch;
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

  .v.ok,
  .ok { color: var(--aligned); }
  .v.warn,
  .warn { color: var(--raw); }
  .v.bad,
  .bad { color: var(--danger); }
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
  }

  .compare {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(260px, 1fr);
    gap: 0;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th,
  td {
    text-align: left;
    padding: 6px 14px;
    border-bottom: 1px solid var(--line);
  }

  thead th {
    font-size: 11px;
    color: var(--muted);
    font-weight: 600;
  }

  tbody th {
    font-weight: 400;
    color: var(--muted);
  }

  td {
    color: var(--text);
    font-weight: 500;
  }

  .diff {
    border-left: 1px solid var(--line);
    padding: 10px 14px;
    background: var(--surface-2);
  }

  .diff h3 {
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }

  .diff ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 4px;
  }

  .diff li {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .param {
    color: var(--muted);
  }

  .a {
    color: var(--aligned);
  }

  .b {
    color: var(--aligned-b);
  }

  .arrow {
    color: var(--muted);
  }
</style>