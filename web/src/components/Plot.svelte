<script lang="ts">
  import { onMount } from 'svelte';
  import { activeFrame, compare, layers, placeScan, placing, preview, result, theme, traceIteration, view } from '../lib/state';
  import type { PreviewResponse } from '../lib/api';
  import { tweenPose } from '../lib/tween';

  let wrap: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let cssW = $state(0);
  let cssH = $state(0);

  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);

  let dragging = false;
  let lastX = 0;
  let lastY = 0;

  // Hand-placed scan: while dragging, this overrides the server's initial pose.
  let scanDragging = false;
  let rotating = false;
  let dragPose = $state<[number, number, number] | null>(null);

  // Scale the cue from the measured Rust run instead of imposing a long delay.
  const ANIM_MIN_MS = 36;
  const ANIM_MAX_MS = 120;
  let animT = $state(1);
  let animHandle: number | undefined;
  let animKey = '';
  let fitKey = '';
  let drawHandle: number | undefined;
  let drawPending = false;
  let context: CanvasRenderingContext2D | null = null;
  let pixelRatio = 1;
  let palette = {
    plot: '#111211',
    muted: '#73736d',
    reference: '#d4cfc2',
    sensor: '#54aaa4',
    candidateB: '#a897d7',
    truth: '#d4e0e5',
    correspondence: '#eef0e5',
    danger: '#a83b34',
    fontMono: 'monospace',
  };
  const MAX_VISIBLE_LINKS = 36;

  function reducedMotion(): boolean {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  }

  function poseOf(requestId: number | undefined): string {
    return requestId === undefined ? '' : String(requestId);
  }

  const extent = $derived($view?.extent ?? 7);

  function fitScale(): number {
    const scale = Math.min(cssW, cssH) / (2 * extent * 1.15);
    return Number.isFinite(scale) && scale > 0 ? scale : 1;
  }

  function toScreen(x: number, y: number): [number, number] {
    const s = fitScale() * zoom;
    return [cssW / 2 + (x - panX) * s, cssH / 2 - (y - panY) * s];
  }

  function toWorld(sx: number, sy: number): [number, number] {
    const s = fitScale() * zoom;
    return [panX + (sx - cssW / 2) / s, panY - (sy - cssH / 2) / s];
  }

  function resetView() {
    const data = $view;
    if (!data) {
      panX = 0;
      panY = 0;
      zoom = 1;
      return;
    }
    // Fit the drawn data, not a fixed centre: panning away then resetting
    // must always bring the scan back into frame.
    let minX = Infinity;
    let maxX = -Infinity;
    let minY = Infinity;
    let maxY = -Infinity;
    for (const cloud of [
      data.reference,
      data.sensor_unaligned,
      data.sensor_aligned,
      data.sensor_aligned_b,
      data.sensor_true,
    ]) {
      if (!cloud) continue;
      for (const [x, y] of cloud) {
        minX = Math.min(minX, x);
        maxX = Math.max(maxX, x);
        minY = Math.min(minY, y);
        maxY = Math.max(maxY, y);
      }
    }
    if (!Number.isFinite(minX)) {
      panX = 0;
      panY = 0;
      zoom = 1;
      return;
    }
    const span = Math.max(maxX - minX, maxY - minY) || 1;
    zoom = Math.min(40, Math.max(0.2, Math.min(cssW, cssH) / (span * 1.25) / fitScale()));
    panX = (minX + maxX) / 2;
    panY = (minY + maxY) / 2;
  }

  /** Keyboard pan and zoom, so the plot is not mouse-only. */
  function onKey(event: KeyboardEvent) {
    const step = 24;
    const scale = fitScale() * zoom;
    if (event.key === 'ArrowLeft') panX -= step / scale;
    else if (event.key === 'ArrowRight') panX += step / scale;
    else if (event.key === 'ArrowUp') panY -= step / scale;
    else if (event.key === 'ArrowDown') panY += step / scale;
    else if (event.key === '+' || event.key === '=') zoom = Math.min(40, zoom * 1.2);
    else if (event.key === '-' || event.key === '_') zoom = Math.max(0.2, zoom / 1.2);
    else if (event.key === '0') resetView();
    else return;
    event.preventDefault();
  }

  function onWheel(event: WheelEvent) {
    event.preventDefault();
    const rect = canvas.getBoundingClientRect();
    const sx = event.clientX - rect.left;
    const sy = event.clientY - rect.top;
    const [wx, wy] = toWorld(sx, sy);
    zoom = Math.min(40, Math.max(0.2, zoom * (event.deltaY < 0 ? 1.12 : 1 / 1.12)));
    const [nx, ny] = toWorld(sx, sy);
    panX += wx - nx;
    panY += wy - ny;
  }

  function onPointerDown(event: PointerEvent) {
    if ($placing && $view?.initial_pose) {
      const trace = $activeFrame?.trace;
      const traceIndex = Math.min($traceIteration, Math.max(0, (trace?.length ?? 1) - 1));
      const targetPose = trace?.[traceIndex]?.pose ?? $view.estimated_pose ?? $view.initial_pose;
      const visiblePose =
        targetPose && $view.initial_pose
          ? tweenPose($view.initial_pose, targetPose, ease(animT))
          : targetPose;
      scanDragging = true;
      rotating = event.shiftKey;
      dragPose = visiblePose ? [...visiblePose] : null;
      lastX = event.clientX;
      lastY = event.clientY;
      canvas.setPointerCapture(event.pointerId);
      return;
    }
    dragging = true;
    lastX = event.clientX;
    lastY = event.clientY;
    canvas.setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: PointerEvent) {
    if (scanDragging && dragPose) {
      if (rotating) {
        dragPose = [dragPose[0], dragPose[1], dragPose[2] + (event.clientX - lastX) * 0.01];
      } else {
        const s = fitScale() * zoom;
        dragPose = [
          dragPose[0] + (event.clientX - lastX) / s,
          dragPose[1] - (event.clientY - lastY) / s,
          dragPose[2],
        ];
      }
      lastX = event.clientX;
      lastY = event.clientY;
      return;
    }
    if (!dragging) return;
    const s = fitScale() * zoom;
    panX -= (event.clientX - lastX) / s;
    panY += (event.clientY - lastY) / s;
    lastX = event.clientX;
    lastY = event.clientY;
  }

  function onPointerUp(event: PointerEvent) {
    if (scanDragging && dragPose) {
      const pose = dragPose;
      scanDragging = false;
      dragPose = null;
      canvas.releasePointerCapture(event.pointerId);
      placeScan(pose);
      return;
    }
    dragging = false;
    canvas.releasePointerCapture(event.pointerId);
  }

  /** Move a cloud already placed at `base` so it sits at `pose` instead. */
  function transformCloud(
    points: [number, number][],
    base: [number, number, number],
    pose: [number, number, number],
  ): [number, number][] {
    const dtheta = pose[2] - base[2];
    const cos = Math.cos(dtheta);
    const sin = Math.sin(dtheta);
    const dx = pose[0] - base[0];
    const dy = pose[1] - base[1];
    return points.map(([x, y]) => {
      const rx = x - base[0];
      const ry = y - base[1];
      return [base[0] + cos * rx - sin * ry + dx, base[1] + sin * rx + cos * ry + dy];
    });
  }

  function drawArrow(ctx: CanvasRenderingContext2D, pose: [number, number, number], stroke: string) {
    const [x, y, theta] = pose;
    const [sx, sy] = toScreen(x, y);
    const len = 0.55 * fitScale() * zoom;
    const hx = sx + Math.cos(theta) * len;
    const hy = sy - Math.sin(theta) * len;
    ctx.strokeStyle = stroke;
    ctx.fillStyle = stroke;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(sx, sy);
    ctx.lineTo(hx, hy);
    ctx.stroke();
    const head = 6;
    ctx.beginPath();
    ctx.arc(hx, hy, head / 2, 0, Math.PI * 2);
    ctx.fill();
  }

  function updatePalette() {
    const styles = getComputedStyle(document.documentElement);
    const value = (name: string, fallback: string) => styles.getPropertyValue(name).trim() || fallback;
    palette = {
      plot: value('--plot', palette.plot),
      muted: value('--muted', palette.muted),
      reference: value('--scan-reference', palette.reference),
      sensor: value('--scan-sensor', palette.sensor),
      candidateB: value('--scan-candidate', palette.candidateB),
      truth: value('--scan-truth', palette.truth),
      correspondence: value('--scan-correspondence', palette.correspondence),
      danger: value('--danger', palette.danger),
      fontMono: value('--font-mono', palette.fontMono),
    };
  }

  function scheduleDraw() {
    if (drawPending) return;
    drawPending = true;
    drawHandle = requestAnimationFrame(() => {
      drawPending = false;
      draw();
    });
  }

  function ease(t: number): number {
    const clamped = Math.min(1, Math.max(0, t));
    return clamped * clamped * (3 - 2 * clamped);
  }

  function draw() {
    const ctx = context;
    if (!ctx || cssW === 0 || cssH === 0) return;
    ctx.clearRect(0, 0, cssW, cssH);
    ctx.fillStyle = palette.plot;
    ctx.fillRect(0, 0, cssW, cssH);

    const data = $view;
    if (!data) {
      ctx.fillStyle = palette.muted;
      ctx.font = `13px ${palette.fontMono}`;
      ctx.textAlign = 'center';
      ctx.fillText('Pick an example and run a match.', cssW / 2, cssH / 2);
      return;
    }

    const visible = $layers;

    const radius = Math.max(1.2, 0.035 * fitScale() * zoom);
    const cloud = (
      points: [number, number][],
      stroke: string,
      alpha: number,
    ) => {
      ctx.fillStyle = stroke;
      ctx.globalAlpha = alpha;
      for (const [x, y] of points) {
        const [sx, sy] = toScreen(x, y);
        ctx.beginPath();
        ctx.arc(sx, sy, radius, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;
    };

    if (visible.truth && data.sensor_true) cloud(data.sensor_true, palette.truth, 0.9);
    if (visible.reference) cloud(data.reference, palette.reference, 0.9);
    const trace = $activeFrame?.trace;
    const traceIndex = Math.min($traceIteration, Math.max(0, (trace?.length ?? 1) - 1));
    const selectedTrace = trace && trace.length > 0 ? trace[traceIndex] : null;
    const targetPose = selectedTrace?.pose ?? data.estimated_pose ?? data.initial_pose;
    const sensorPose =
      dragPose ??
      (targetPose && data.initial_pose
        ? tweenPose(data.initial_pose, targetPose, ease(animT))
        : targetPose);
    const sensorCloud =
      sensorPose && data.initial_pose
        ? transformCloud(data.sensor_unaligned, data.initial_pose, sensorPose)
        : data.sensor_unaligned;

    // One sensor cloud follows the pose. The initial and solved positions are
    // states of the same scan, so the plot never doubles the sensor visually.
    if (visible.scan) cloud(sensorCloud, palette.sensor, 0.95);

    const candidatePose =
      data.estimated_pose_b && data.initial_pose
        ? tweenPose(data.initial_pose, data.estimated_pose_b, ease(animT))
        : data.estimated_pose_b;
    if (visible.candidateB && data.estimated_pose_b) {
      const candidateCloud =
        candidatePose && data.initial_pose
          ? transformCloud(data.sensor_unaligned, data.initial_pose, candidatePose)
          : data.sensor_aligned_b;
      if (candidateCloud) cloud(candidateCloud, palette.candidateB, 0.95);
    }

    if (visible.truth && data.truth_pose) drawArrow(ctx, data.truth_pose, palette.truth);
    if (visible.scan && sensorPose) drawArrow(ctx, sensorPose, palette.sensor);
    if (visible.candidateB && candidatePose) drawArrow(ctx, candidatePose, palette.candidateB);

    // A rejected update is marked, never drawn as a successful move.
    if (data.rejected && data.estimated_pose) {
      const [sx, sy] = toScreen(data.estimated_pose[0], data.estimated_pose[1]);
      ctx.strokeStyle = palette.danger;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(sx, sy, 9, 0, Math.PI * 2);
      ctx.stroke();
    }

    // The library records the sensor point before solving the selected
    // iteration. Use the previous pose as its frame of reference so every
    // endpoint stays attached to the sensor point drawn above.
    if (visible.correspondences && selectedTrace) {
      const correspondences = selectedTrace.correspondences.filter(
        (item) =>
          Number.isFinite(item.sensor_point[0]) &&
          Number.isFinite(item.sensor_point[1]) &&
          Number.isFinite(item.reference_point[0]) &&
          Number.isFinite(item.reference_point[1]),
      );
      const distances = correspondences
        .map((item) => item.distance)
        .filter((distance) => Number.isFinite(distance));
      const maxDistance = Math.max(...distances, 0.0001);
      const stride = Math.max(1, Math.ceil(correspondences.length / MAX_VISIBLE_LINKS));
      const correspondenceBasePose =
        traceIndex > 0 ? trace?.[traceIndex - 1]?.pose ?? data.initial_pose : data.initial_pose;
      const endpoints: Array<{ sensor: [number, number]; reference: [number, number]; quality: number }> = [];
      for (let index = 0; index < correspondences.length; index += stride) {
        const correspondence = correspondences[index];
        const sensorPoint = sensorPose
          ? transformCloud([correspondence.sensor_point], correspondenceBasePose, sensorPose)[0]
          : correspondence.sensor_point;
        const [x1, y1] = toScreen(sensorPoint[0], sensorPoint[1]);
        const [x2, y2] = toScreen(
          correspondence.reference_point[0],
          correspondence.reference_point[1],
        );
        const quality = 1 - Math.min(1, Math.max(0, correspondence.distance / maxDistance));
        const stroke = quality < 0.35 ? palette.danger : palette.correspondence;
        ctx.strokeStyle = stroke;
        ctx.lineCap = 'round';
        ctx.globalAlpha = 0.18 + quality * 0.62;
        ctx.lineWidth = 0.75 + quality * 1.25;
        ctx.beginPath();
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
        ctx.stroke();
        const angle = Math.atan2(y2 - y1, x2 - x1);
        const head = 4 + quality * 2;
        ctx.fillStyle = stroke;
        ctx.globalAlpha = 0.24 + quality * 0.58;
        ctx.beginPath();
        ctx.moveTo(x2, y2);
        ctx.lineTo(x2 - Math.cos(angle - 0.5) * head, y2 - Math.sin(angle - 0.5) * head);
        ctx.lineTo(x2 - Math.cos(angle + 0.5) * head, y2 - Math.sin(angle + 0.5) * head);
        ctx.closePath();
        ctx.fill();
        endpoints.push({ sensor: [x1, y1], reference: [x2, y2], quality });
      }
      for (const endpoint of endpoints) {
        ctx.globalAlpha = 0.75;
        ctx.fillStyle = palette.sensor;
        ctx.beginPath();
        ctx.arc(endpoint.sensor[0], endpoint.sensor[1], 2.2, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = endpoint.quality < 0.35 ? palette.danger : palette.reference;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.arc(endpoint.reference[0], endpoint.reference[1], 2.8, 0, Math.PI * 2);
        ctx.stroke();
      }
      ctx.globalAlpha = 1;
      ctx.lineCap = 'butt';
    }
  }

  function resizeCanvas() {
    if (!canvas || cssW === 0 || cssH === 0) return;
    const nextRatio = window.devicePixelRatio || 1;
    const width = Math.round(cssW * nextRatio);
    const height = Math.round(cssH * nextRatio);
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
      context = canvas.getContext('2d');
    }
    pixelRatio = nextRatio;
    context?.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
  }

  onMount(() => {
    context = canvas.getContext('2d');
    updatePalette();
    const observer = new ResizeObserver((entries) => {
      const rect = entries[0].contentRect;
      cssW = rect.width;
      cssH = rect.height;
      resizeCanvas();
      scheduleDraw();
    });
    observer.observe(wrap);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(drawHandle ?? 0);
      cancelAnimationFrame(animHandle ?? 0);
    };
  });

  $effect(() => {
    // Coalesce pointer and store updates into one paint per browser frame.
    void $view;
    void $activeFrame;
    void $traceIteration;
    void $layers;
    void $theme;
    void $placing;
    void cssW;
    void cssH;
    void zoom;
    void panX;
    void panY;
    void animT;
    void dragPose;
    scheduleDraw();
  });

  $effect(() => {
    void $theme;
    updatePalette();
    scheduleDraw();
  });

  $effect(() => {
    // One animation per new result, keyed by identity so
    // panning and zooming never restart it.
    const key = `${poseOf($result?.request_id)}:${poseOf($compare?.request_id)}:${poseOf($activeFrame?.request_id)}`;
    if (!key || key === animKey) return;
    animKey = key;
    if (reducedMotion()) {
      animT = 1;
      return;
    }
    animT = 0;
    cancelAnimationFrame(animHandle ?? 0);
    const start = performance.now();
    const runtimeMs = $activeFrame?.normal_ms ?? $compare?.a.normal_ms ?? 0;
    const duration = Math.min(ANIM_MAX_MS, Math.max(ANIM_MIN_MS, runtimeMs * 4));
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / duration);
      animT = t;
      if (t < 1) animHandle = requestAnimationFrame(tick);
    };
    animHandle = requestAnimationFrame(tick);
  });

  $effect(() => {
    // Fit once per new run or preview, never on scrub or pan.
    const key = `${poseOf($result?.request_id)}:${poseOf($compare?.request_id)}`;
    if (!key || key === fitKey || cssW <= 0) return;
    fitKey = key;
    resetView();
  });

  let lastPreview: PreviewResponse | null = null;
  $effect(() => {
    const currentPreview = $preview;
    void cssW;
    if (!currentPreview || currentPreview === lastPreview || cssW <= 0) return;
    const samePoints = (a: [number, number][] | undefined, b: [number, number][] | undefined) =>
      a?.length === b?.length &&
      a?.every((point, index) => point[0] === b?.[index]?.[0] && point[1] === b?.[index]?.[1]);
    const geometryChanged =
      !lastPreview ||
      !samePoints(currentPreview.reference, lastPreview.reference) ||
      !samePoints(currentPreview.sensor_true, lastPreview.sensor_true);
    lastPreview = currentPreview;
    if (geometryChanged) resetView();
  });

  const selectedIteration = $derived.by(() => {
    const iterations = $activeFrame?.trace ?? [];
    return iterations[Math.min($traceIteration, Math.max(0, iterations.length - 1))] ?? null;
  });
</script>

<div class="plot" bind:this={wrap}>
  <!-- svelte-ignore a11y_no_interactive_element_to_noninteractive_role a11y_no_noninteractive_tabindex -->
  <canvas
    bind:this={canvas}
    style="width: {cssW}px; height: {cssH}px"
    tabindex="0"
    role="img"
    aria-label="Scan plot. Use arrow keys to pan, plus and minus to zoom, and 0 to reset."
    onwheel={onWheel}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    ondblclick={resetView}
    onkeydown={onKey}
    class:placing={$placing}
  ></canvas>

  <details class="legend" aria-label="Scan layers">
    <summary>Layers</summary>
    <div class="layer-list">
      <label><input type="checkbox" bind:checked={$layers.reference} /> <i class="sw ref"></i>reference scan</label>
      <label><input type="checkbox" bind:checked={$layers.scan} /> <i class="sw sensor"></i>sensor scan</label>
      <label><input type="checkbox" bind:checked={$layers.candidateB} /> <i class="sw candidate-b"></i>candidate B</label>
      <label><input type="checkbox" bind:checked={$layers.truth} /> <i class="sw truth"></i>ground truth</label>
      <label><input type="checkbox" bind:checked={$layers.correspondences} /> <i class="sw correspondence"></i>correspondences</label>
      <button class="reset" onclick={resetView}>Reset view</button>
    </div>
  </details>

  {#if $placing}
    <div class="plot-tip" role="status">
      <strong>Placement mode</strong>
      <span>drag to move · shift + drag to rotate</span>
    </div>
  {/if}
</div>

<style>
  .plot {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  canvas {
    display: block;
    touch-action: none;
    cursor: grab;
  }

  canvas:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  canvas:active {
    cursor: grabbing;
  }

  .legend {
    position: absolute;
    top: 16px;
    left: 16px;
    margin: 0;
    padding: 0;
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    font-size: 12px;
  }

  .legend summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 92px;
    padding: 7px 9px;
    color: var(--text);
    cursor: pointer;
    font-size: 11px;
    list-style: none;
  }

  .legend summary::-webkit-details-marker {
    display: none;
  }

  .legend summary::after {
    content: '+';
    color: var(--muted);
    font-family: var(--font-mono);
  }

  .legend[open] summary {
    border-bottom: 1px solid var(--line);
  }

  .legend[open] summary::after {
    content: '–';
  }

  .layer-list {
    display: grid;
    gap: 4px;
    padding: 8px 9px 9px;
  }

  .legend label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    cursor: pointer;
  }

  .legend input {
    width: auto;
    margin: 0;
  }

  .sw {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    display: inline-block;
  }

  .sw.ref { background: var(--scan-reference); }
  .sw.sensor { background: var(--scan-sensor); }
  .sw.candidate-b { background: var(--scan-candidate); }
  .sw.truth { background: var(--scan-truth); }
  .sw.correspondence { background: var(--scan-correspondence); }

  .reset {
    width: 100%;
    margin-top: 3px;
    font-size: 11px;
    padding: 2px 6px;
  }

  .plot-tip {
    position: absolute;
    top: 16px;
    right: 16px;
    display: grid;
    gap: 2px;
    padding: 8px 10px;
    border: 1px solid var(--line-strong);
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    color: var(--text);
    font-size: 11px;
  }

  .plot-tip span {
    color: var(--muted);
    font-size: 10px;
  }

  canvas.placing {
    cursor: crosshair;
  }
</style>
