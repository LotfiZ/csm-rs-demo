<script lang="ts">
  import { onMount } from 'svelte';
  import { activeFrame, compare, layers, placed, placeScan, placing, resetPlacement, result, sequence, setPlacing, theme, traceIteration, view } from '../lib/state';
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

  // Match animation: 0 is the raw pose, 1 the solved pose.
  const ANIM_MS = 350;
  let animT = $state(1);
  let animHandle: number | undefined;
  let animKey = '';
  let fitKey = '';

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
      scanDragging = true;
      rotating = event.shiftKey;
      dragPose = [...$view.initial_pose] as [number, number, number];
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

  function color(name: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
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

  function drawPath(ctx: CanvasRenderingContext2D, points: [number, number][], stroke: string) {
    if (points.length < 2) return;
    ctx.strokeStyle = stroke;
    ctx.lineWidth = 1.5;
    ctx.globalAlpha = 0.8;
    ctx.beginPath();
    points.forEach(([x, y], index) => {
      const [sx, sy] = toScreen(x, y);
      if (index === 0) ctx.moveTo(sx, sy);
      else ctx.lineTo(sx, sy);
    });
    ctx.stroke();
    ctx.globalAlpha = 1;
  }

  function draw() {
    if (!canvas || cssW === 0 || cssH === 0) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(cssW * dpr);
    canvas.height = Math.round(cssH * dpr);
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssW, cssH);
    ctx.fillStyle = color('--plot');
    ctx.fillRect(0, 0, cssW, cssH);

    const data = $view;
    if (!data) {
      ctx.fillStyle = color('--muted');
      ctx.font = `13px ${color('--font-mono') || 'monospace'}`;
      ctx.textAlign = 'center';
      ctx.fillText('Pick an example and run a match.', cssW / 2, cssH / 2);
      return;
    }

    const visible = $layers;

    if (visible.walls) {
      ctx.strokeStyle = color('--line-strong');
      ctx.lineWidth = 1;
      ctx.beginPath();
      for (const [[ax, ay], [bx, by]] of data.segments) {
        const [x1, y1] = toScreen(ax, ay);
        const [x2, y2] = toScreen(bx, by);
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
      }
      ctx.stroke();
    }

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

    if (visible.truth && data.sensor_true) cloud(data.sensor_true, color('--truth'), 0.9);
    if (visible.unaligned) {
      const shown =
        dragPose && data.initial_pose
          ? transformCloud(data.sensor_unaligned, data.initial_pose, dragPose)
          : data.sensor_unaligned;
      cloud(shown, color('--raw'), 0.75);
    }
    if (visible.reference) cloud(data.reference, color('--ref'), 0.9);
    if (visible.aligned && data.sensor_aligned && !dragPose)
      cloud(
        animT < 1 && data.estimated_pose && data.initial_pose
          ? transformCloud(
              data.sensor_aligned,
              data.estimated_pose,
              tweenPose(data.initial_pose, data.estimated_pose, animT),
            )
          : data.sensor_aligned,
        color('--aligned'),
        0.95,
      );
    if (visible.alignedB && data.sensor_aligned_b && !dragPose)
      cloud(
        animT < 1 && data.estimated_pose_b && data.initial_pose
          ? transformCloud(
              data.sensor_aligned_b,
              data.estimated_pose_b,
              tweenPose(data.initial_pose, data.estimated_pose_b, animT),
            )
          : data.sensor_aligned_b,
        color('--aligned-b'),
        0.95,
      );

    if (visible.truth && data.truth_pose) drawArrow(ctx, data.truth_pose, color('--truth'));
    if (visible.unaligned && (dragPose ?? data.initial_pose))
      drawArrow(ctx, (dragPose ?? data.initial_pose) as [number, number, number], color('--raw'));
    if (visible.aligned && data.estimated_pose && !dragPose)
      drawArrow(ctx, data.estimated_pose, color('--aligned'));
    if (visible.alignedB && data.estimated_pose_b)
      drawArrow(ctx, data.estimated_pose_b, color('--aligned-b'));

    // Sequence trajectories: truth and the estimate actually travelled.
    if (data.trajectory_true) drawPath(ctx, data.trajectory_true, color('--truth'));
    if (data.trajectory_estimate) drawPath(ctx, data.trajectory_estimate, color('--aligned'));

    // A rejected update is marked, never drawn as a successful move.
    if (data.rejected && data.estimated_pose) {
      const [sx, sy] = toScreen(data.estimated_pose[0], data.estimated_pose[1]);
      ctx.strokeStyle = color('--danger');
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(sx, sy, 9, 0, Math.PI * 2);
      ctx.stroke();
    }

    // Correspondences of the selected traced iteration, as real pairs.
    const trace = $activeFrame?.trace;
    if (visible.correspondences && trace && trace.length > 0) {
      const iteration = trace[Math.min($traceIteration, trace.length - 1)];
      if (iteration) {
        ctx.strokeStyle = color('--muted');
        ctx.lineWidth = 1;
        ctx.globalAlpha = 0.5;
        ctx.beginPath();
        for (const correspondence of iteration.correspondences) {
          const [x1, y1] = toScreen(correspondence.sensor_point[0], correspondence.sensor_point[1]);
          const [x2, y2] = toScreen(
            correspondence.reference_point[0],
            correspondence.reference_point[1],
          );
          ctx.moveTo(x1, y1);
          ctx.lineTo(x2, y2);
        }
        ctx.stroke();
        ctx.globalAlpha = 1;
      }
    }
  }

  onMount(() => {
    const observer = new ResizeObserver((entries) => {
      const rect = entries[0].contentRect;
      cssW = rect.width;
      cssH = rect.height;
    });
    observer.observe(wrap);
    return () => observer.disconnect();
  });

  $effect(() => {
    // Track every input that changes the picture.
    void $view;
    void $activeFrame;
    void $traceIteration;
    void $layers;
    void $theme;
    void $placing;
    void $placed;
    void cssW;
    void cssH;
    void zoom;
    void panX;
    void panY;
    void animT;
    void dragPose;
    draw();
  });

  $effect(() => {
    // One animation per new result or sequence frame, keyed by identity so
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
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / ANIM_MS);
      animT = t;
      if (t < 1) animHandle = requestAnimationFrame(tick);
    };
    animHandle = requestAnimationFrame(tick);
  });

  $effect(() => {
    // Fit once per new run or sequence, never on scrub or preview refresh.
    const seqLen = $sequence?.frames.length ?? 0;
    const key = `${poseOf($result?.request_id)}:${poseOf($compare?.request_id)}:${seqLen}`;
    if (!key || key === fitKey || cssW <= 0) return;
    fitKey = key;
    resetView();
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

  <fieldset class="legend" aria-label="Scan layers">
    <label><input type="checkbox" bind:checked={$layers.walls} /> walls</label>
    <label><input type="checkbox" bind:checked={$layers.reference} /> <i class="sw ref"></i>reference</label>
    <label><input type="checkbox" bind:checked={$layers.unaligned} /> <i class="sw raw"></i>raw sensor</label>
    <label><input type="checkbox" bind:checked={$layers.aligned} /> <i class="sw aligned"></i>aligned A</label>
    <label><input type="checkbox" bind:checked={$layers.alignedB} /> <i class="sw aligned-b"></i>aligned B</label>
    <label><input type="checkbox" bind:checked={$layers.truth} /> <i class="sw truth"></i>truth</label>
    <label><input type="checkbox" bind:checked={$layers.correspondences} /> correspondences</label>
    <button class="reset" onclick={resetView}>reset view</button>
    <div class="place">
      <button class:active={$placing} onclick={() => setPlacing(!$placing)}>
        {$placing ? 'Placing scan' : 'Place scan'}
      </button>
      {#if $placed}
        <button onclick={resetPlacement}>Reset placement</button>
      {/if}
    </div>
    {#if $placing}
      <p class="tip">Drag to move the scan, shift-drag to rotate. Release to re-run.</p>
    {/if}
  </fieldset>
</div>

<style>
  .plot {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 320px;
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
    top: 10px;
    left: 10px;
    margin: 0;
    padding: 8px 10px;
    display: grid;
    gap: 3px;
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    font-size: 12px;
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

  .sw.ref { background: var(--ref); }
  .sw.raw { background: var(--raw); }
  .sw.aligned { background: var(--aligned); }
  .sw.aligned-b { background: var(--aligned-b); }
  .sw.truth { background: var(--truth); }

  .reset {
    margin-top: 4px;
    font-size: 11px;
    padding: 2px 6px;
  }

  .place {
    display: flex;
    gap: 4px;
    margin-top: 4px;
  }

  .place button {
    font-size: 11px;
    padding: 2px 6px;
  }

  .place button.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 20%, var(--surface-2));
  }

  .tip {
    margin: 4px 0 0;
    max-width: 190px;
    font-size: 11px;
    color: var(--muted);
  }

  canvas.placing {
    cursor: crosshair;
  }
</style>