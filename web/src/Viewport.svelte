<script lang="ts">
  import { t } from "./core";
  import type { GearView } from "./state.svelte";
  // Draws what the core produced. It computes no geometry: the profile arrives
  // as a flat [x0, y0, x1, y1, ...] array and the reference radii as numbers.
  // Only view transform lives here.

  let {
    points,
    pitch,
    base,
    tip,
    root,
    rim = null,
    view = $bindable(),
  }: {
    points: Float64Array | null;
    pitch: number;
    base: number;
    tip: number;
    root: number;
    /** Set for a ring: the radius its rim is drawn out to, from Rust. The
     *  outline is then the *bore*, and what is shaded is the material around
     *  it. Omitted for an external gear, where the outline is the part. */
    rim?: number | null;
    /** Where the reader is looking, held by the tab rather than here.
     *
     *  This component is destroyed and rebuilt every time the reader looks at
     *  another gear, so state it owned would be state a glance away from the
     *  panel erased. Bound rather than passed: the pointer and the wheel are
     *  here, so this is the component that *writes* it. */
    view: GearView;
  } = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let dragging = false;
  let lastX = 0;
  let lastY = 0;

  function reset() {
    view.zoom = 1;
    view.panX = 0;
    view.panY = 0;
  }

  /** Zoom by `factor` about a point given in pixels from the canvas centre, so
   *  the feature under the cursor stays under it.
   *
   *  A screen point maps to the drawing through `screen = centre + pan +
   *  scale · point`; holding one screen point fixed across a change of scale is
   *  that relation solved for the new pan. Nothing about the gear is computed
   *  here — this is the view transform, and it composes: zooming in and back
   *  out about the same point returns to exactly where it started. */
  function zoomAbout(cx: number, cy: number, factor: number) {
    const before = view.zoom;
    view.zoom = Math.min(50, Math.max(0.2, view.zoom * factor));
    if (view.zoom === before) return;
    // The pan that keeps the drawing point under the cursor where it is: with
    // `cx = panX + s·x`, the same `x` at the new scale needs
    // `panX' = cx − (cx − panX)·s'/s`. The y axis is flipped in the transform,
    // but the flip cancels in that ratio, so both axes take the same form.
    const growth = view.zoom / before;
    view.panX = cx - (cx - view.panX) * growth;
    view.panY = cy - (cy - view.panY) * growth;
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const c = canvas;
    if (!c) return;
    const rect = c.getBoundingClientRect();
    zoomAbout(
      e.clientX - rect.left - rect.width / 2,
      e.clientY - rect.top - rect.height / 2,
      Math.exp(-e.deltaY / 400),
    );
  }
  function onDown(e: PointerEvent) {
    dragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
    (e.currentTarget as HTMLCanvasElement).setPointerCapture(e.pointerId);
  }
  function onMove(e: PointerEvent) {
    if (!dragging) return;
    view.panX += e.clientX - lastX;
    view.panY += e.clientY - lastY;
    lastX = e.clientX;
    lastY = e.clientY;
  }
  function onUp() {
    dragging = false;
  }

  $effect(() => {
    const c = canvas;
    if (!c || !points || points.length < 4) return;
    // referenced so the effect re-runs on view changes
    void [view.zoom, view.panX, view.panY, view.circles, pitch, base, tip, root, rim];

    const dpr = window.devicePixelRatio || 1;
    const w = c.clientWidth;
    const h = c.clientHeight;
    c.width = Math.round(w * dpr);
    c.height = Math.round(h * dpr);
    const ctx = c.getContext("2d");
    if (!ctx) return;

    const style = getComputedStyle(c);
    const token = (n: string, fallback: string) =>
      style.getPropertyValue(n).trim() || fallback;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    // Frame whatever the part actually reaches: a ring's tip radius is its
    // *smallest*, so scaling by it drew the rim off the edge of the canvas.
    const extent = Math.max(tip, root, rim ?? 0, 1e-9);
    const scale = ((Math.min(w, h) * 0.45) / extent) * view.zoom;
    ctx.translate(w / 2 + view.panX, h / 2 + view.panY);
    ctx.scale(scale, -scale); // +y up, the way a drawing is read
    ctx.lineWidth = 1 / scale;

    if (view.circles) {
      ctx.setLineDash([4 / scale, 4 / scale]);
      ctx.strokeStyle = token("--reference", "#888");
      for (const r of [root, base, pitch, tip]) {
        ctx.beginPath();
        ctx.arc(0, 0, r, 0, Math.PI * 2);
        ctx.stroke();
      }
      ctx.setLineDash([]);
    }

    ctx.beginPath();
    ctx.moveTo(points[0], points[1]);
    for (let i = 2; i < points.length; i += 2) ctx.lineTo(points[i], points[i + 1]);
    ctx.closePath();

    if (rim !== null) {
      // A ring's material is *outside* its bore. Adding the rim circle to the
      // same path and filling by the even-odd rule shades the annulus between
      // the two, which is what makes a ring look like a ring rather than like
      // an external gear with the teeth drawn the other way up.
      ctx.moveTo(rim, 0);
      ctx.arc(0, 0, rim, 0, Math.PI * 2);
      ctx.fillStyle = token("--flank", "#dbe3ec");
      ctx.fill("evenodd");
      ctx.strokeStyle = token("--accent", "#2f5d8a");
      ctx.stroke();
    } else {
      ctx.fillStyle = token("--flank", "#dbe3ec");
      ctx.fill();
      ctx.strokeStyle = token("--accent", "#2f5d8a");
      ctx.stroke();
    }

  });
</script>

<div class="wrap">
  <canvas
    bind:this={canvas}
    onwheel={onWheel}
    onpointerdown={onDown}
    onpointermove={onMove}
    onpointerup={onUp}
    onpointercancel={onUp}
  ></canvas>
  <div class="bar">
    <label><input type="checkbox" bind:checked={view.circles} />
      {t("ui.viewport_reference_circles")}</label>
    <span class="hint">{t("ui.viewport_drag_pan_scroll_zoom")}</span>
    <button onclick={reset}>{t("ui.viewport_reset_view")}</button>
  </div>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  canvas {
    width: 100%;
    aspect-ratio: 1;
    max-height: 58vh;
    border: 1px solid var(--rule);
    border-radius: 4px;
    background: var(--bg);
    touch-action: none;
    cursor: grab;
  }
  canvas:active {
    cursor: grabbing;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .bar label {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .hint {
    margin-left: auto;
  }
  button {
    font: inherit;
    font-size: 0.75rem;
    padding: 0.15rem 0.5rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: var(--panel);
    color: var(--fg);
    cursor: pointer;
  }
</style>
