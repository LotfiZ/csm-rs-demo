<script lang="ts">
  import { DEFAULT_MATCHER } from '../lib/api';
  import { matcher } from '../lib/state';

  function reset() {
    matcher.set({ ...DEFAULT_MATCHER });
  }
</script>

<section class="panel">
  <header>
    <h2>Matcher</h2>
    <button class="reset" onclick={reset}>reset</button>
  </header>
  <p class="hint">Changes the algorithm. Same for generated and imported scans.</p>

  <label for="search">Search</label>
  <select id="search" bind:value={$matcher.search}>
    <option value="tricks">Tricks</option>
    <option value="naive">Naive</option>
  </select>

  <label for="metric">Distance metric</label>
  <select id="metric" bind:value={$matcher.metric}>
    <option value="point_to_line">Point to line</option>
    <option value="point_to_point">Point to point</option>
  </select>

  <label for="maxdist">Max correspondence, m</label>
  <input id="maxdist" type="number" min="0" step="0.1" bind:value={$matcher.max_correspondence_dist} />

  <label for="maxiter">Max iterations</label>
  <input id="maxiter" type="number" min="0" step="1" bind:value={$matcher.max_iterations} />

  <label for="outliers">Kept correspondence fraction</label>
  <input id="outliers" type="number" min="0" max="1" step="0.01" bind:value={$matcher.outliers_max_perc} />

  <fieldset>
    <legend>Switches</legend>
    <label class="check"><input type="checkbox" bind:checked={$matcher.restart} /> Restart shell</label>
    <label class="check"><input type="checkbox" bind:checked={$matcher.remove_doubles} /> Remove duplicate correspondences</label>
    <label class="check"><input type="checkbox" bind:checked={$matcher.do_alpha_test} /> Alpha orientation test</label>
    <label class="check"><input type="checkbox" bind:checked={$matcher.do_visibility_test} /> Visibility test</label>
    <label class="check"><input type="checkbox" bind:checked={$matcher.do_compute_covariance} /> Compute covariance</label>
  </fieldset>
</section>

<style>
  .panel {
    padding: 12px;
    overflow-y: auto;
    border-left: 1px solid var(--line);
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }

  .hint {
    margin: 2px 0 8px;
    font-size: 12px;
    color: var(--muted);
  }

  .reset {
    font-size: 11px;
    padding: 2px 6px;
  }

  label {
    margin-top: 8px;
  }

  label.check {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    margin-top: 4px;
  }

  label.check input {
    width: auto;
  }

  fieldset {
    margin: 12px 0 0;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 8px;
  }

  legend {
    font-size: 12px;
    color: var(--muted);
    padding: 0 4px;
  }
</style>