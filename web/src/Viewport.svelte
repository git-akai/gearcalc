<script lang="ts">
  // Draws the profile the core produced. It computes nothing: the points arrive
  // as a flat [x0, y0, x1, y1, ...] array and are only scaled to fit.
  let { points }: { points: Float64Array | null } = $props();

  let canvas: HTMLCanvasElement | undefined = $state();

  $effect(() => {
    const c = canvas;
    if (!c || !points || points.length < 4) return;

    const dpr = window.devicePixelRatio || 1;
    const w = c.clientWidth;
    const h = c.clientHeight;
    c.width = Math.round(w * dpr);
    c.height = Math.round(h * dpr);

    const ctx = c.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    let extent = 0;
    for (let i = 0; i < points.length; i++) {
      const v = Math.abs(points[i]);
      if (v > extent) extent = v;
    }
    if (extent === 0) return;

    const scale = (Math.min(w, h) * 0.45) / extent;
    const style = getComputedStyle(c);
    ctx.translate(w / 2, h / 2);
    ctx.scale(scale, -scale); // +y up, the way a drawing is read

    ctx.beginPath();
    ctx.moveTo(points[0], points[1]);
    for (let i = 2; i < points.length; i += 2) ctx.lineTo(points[i], points[i + 1]);
    ctx.closePath();

    ctx.fillStyle = style.getPropertyValue("--flank").trim() || "#cbd5e1";
    ctx.fill();
    ctx.lineWidth = 1 / scale;
    ctx.strokeStyle = style.getPropertyValue("--accent").trim() || "#2563eb";
    ctx.stroke();
  });
</script>

<canvas bind:this={canvas}></canvas>

<style>
  canvas {
    width: 100%;
    aspect-ratio: 1;
    max-height: 60vh;
    border: 1px solid var(--rule);
    border-radius: 4px;
    background: var(--bg);
  }
</style>
