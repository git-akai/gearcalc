<script lang="ts">
  import {
    FIELDS,
    KINDS,
    defaults,
    dxf,
    isUnavailable,
    profile,
    solve,
    validate,
    boundFor,
    ringDxf,
    ringProfile,
    solveRing,
    type GearRequest,
    type RingRequest,
    type Maybe,
    type PinsOut,
    type FieldSpec,
    type ShiftRange,
    type GearKind,
    note,
    t,
  } from "./core";
  import { developer, setKind, workspace, type GearTab as Tab } from "./state.svelte";
  import Viewport from "./Viewport.svelte";

  let { tab }: { tab: Tab } = $props();

  // Raw text per field, so a half-typed "-" or "1e" never reaches the solver.
  // The last valid value stays in `tab.params`, which is what Rust sees.
  let raw = $state<Record<string, string>>(
    Object.fromEntries(FIELDS.map((f) => [f.key, String(tab.params[f.key])])),
  );
  let errors = $state<Record<string, string | null>>({});

  /** The bound, phrased for the field it belongs to.
   *
   *  The sentences live in the catalogue rather than here, because the
   *  geartrain tab shows the same bounds and the two wordings drifted while
   *  each had its own copy. */
  function boundNote(key: string): string | null {
    if (!("ok" in result)) return null;
    const r = result.ok.ranges;
    switch (key) {
      case "addendum":
        return r.addendum.min === null
          ? null
          : swept(t("ui.bound_addendum", { min: n(r.addendum.min) }));
      case "dedendum":
        return r.dedendum.max === null
          ? null
          : swept(
              t("ui.bound_dedendum", {
                min: n(r.dedendum.min ?? 0),
                max: n(r.dedendum.max),
              }),
            );
      case "root_radius":
        return r.root_radius.max === null
          ? null
          : t(eccentric ? "ui.bound_root_radius_shared_cutter" : "ui.bound_root_radius", {
              max: n(r.root_radius.max),
            });
      // The amplitude's own bound, which is about the *tool* rather than about
      // any one tooth: past it no single cutter reaches the high tooth without
      // driving the low one's root into the axis.
      case "angular_shift":
        return r.angular_shift.max === null
          ? null
          : t("ui.bound_angular_shift", { max: n(r.angular_shift.max) });
      default:
        return null;
    }
  }

  /** A field's note, and every note it could be showing instead.
   *
   *  All of them are rendered, stacked in one grid cell, with the ones that do
   *  not apply hidden — so the slot is as tall as the tallest note the field
   *  can produce *at this width*, and a note appearing or disappearing moves
   *  nothing below it. The blank first candidate is what reserves the space on
   *  a field that has no note at all right now: an error message arriving as
   *  you type is exactly the case that used to shift the whole column.
   *
   *  Sized by the browser rather than by a line count written down here, so it
   *  stays right at any window width and cannot be made stale by editing the
   *  text of a note. */
  type Note = { text: string; err?: boolean };

  function notesFor(f: FieldSpec): { all: Note[]; shown: number } {
    const all: Note[] = [{ text: "\u00a0" }];
    const normal =
      f.key === "profile_shift" && "ok" in result
        ? shiftNote(result.ok.ranges.profile_shift)
        : (boundNote(f.key) ?? (internal && f.ringNote ? t(f.ringNote) : (f.note ? t(f.note) : null)));
    if (normal) all.push({ text: normal });
    const err = errors[f.key];
    if (err) all.push({ text: err, err: true });
    return { all, shown: err ? all.length - 1 : normal ? 1 : 0 };
  }

  /** The profile shift's three bounds, as one line of text. For an eccentric
   *  gear these are the window the *nominal* shift x̄ can sit in so that every
   *  swept tooth still builds. */
  function shiftNote(r: ShiftRange): string {
    // The range, then one clause per threshold that has something to say. Built
    // from parts rather than as one sentence per combination: there are three
    // thresholds now and a sentence each would be eight strings to translate,
    // seven of which restate the others.
    const parts = [
      t("ui.bound_profile_shift", {
        min: n(r.bound.min ?? 0),
        max: n(r.bound.max ?? 0),
        undercut: n(r.undercut),
        sharp: n(r.sharp_rack_undercut),
      }),
    ];
    if (r.shallow_cut < (r.bound.max ?? Infinity))
      parts.push(t("ui.bound_profile_shift_deep_cut", { deep: n(r.shallow_cut) }));
    if (r.pointed !== null)
      parts.push(t("ui.bound_profile_shift_pointed", { pointed: n(r.pointed) }));
    return swept(parts.join(" · "));
  }

  function onInput(key: string, text: string) {
    raw[key] = text;
    const f = FIELDS.find((f) => f.key === key)!;
    const v = Number(text);
    const b = "ok" in result ? boundFor(f.key, result.ok.ranges) : null;
    const err = text.trim() === "" ? t("ui.validation_required") : validate(f, v, b);
    errors[key] = err;
    if (!err) tab.params[f.key] = v;
  }

  const n = (v: number) => v.toFixed(3);
  /** "lo to hi" — one shape, so the word between two numbers is written once. */
  const range = (lo: string, hi: string) => t("ui.range", { lo, hi });
  /** What a measurement takes around the revolution, where that is more than one
   *  value.
   *
   *  The two ends are the same bits for an ordinary gear — Rust guarantees it,
   *  rather than this side testing a kind — so the comparison is exact and an
   *  evenly cut gear simply shows nothing extra. */
  const around = (r: [number, number]) =>
    r[0] === r[1] ? "" : " · " + t("ui.gear_measurement_around", { lo: n(r[0]), hi: n(r[1]) });
  const request = $derived<GearRequest>({
    params: tab.params,
    pin_diameter: tab.pinDiameter > 0 ? tab.pinDiameter : undefined,
    tolerance_class: tab.toleranceClass ?? undefined,
    chord_tolerance: tab.chordTolerance,
    reference_circles: tab.referenceCircles,
    // Sent only when it can mean something. A concentric gear commands one
    // centre distance and the core says so rather than profiling a constant.
    // Absent rather than null: Rust takes these as `Option` with `serde(default)`,
    // which the generated type states as an optional field.
    mate: tab.kind === "eccentric" ? tab.mate : undefined,
    // When set, Rust solves `angular_shift` from it — see `resolved_params`.
    eccentric_throw:
      tab.kind === "eccentric" && tab.eccentricThrow !== null ? tab.eccentricThrow : undefined,
  });

  const result = $derived(solve(request));

  // A ring is a different part with different answers, so it gets its own
  // request and its own summary rather than optional fields on the gear's.
  const ringRequest = $derived<RingRequest>({
    params: tab.params,
    // The gear tab's pin box serves both kinds: over-pins outside, between-pins
    // inside.
    pin_diameter: tab.pinDiameter,
    cutter: tab.cutter,
    chord_tolerance: tab.chordTolerance,
    reference_circles: tab.referenceCircles,
  });
  // The ring path is unchanged; `internal` is now one of three kinds rather
  // than a boolean, and this is the only place that difference is spent.
  const internal = $derived(tab.kind === "internal");
  const eccentric = $derived(tab.kind === "eccentric");
  const ring = $derived(internal ? solveRing(ringRequest) : null);

  /** The kinds this tab may be switched to, and the note under the one it holds.
   *
   *  A kind marked `developer` is not in the list until the mode is knocked for
   *  in the sidebar. The tab's own kind cannot fall out of the list underneath
   *  it: the picker is the only way a tab becomes eccentric, and the mode does
   *  not close (`Developer` in `state.svelte.ts`). */
  const kinds = $derived(KINDS.filter((k) => !k.developer || developer.enabled));
  const kindNote = $derived(KINDS.find((k) => k.key === tab.kind)?.note);

  /** For an eccentric gear the input bounds are for the whole swept interval,
   *  not the nominal tooth — this says so, once, wherever a bound is shown.
   *  It wraps the bound rather than being glued onto it, so the catalogue holds
   *  a whole sentence and a translator can put the qualifier where it belongs. */
  const swept = $derived((bound: string) =>
    eccentric
      ? t("ui.bound_every_tooth_cut_at", {
          bound,
          amplitude: n(
            Math.abs("ok" in result ? result.ok.angular_shift : tab.params.angular_shift),
          ),
        })
      : bound,
  );

  // Kept as a typed array rather than an inline tuple list: destructuring a
  // mixed tuple inside {#each} widens both members to their union and loses the
  // field types.
  const pinRows = $derived<{ label: string; value: Maybe<PinsOut> }[]>(
    "ok" in result
      ? [
          { label: t("ui.gear_two_pins"), value: result.ok.over_two_pins },
          { label: t("ui.gear_three_pins"), value: result.ok.over_three_pins },
        ]
      : [],
  );
  const outline = $derived(
    internal
      ? ring && "ok" in ring
        ? ringProfile(ringRequest, 600)
        : null
      : "ok" in result
        ? profile(request, 600)
        : null,
  );

  let confirmingDelete = $state(false);

  let exportError = $state<string | null>(null);

  function saveDxf() {
    const r = internal ? ringDxf(ringRequest) : dxf(request);
    if ("error" in r) {
      // A click that silently does nothing is the worst of the three outcomes:
      // the user cannot tell a refusal from a broken button.
      exportError = r.error;
      return;
    }
    exportError = null;
    const blob = new Blob([r.ok], { type: "application/dxf" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    const kind = internal ? "ring" : "gear";
    a.download = `${(tab.name || kind).replace(/\s+/g, "_")}_m${tab.params.module}_z${tab.params.teeth}.dxf`;
    a.click();
    URL.revokeObjectURL(url);
  }

  const mm = (v: number) => `${v.toFixed(4)} mm`;
  const um = (v: number) => `${v.toFixed(1)} µm`;
</script>

<header>
  <input class="title" bind:value={tab.name} aria-label={t("ui.gear_name")} />
  <div class="actions">
    <button onclick={() => workspace.copy(tab.id)}>{t("ui.gear_copy")}</button>
    <button onclick={() => workspace.create()}>{t("ui.gear_new")}</button>
    <button class="danger" onclick={() => (confirmingDelete = true)}>{t("ui.gear_delete")}</button>
  </div>
</header>

{#if confirmingDelete}
  <div class="confirm" role="alertdialog">
    <span>{t("ui.gear_delete_question", { name: tab.name || t("ui.gear_unnamed") })}</span>
    <button
      class="danger"
      onclick={() => {
        workspace.remove(tab.id);
        confirmingDelete = false;
      }}>{t("ui.gear_delete_confirm")}</button
    >
    <button onclick={() => (confirmingDelete = false)}>{t("ui.gear_cancel")}</button>
  </div>
{/if}

<div class="columns">
  <section class="inputs">
    <h2>{t("ui.gear_parameters")}</h2>
    <div class="grid">
      <label class="wide">
        <span>{t("ui.gear_kind")}</span>
        <select
          value={tab.kind}
          onchange={(e) => setKind(tab, e.currentTarget.value as GearKind)}
        >
          {#each kinds as k (k.key)}
            <option value={k.key}>{t(k.label)}</option>
          {/each}
        </select>
        <small>{kindNote ? t(kindNote) : ""}</small>
      </label>
    </div>
    <div class="grid">
      {#each FIELDS.filter((f) => !f.kinds || f.kinds.includes(tab.kind)) as f (f.key)}
        <!-- The mate is what the commanded centre distance is commanded
             *against*, so it sits with the eccentricity it belongs to rather
             than above the gear's own parameters. -->
        {#if f.key === "angular_shift"}
          <label>
            <span>{t("ui.gear_mate_teeth")}</span>
            <input type="number" step="1" min="1" bind:value={tab.mate.teeth} />
            <small>{t("ui.gear_mate_shares_module_angle_helix")}</small>
          </label>
          <label>
            <span>{t("ui.gear_mate_profile_shift")}</span>
            <input type="number" step="0.05" bind:value={tab.mate.profile_shift} />
            <em>{t("ui.gear_m")}</em>
          </label>
          <label class="check">
            <input type="checkbox" bind:checked={tab.mate.internal} />
            <span>{t("ui.gear_mate_is_a_ring")}</span>
            <small>{t("ui.gear_mate_ring_runs_inside")}</small>
          </label>
          <!-- The shift amplitude and the centre-distance throw are the same
               eccentricity, one solved from the other — like a worm stage sized
               by helix angle or pitch diameter. Switching seeds the new input
               from the geometry so nothing jumps. -->
          <label>
            <span>{t("ui.gear_eccentric_sized_by")}</span>
            <select
              value={tab.eccentricThrow === null ? "amplitude" : "throw"}
              onchange={(e) => {
                if (e.currentTarget.value === "throw") {
                  const cp = "ok" in result ? result.ok.centre_profile : null;
                  tab.eccentricThrow =
                    cp && !isUnavailable(cp)
                      ? cp.sinusoid.amplitude
                      : defaults().gear.eccentric_throw;
                } else {
                  if ("ok" in result) {
                    tab.params = { ...tab.params, angular_shift: result.ok.angular_shift };
                    raw.angular_shift = String(result.ok.angular_shift);
                  }
                  tab.eccentricThrow = null;
                }
              }}
            >
              <option value="amplitude">{t("ui.gear_eccentric_by_amplitude")}</option>
              <option value="throw">{t("ui.gear_eccentric_by_throw")}</option>
            </select>
          </label>
          {#if tab.eccentricThrow !== null}
            <label class:invalid={"error" in result}>
              <span>{t("ui.gear_centre_distance_throw")}</span>
              <input type="number" step="0.05" bind:value={tab.eccentricThrow} />
              <em>{t("ui.gear_mm")}</em>
              <span class="note">
                <small class:err={"error" in result}>
                  {"error" in result ? result.error : t("ui.gear_throw_solves_the_amplitude")}
                </small>
              </span>
            </label>
          {/if}
        {/if}
        {#if f.key !== "angular_shift" || tab.eccentricThrow === null}
          {@const notes = notesFor(f)}
          <label class:invalid={errors[f.key]}>
            <span>{t(f.label)}</span>
            <input
              type="number"
              step={f.step}
              value={raw[f.key]}
              oninput={(e) => onInput(f.key, e.currentTarget.value)}
            />
            <em>{f.unit ? t(f.unit) : ""}</em>
            <span class="note">
              {#each notes.all as note, i (i)}
                <small class:err={note.err} class:hidden={i !== notes.shown}>{note.text}</small>
              {/each}
            </span>
          </label>
        {/if}
      {/each}
    </div>
    <!-- The cutter comes last, under the thickness modification, because it is
         the one group here that is not a dimension of the gear: a ring has no
         dedendum and no root-radius coefficient, it has a *tool*, and where the
         tool reaches is what those become (docs/reference.md#internal-gears).
         Reading it after the gear's own parameters also lines the ring's panel
         up with the external one, whose last field is the same thickness
         modification. -->
    {#if internal}
      <div class="grid">
        <label>
          <span>{t("ui.gear_cutter_teeth")}</span>
          <input type="number" step="1" min="1" bind:value={tab.cutter.teeth} />
        </label>
        <label>
          <span>{t("ui.gear_cutter_addendum")}</span>
          <input type="number" step="0.05" bind:value={tab.cutter.addendum} />
          <em>{t("ui.gear_m")}</em>
        </label>
        <label>
          <span>{t("ui.gear_cutter_tip_round")}</span>
          <input type="number" step="0.02" bind:value={tab.cutter.tip_round} />
          <em>{t("ui.gear_m")}</em>
        </label>
      </div>
    {/if}

    <h2>{t("ui.gear_measurement")}</h2>
    <div class="grid">
      <label>
        <span>{t("ui.gear_pin_ball_diameter")}</span>
        <input type="number" step="0.05" bind:value={tab.pinDiameter} />
        <em>{t("ui.gear_mm")}</em>
      </label>
      {#if "ok" in result}
        <label>
          <span>{t("ui.gear_tolerance_class")}</span>
          <select
            value={result.ok.tolerance && !isUnavailable(result.ok.tolerance)
              ? `${result.ok.tolerance.class.scale}:${result.ok.tolerance.class.grade}`
              : ""}
            onchange={(e) => {
              const [scale, grade] = e.currentTarget.value.split(":");
              tab.toleranceClass = scale
                ? { scale: scale as "fine" | "standard", grade: Number(grade) }
                : null;
            }}
          >
            {#each result.ok.available_classes as c}
              <option value="{c.scale}:{c.grade}">
                {c.scale === "fine" ? "Fine" : "Standard"}
                {c.grade}
              </option>
            {:else}
              <option value="">{t("ui.gear_none_available")}</option>
            {/each}
          </select>
          <em></em>
        </label>
      {/if}
    </div>

    <h2>{t("ui.gear_export")}</h2>
    <div class="grid">
      <label>
        <span>{t("ui.gear_chord_tolerance")}</span>
        <input type="number" step="0.0005" min="0" bind:value={tab.chordTolerance} />
        <em>{t("ui.gear_mm")}</em>
        <small>{t("ui.gear_maximum_deviation_exported_outline_from_true")}</small>
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={tab.referenceCircles} />
        <span>{t("ui.gear_include_reference_circles")}</span>
      </label>
    </div>
    <button class="primary" onclick={saveDxf} disabled={!("ok" in result)}>{t("ui.gear_export_dxf")}</button>
    {#if exportError}
      <p class="error">{t("ui.gear_export_failed", { reason: exportError })}</p>
    {/if}
  </section>

  <section class="results">
    {#if internal}
      {#if ring && "error" in ring}
        <p class="error">{ring.error}</p>
      {:else if ring && "ok" in ring}
        {@const r = ring.ok}
        <Viewport
          points={outline}
          pitch={r.pitch_radius}
          base={r.base_radius}
          tip={r.tip_radius}
          root={r.root_radius}
          rim={r.rim_radius}
          bind:view={tab.view}
        />
        <h2>{t("ui.gear_geometry")}</h2>
        <dl>
          <dt>{t("ui.gear_transverse_module")}</dt>
          <dd>{mm(r.transverse_module)}</dd>
          <dt>{t("ui.gear_transverse_pressure_angle")}</dt>
          <dd>{r.transverse_pressure_angle.toFixed(4)}°</dd>
          <dt>{t("ui.gear_pitch_diameter")}</dt>
          <dd>{mm(r.pitch_diameter)}</dd>
          <dt>{t("ui.gear_base_diameter")}</dt>
          <dd>{mm(r.base_diameter)}</dd>
          <dt>{t("ui.gear_tip_diameter")}</dt>
          <dd>{mm(r.tip_diameter)} <small>{t("ui.gear_inside_pitch_circle")}</small></dd>
          <dt>{t("ui.gear_root_diameter")}</dt>
          <dd>{mm(r.root_diameter)} <small>{t("ui.gear_outside")}</small></dd>
          <dt>{t("ui.gear_flank_fillet_junction")}</dt>
          <dd>
            {#if r.junction_radius === null}
              <span class="warn">{t("ui.gear_none_this_cutter_generated_no_fillet")}</span>
            {:else}
              {mm(r.junction_radius)}
            {/if}
          </dd>
          <dt>{t("ui.gear_generated_down")}</dt>
          <dd>
            {mm(r.generation_limit)}
            {#if !r.fully_generated}
              <small class="warn">{t("ui.gear_below_tip_cutter_s_own_involute")}</small>
            {:else}
              <small>{t("ui.gear_past_tip_so_whole_flank_generated")}</small>
            {/if}
          </dd>
          <dt>{t("ui.gear_smallest_tooth_count")}</dt>
          <dd>
            {r.smallest_tooth_count}
            <small>{t("ui.gear_below_this_tip_would_reach_inside")}</small>
          </dd>
        </dl>
        {#if r.clamps.length}
          <ul class="notes">
            {#each r.clamps as c (c.key)}<li>{t("ui.gear_clamped")} {note(c)}</li>{/each}
          </ul>
        {/if}
        <!-- Same heading and same row as the external gear's, because it is the
             same measurement read at the opposite sign. That a ring has no span
             over teeth is not noted: absence needs saying only where the thing
             was expected, and nothing here offers one. -->
        <h2>{t("ui.gear_measurement_between_pins")}</h2>
        <dl>
          <dt>{t("ui.gear_two_pins")}</dt>
          <dd>
            {#if isUnavailable(r.between_pins)}
              <span class="na">{note(r.between_pins.unavailable)}</span>
            {:else}
              {mm(r.between_pins.nominal)}
            {/if}
          </dd>
        </dl>
      {/if}
    {:else if "error" in result}
      <p class="error">{result.error}</p>
    {:else}
      {@const s = result.ok}
      <Viewport
        points={outline}
        pitch={s.pitch_radius}
        base={s.base_radius}
        tip={s.tip_radius}
        root={s.root_radius}
        bind:view={tab.view}
      />

      <!-- For an eccentric gear undercut/severed are per-tooth: the Eccentricity
           section below names which teeth and where. Only the tool-level clamps
           belong here. -->
      <!-- `clamp.tooth_severed` is the same event as the line above it, and the
           core pushes it into `clamps` as well as setting `severed` — so it is
           dropped here rather than said twice. It still carries its own weight
           in the eccentric gear's per-tooth list, where there is no bool. -->
      {@const clamps = s.clamps.filter((c) => c.key !== "clamp.tooth_severed")}
      {#if (!eccentric && (s.undercut || s.severed)) || clamps.length}
        <ul class="notes">
          {#if !eccentric && s.undercut}<li>{t("ui.gear_undercut")}</li>{/if}
          {#if !eccentric && s.severed}<li>{t("ui.gear_severed")}</li>{/if}
          {#each clamps as c}<li>{t("ui.gear_clamped")} {note(c)}</li>{/each}
        </ul>
      {/if}

      <h2>{t("ui.gear_geometry")}</h2>
      <!-- Tip/root diameter and tooth thickness vary around an eccentric gear;
           they are shown as ranges in the Eccentricity section rather than as a
           mean-tooth scalar here. -->
      <dl>
        <dt>{t("ui.gear_pitch_diameter")}</dt><dd>{mm(s.pitch_diameter)}</dd>
        <dt>{t("ui.gear_base_diameter")}</dt><dd>{mm(s.base_diameter)}</dd>
        {#if !eccentric}
          <dt>{t("ui.gear_tip_diameter")}</dt><dd>{mm(s.tip_diameter)}</dd>
          <dt>{t("ui.gear_root_diameter")}</dt><dd>{mm(s.root_diameter)}</dd>
          <dt>{t("ui.gear_tooth_thickness")}</dt><dd>{mm(s.tooth_thickness)}</dd>
        {/if}
        <dt>{t("ui.gear_fillet_radius")}</dt><dd>{mm(s.fillet_radius)}</dd>
        <dt>{t("ui.gear_transverse_pressure_angle")}</dt><dd>{s.transverse_pressure_angle.toFixed(4)}°</dd>
        <dt>{t("ui.gear_cutter_tip_width")}</dt><dd>{mm(s.cutter_tip_width)}</dd>
      </dl>

      <!-- Only when the gear actually varies. Every field below is zero for an
           ordinary one, so this is a question of what is worth reading rather
           than of what the core computed. -->
      {#if eccentric}
        <h2>{t("ui.gear_eccentricity")}</h2>
        {#if s.per_tooth_clamps.teeth.length}
          <!-- A guard on a tool *setting* is shared, so it trips for the whole
               gear or not at all; these are the ones true of one tooth and not
               its neighbour, and they break the envelope where they land. -->
          <ul class="notes">
            <li>
              {t("ui.gear_teeth_not_as_drawn", {
                count: String(s.per_tooth_clamps.teeth.length),
                total: String(tab.params.teeth),
                which: s.per_tooth_clamps.teeth.join(", "),
              })}
            </li>
            {#each s.per_tooth_clamps.notes as n (n.key)}<li>{note(n)}</li>{/each}
          </ul>
        {/if}
        <dl>
          {#if tab.eccentricThrow !== null}
            <dt>{t("ui.gear_shift_amplitude")}</dt>
            <dd>
              {n(s.angular_shift)} module
              <small>{t("ui.gear_throw_solves_the_amplitude")}</small>
            </dd>
          {/if}
          <dt>{t("ui.gear_envelope_eccentricity")}</dt>
          <dd>
            {mm(s.variation.eccentricity)}
            <small>{t("ui.gear_tip_root_envelopes_pitch_base_circles")}</small>
          </dd>
          <dt>{t("ui.gear_departure_from_a_circle")}</dt>
          <dd>
            {n(s.variation.circle_departure)} mm
            <small>{t("ui.gear_envelope_limacon_not_circle_e_2ro")}</small>
          </dd>
          <dt>{t("ui.gear_tip_diameter")}</dt>
          <dd>
            {range(n(2 * s.variation.tip_radius[0]), n(2 * s.variation.tip_radius[1]))} mm
            <small>{t("ui.gear_what_teeth_reach_short_2e_odd")}</small>
          </dd>
          <dt>{t("ui.gear_root_diameter")}</dt>
          <dd>
            {range(n(2 * s.variation.root_radius[0]), n(2 * s.variation.root_radius[1]))} mm
          </dd>
          <dt>{t("ui.gear_tooth_thickness")}</dt>
          <dd>
            {range(n(s.variation.tooth_thickness[0]), n(s.variation.tooth_thickness[1]))} mm
            <small>
              {t("ui.gear_at_the_base_circle", {
                lo: n(s.variation.base_thickness[0]),
                hi: n(s.variation.base_thickness[1]),
              })}
            </small>
          </dd>
          <dt>{t("ui.gear_pitch_error_drive")}</dt>
          <dd>
            {um(1e3 * s.variation.drive_pitch_error)}
            <small>
              {t("ui.gear_accumulated_drive", { value: um(1e3 * s.variation.drive_index_error) })}
            </small>
          </dd>
          <dt>{t("ui.gear_pitch_error_coast")}</dt>
          <dd>
            {um(1e3 * s.variation.coast_pitch_error)}
            <small>
              {t("ui.gear_accumulated_coast", { value: um(1e3 * s.variation.coast_index_error) })}
            </small>
          </dd>
        </dl>

        <h2>{t("ui.gear_commanded_centre_distance")}</h2>
        {#if isUnavailable(s.centre_profile)}
          <p class="aside">{note(s.centre_profile.unavailable)}</p>
        {:else}
          {@const p = s.centre_profile}
          <dl>
            <dt>{t("ui.gear_centre_distance")}</dt>
            <dd>
              {range(n(p.range[0]), n(p.range[1]))} mm
              <small>{t("ui.gear_zero_backlash_at_each_tooth")}</small>
            </dd>
            <dt>{t("ui.gear_best_fit_sinusoid")}</dt>
            <dd>
              {n(p.sinusoid.mean)} ± {n(p.sinusoid.amplitude)} mm
              <small>
                {t("ui.gear_sinusoid_phase", { phase: p.sinusoid.phase_degrees.toFixed(1) })}
              </small>
            </dd>
            <dt>{t("ui.gear_departure_from_that_sinusoid")}</dt>
            <dd>
              {um(1e3 * p.sinusoid_error)}
              <small>{t("ui.gear_ideal_not_sinusoidal_inv_cosine")}</small>
            </dd>
            <dt>{t("ui.gear_backlash_a_crank_leaves")}</dt>
            <dd>
              <span class:warn={p.sinusoid_backlash[0] < 0}>
                {range(um(1e3 * p.sinusoid_backlash[0]), um(1e3 * p.sinusoid_backlash[1]))}
              </span>
              <small>{t("ui.gear_negative_is_interference_not_slack")}</small>
            </dd>
          </dl>
        {/if}
      {/if}

      <h2>{t("ui.gear_measurement_over_teeth")}</h2>
      <dl>
        {#if isUnavailable(s.span)}
          <dt>{t("ui.gear_span")}</dt><dd class="na">{note(s.span.unavailable)}</dd>
        {:else}
          <dt>{t("ui.gear_teeth_spanned")}</dt><dd>{s.span.teeth_spanned}</dd>
          <dt>{t("ui.gear_nominal")}</dt><dd>{mm(s.span.nominal)}{around(s.span.around)}</dd>
          <dt>{t("ui.gear_contact_radius")}</dt><dd>{mm(s.span.contact_radius)}</dd>
        {/if}
      </dl>

      <h2>{t("ui.gear_measurement_over_pins")}</h2>
      <dl>
        {#each pinRows as row (row.label)}
          {#if isUnavailable(row.value)}
            <dt>{row.label}</dt><dd class="na">{note(row.value.unavailable)}</dd>
          {:else}
            <dt>{row.label}</dt><dd>{mm(row.value.nominal)}{around(row.value.around)}</dd>
          {/if}
        {/each}
      </dl>

      <h2>{t("ui.gear_composite_error_jgma_116_02")}</h2>
      <dl>
        {#if isUnavailable(s.tolerance)}
          <dt>{t("ui.gear_tolerance")}</dt><dd class="na">{note(s.tolerance.unavailable)}</dd>
        {:else}
          <dt>{t("ui.gear_class")}</dt>
          <dd>
            {s.tolerance.class.scale === "fine" ? "Fine" : "Standard"}
            {s.tolerance.class.grade}
          </dd>
          <dt>{t("ui.gear_tooth_tooth_max")}</dt><dd>{um(s.tolerance.tooth_to_tooth)}</dd>
          <dt>{t("ui.gear_total_max")}</dt><dd>{um(s.tolerance.total)}</dd>
        {/if}
      </dl>
    {/if}
  </section>
</div>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .title {
    font: inherit;
    font-size: 1.15rem;
    font-weight: 600;
    flex: 1;
    min-width: 0;
    padding: 0.25rem 0.4rem;
    border: 1px solid transparent;
    border-radius: 3px;
    background: none;
    color: var(--fg);
  }
  .title:hover,
  .title:focus {
    border-color: var(--rule);
    background: var(--bg);
  }
  .actions {
    display: flex;
    gap: 0.4rem;
  }
  button {
    font: inherit;
    font-size: 0.8rem;
    padding: 0.25rem 0.7rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: var(--panel);
    color: var(--fg);
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--hover);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  button.danger {
    color: var(--warn);
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    margin-top: 0.5rem;
  }

  .confirm {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    margin-bottom: 1rem;
    border: 1px solid var(--warn);
    border-radius: 4px;
    font-size: 0.85rem;
  }

  .columns {
    display: grid;
    grid-template-columns: minmax(20rem, 26rem) minmax(0, 1fr);
    gap: 1.5rem;
    align-items: start;
  }
  @media (max-width: 62rem) {
    .columns {
      grid-template-columns: 1fr;
    }
  }

  h2 {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin: 1.25rem 0 0.5rem;
  }
  h2:first-child {
    margin-top: 0;
  }

  .grid {
    display: flex;
    flex-direction: column;
    gap: var(--field-gap);
  }
  /* The gap inside a group is not the gap *between* groups until it is said so.
     Type sits in its own group — it decides which fields follow it — and the
     ring's cutter in a third, so both landed flush against the group below
     while every other pair of fields was a field-gap apart. */
  .grid + .grid {
    margin-top: var(--field-gap);
  }
  label {
    display: grid;
    grid-template-columns: 1fr 7rem 3.5rem;
    align-items: center;
    /* Column gap spaces the label, box and unit; row gap is what holds a note
       to the box it belongs to. They are not the same measurement. */
    column-gap: 0.5rem;
    row-gap: var(--note-gap);
    font-size: 0.85rem;
  }
  /* A select holding words, not a number, needs the room the number column does
     not: "Internal (ring)" was arriving as "Internal (rin". It takes the unit
     column's width as well, since a kind has no unit to print. */
  label.wide {
    grid-template-columns: 1fr 10.5rem;
  }
  label.check {
    grid-template-columns: auto 1fr;
  }
  label small {
    grid-column: 1 / -1;
    font-size: 0.72rem;
    color: var(--muted);
  }
  label small.err {
    color: var(--warn);
  }
  /* Every note a field can show, stacked in one cell: the slot takes the
     height of the tallest, so nothing moves when the visible one changes. */
  /* A note belongs to the box above it, so it ends where that box ends: the
     label and input columns only, right-aligned within them. */
  label .note {
    grid-column: 1 / 3;
    display: grid;
    text-align: right;
  }
  label .note small {
    grid-area: 1 / 1;
  }
  label .note small.hidden {
    visibility: hidden;
  }
  input[type="number"],
  select {
    font: inherit;
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
    text-align: right;
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
  }
  select {
    text-align: left;
  }
  label.invalid input {
    border-color: var(--warn);
  }
  em {
    font-style: normal;
    color: var(--muted);
    font-size: 0.75rem;
  }

  /* The label column is `max-content`, so a name never wraps. It was `1fr`
     against an `auto` value column, which is fine while the values are short
     numbers and falls apart once one of them carries an explanatory `small`:
     the value column then wants the whole row and squeezes the labels into two
     and three lines. Sizing the labels to their own longest and giving the rest
     to the values is the way round that reads. */
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.15rem 1rem;
    margin: 0;
    font-size: 0.85rem;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  dd.na {
    font-weight: 400;
    color: var(--muted);
    font-style: italic;
    text-align: right;
  }

  .notes {
    margin: 0.75rem 0 0;
    padding-left: 1.1rem;
    font-size: 0.8rem;
    color: var(--warn);
  }
  .error {
    color: var(--warn);
  }
  .warn {
    color: var(--warn);
  }
</style>
