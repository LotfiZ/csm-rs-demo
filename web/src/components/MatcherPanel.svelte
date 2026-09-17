<script lang="ts">
  import type { Writable } from 'svelte/store';
  import {
    applyPreset,
    MATCHER_GROUPS,
    PRESETS,
    type FieldGroup,
    type FieldKey,
    type FieldSpec,
  } from '../lib/matcherFields';
  import { DEFAULT_MATCHER, type MatcherConfig } from '../lib/api';
  import { abMode, editingSide, issues, matcherPreset } from '../lib/state';

  let { target }: { target: Writable<MatcherConfig> } = $props();

  const invalid = $derived($issues.length > 0);

  function setField(key: FieldKey, value: unknown) {
    target.update((current) => ({ ...current, [key]: value }));
  }

  function resetGroup(group: FieldGroup) {
    target.update((current) => {
      const next = { ...current };
      const writable = next as unknown as Record<string, unknown>;
      for (const field of group.fields) writable[field.key] = DEFAULT_MATCHER[field.key];
      return next;
    });
  }

  function applyPresetId(id: string) {
    const preset = PRESETS.find((candidate) => candidate.id === id);
    if (preset) target.set(applyPreset(preset));
    matcherPreset.set('');
  }

  function defaultValue(key: FieldKey): string {
    const value = DEFAULT_MATCHER[key];
    if (typeof value === 'number') return String(Number(value.toPrecision(4)));
    return String(value);
  }

  function numberValue(field: FieldSpec): number {
    return $target[field.key] as number;
  }

  function stringValue(field: FieldSpec): string {
    return $target[field.key] as string;
  }

  function boolValue(field: FieldSpec): boolean {
    return $target[field.key] as boolean;
  }
</script>

<section class="panel">
  <header>
    <h2>Matcher {$abMode ? $editingSide : ''}</h2>
    <button class="reset" onclick={() => target.set({ ...DEFAULT_MATCHER })}>reset all</button>
  </header>

  {#if $abMode}
    <div class="sides" role="tablist" aria-label="Configuration being edited">
      <button class:active={$editingSide === 'A'} onclick={() => editingSide.set('A')}>A</button>
      <button class:active={$editingSide === 'B'} onclick={() => editingSide.set('B')}>B</button>
    </div>
  {/if}

  <p class="hint">
    Changes the algorithm, not the problem. Generated and imported scans use the same settings.
  </p>

  <label for="preset">Preset</label>
  <select id="preset" bind:value={$matcherPreset} onchange={(e) => applyPresetId(e.currentTarget.value)}>
    <option value="">Choose a preset…</option>
    {#each PRESETS as preset (preset.id)}
      <option value={preset.id}>{preset.name}</option>
    {/each}
  </select>

  {#if invalid}
    <div class="issues" role="alert">
      <strong>Fix before running:</strong>
      <ul>
        {#each $issues as issue (issue)}
          <li>{issue}</li>
        {/each}
      </ul>
    </div>
  {/if}

  {#each MATCHER_GROUPS as group (group.id)}
    <details open={group.open} class="group">
      <summary>
        <span>{group.title}</span>
        <button
          class="reset"
          onclick={(event) => {
            event.preventDefault();
            resetGroup(group);
          }}
        >
          reset group
        </button>
      </summary>

      {#each group.fields as field (field.key)}
        <div class="field">
          {#if field.kind === 'toggle'}
            <label class="check">
              <input
                type="checkbox"
                checked={boolValue(field)}
                onchange={(event) => setField(field.key, event.currentTarget.checked)}
              />
              {field.label}
            </label>
          {:else if field.kind === 'select'}
            <label for={field.key}>{field.label}</label>
            <select
              id={field.key}
              value={stringValue(field)}
              onchange={(event) => setField(field.key, event.currentTarget.value)}
            >
              {#each field.options ?? [] as option (option.value)}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          {:else}
            <label for={field.key}>
              {field.label}
              {#if field.unit}<span class="unit">{field.unit}</span>{/if}
              <span class="default">default {defaultValue(field.key)}</span>
            </label>
            <input
              id={field.key}
              type="number"
              min={field.min}
              max={field.max}
              step={field.step}
              value={numberValue(field)}
              oninput={(event) => setField(field.key, Number(event.currentTarget.value))}
            />
          {/if}
          <p class="help">
            {field.help}
            {#if field.depends}<em>{field.depends}</em>{/if}
          </p>
        </div>
      {/each}
    </details>
  {/each}
</section>

<style>
  .panel {
    padding: 12px;
    overflow-y: auto;
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

  .sides {
    display: flex;
    gap: 4px;
    margin-top: 6px;
  }

  .sides button {
    flex: 1;
  }

  .sides button.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, var(--surface-2));
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

  .issues {
    margin: 10px 0;
    padding: 8px 10px;
    border: 1px solid var(--danger);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--danger) 12%, var(--surface));
    font-size: 12px;
  }

  .issues ul {
    margin: 4px 0 0;
    padding-left: 16px;
  }

  .group {
    border-top: 1px solid var(--line);
    padding: 6px 0;
  }

  summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    list-style: none;
  }

  summary::-webkit-details-marker {
    display: none;
  }

  summary::before {
    content: '▸';
    margin-right: 6px;
    color: var(--muted);
  }

  details[open] > summary::before {
    content: '▾';
  }

  .field {
    padding: 6px 0 4px 14px;
  }

  label {
    display: block;
    margin-top: 2px;
    font-size: 12px;
    color: var(--text);
  }

  label.check {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  label.check input {
    width: auto;
  }

  .unit,
  .default {
    font-size: 11px;
    color: var(--muted);
    margin-left: 6px;
  }

  .default {
    float: right;
    font-family: var(--font-mono);
  }

  input[type='number'],
  select {
    margin-top: 2px;
  }

  .help {
    margin: 3px 0 0;
    font-size: 11px;
    color: var(--muted);
    line-height: 1.35;
  }

  .help em {
    font-style: normal;
    color: var(--raw);
  }
</style>