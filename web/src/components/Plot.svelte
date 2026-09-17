<script lang="ts">
  import { onMount } from 'svelte';
  import { activeFrame, layers, theme, traceIteration, view } from '../lib/state';

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
    panX = 0;
    panY = 0;
    zoom = 1;
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
    dragging = true;
    lastX = event.clientX;
    lastY = event.clientY;
    canvas.setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragging) return;
    const s = fitScale() * zoom;
    panX -= (event.clientX - lastX) / s;
    panY += (event.clientY - lastY) / s;
    lastX = event.clientX;
    lastY = event.clientY;
  }

  function onPointerUp(event: PointerEvent) {
    dragging = false;
    canvas.releasePointerCapture(event.pointerId);
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

    if (visible.truth) cloud(data.sensor_true, color('--truth'), 0.9);
    if (visible.unaligned) cloud(data.sensor_unaligned, color('--raw'), 0.75);
    if (visible.reference) cloud(data.reference, color('--ref'), 0.9);
    if (visible.aligned && data.sensor_aligned) cloud(data.sensor_aligned, color('--aligned'), 0.95);
    if (visible.alignedB && data.sensor_aligned_b)
      cloud(data.sensor_aligned_b, color('--aligned-b'), 0.95);

    if (visible.truth) drawArrow(ctx, data.truth_pose, color('--truth'));
    if (visible.unaligned) drawArrow(ctx, data.initial_pose, color('--raw'));
    if (visible.aligned && data.estimated_pose) drawArrow(ctx, data.estimated_pose, color('--aligned'));
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
    void cssW;
    void cssH;
    void zoom;
    void panX;
    void panY;
    draw();
  });
</script>

<div class="plot" bind:this={wrap}>
  <canvas
    bind:this={canvas}
    style="width: {cssW}px; height: {cssH}px"
    onwheel={onWheel}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    ondblclick={resetView}
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
</style>