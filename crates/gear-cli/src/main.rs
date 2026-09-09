//! Development harness for `gear-core`.
//!
//! Exists so the mathematics can be driven and inspected without a browser.
//!
//! ```text
//! gear-cli show   [z] [x]     print the derived geometry of one gear
//! gear-cli sweep              scan a parameter grid for clamps and undercut
//! gear-cli materials          the material library, with each value's basis
//! gear-cli strength [z1] [z2] [torque] [material] [helix]
//!                             a worked mesh: bending, contact, efficiency
//! gear-cli train              a two-stage geartrain, end to end
//! gear-cli trainfile [path]   export a geartrain to TOML, read it back, and
//!                             show that both solve to the same answers
//! gear-cli crossed [z1] [z2] [shaft angle]
//!                             a crossed gear pair, swept over the helix split
//! gear-cli planetstage [z_sun] [z_planet] [z_ring] [N] [helix]
//!                             a planetary stage, end to end
//! gear-cli hulaband [z] [clearance in modules]
//!                             the same reduction at every tooth difference, to
//!                             see what the difference of one costs
//! gear-cli meshsweep [z_ring] [z_pinion] [ring addendum] [pinion addendum]
//!                             roll an ordinary internal pair through a tooth —
//!                             the control the hula sweep is read against
//! gear-cli hulasweep [N] [clearance] [mesh]
//!                             the same roll, on a hula pair, where the tip
//!                             circles cross and the pitch point is outside both
//! gear-cli planetary [z_sun] [z_planet] [N] [x_sun] [x_ring]
//!                             the ring counts that can be made to work, and
//!                             the planet shift each of them needs
//! gear-cli hula [N] [clearance] [m_outer] [m_inner] [cutter teeth]
//!                             a hula stage: the offset both meshes run at, the
//!                             shifts it takes, and what the teeth then do
//! ```

mod diagram;
mod matrix;

use gear_core::{GearParams, Tooth};

/// The English catalogue, for turning a [`Note`](gear_core::note::Note) into a
/// sentence.
///
/// The harness has no locale to choose from and does not want one — it exists to
/// show what the core computed. Built per call rather than cached: this is a
/// development tool printing a handful of lines, and a `OnceLock` here would be
/// machinery in place of a parse that costs nothing.
fn words() -> gear_io::strings::Catalogue {
    gear_io::strings::Catalogue::english()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("sweep") => sweep(),
        Some("dump") => dump(),
        Some("bending") => bending_report(),
        Some("matrix") => matrix_report(),
        Some("loadcase") => loadcase_report(),
        Some("materials") => materials(),
        Some("train") => train_report(args.get(1).map(String::as_str) == Some("mixed")),
        Some("trainfile") => train_file_report(args.get(1).map(String::as_str)),
        Some("crossed") => crossed_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(23),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(90.0),
        ),
        Some("wormstage") => worm_stage_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(7.0),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2.0),
        ),
        Some("worm") => worm_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(7.0),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(90.0),
        ),
        Some("strength") => strength_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(43),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2.0),
            args.get(4).map_or("4340 Hardened Steel", String::as_str),
            args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            args.get(6).and_then(|s| s.parse().ok()),
        ),
        Some("dxf") => dxf(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1e-3),
        ),
        Some("planetstage") => planetary_stage_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(24),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(18),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(60),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(3),
            args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        ),
        Some("planetary") => planetary_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        ),
        Some("verify") => verify(
            args.get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(usize::MAX),
        ),
        Some("hula") => hula_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(18),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.5),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0),
            args.get(5).and_then(|s| s.parse().ok()),
        ),
        Some("hulaband") => hula_band(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(18),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.30),
        ),
        Some("meshsweep") => mesh_sweep(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(40),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0),
        ),
        Some("hulasweep") => hula_sweep(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(18),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.2),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
        ),
        Some("show") | None => {
            let teeth = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17);
            let x = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            show(GearParams {
                teeth,
                profile_shift: x,
                ..Default::default()
            });
        }
        Some(other) => eprintln!("unknown command {other:?}; try `show` or `sweep`"),
    }
}

/// A hula stage, from the arrangement down to what the teeth do.
///
/// Drives `train::solve_hula_stage` rather than assembling the parts itself:
/// the stage is where an arrangement becomes gears, and a harness that built its own
/// would be a second answer to the same question — which is how the two start
/// disagreeing.
fn hula_report(n: u32, clearance: f64, m_outer: f64, m_inner: f64, cutter_teeth: Option<u32>) {
    use gear_core::train::{solve_hula_stage, HulaStage, StageTorques};

    let lib = gear_io::default_library();
    let teeth = [n + 1, n, n - 1, n];
    let mut stage = HulaStage {
        module: [m_outer, m_inner],
        clearance,
        ..HulaStage::default()
    };
    for (gear, count) in stage.gears.iter_mut().zip(teeth) {
        gear.teeth = count;
    }
    // **A shaper has to be smaller than the ring it cuts**, and this command
    // builds rings far smaller than the ones the stage ships with: `hula 18`
    // asks for a 19-tooth ring, which the stocked default would not fit inside.
    // So the default stands where it fits and gives way to five teeth under the
    // ring where it does not — the sizing the comment here has always claimed
    // and never did, since the count it worked out was discarded unused.
    for (mesh, cutter) in stage.cutter.iter_mut().enumerate() {
        let ring = teeth[mesh * 2].max(teeth[mesh * 2 + 1]);
        let stocked = cutter.teeth;
        cutter.teeth = cutter_teeth.unwrap_or_else(|| stocked.min(ring.saturating_sub(5)).max(4));
    }

    let result = match solve_hula_stage(&stage, 1000.0, StageTorques::just(2.0), &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("that stage has no geometry: {e}");
            return;
        }
    };

    println!(
        "hula  z {}/{}/{}/{}  module {m_outer}/{m_inner}  alpha {} deg  clearance {clearance} mm",
        teeth[0], teeth[1], teeth[2], teeth[3], stage.pressure_angle
    );
    println!(
        "  ratio {} / {} = {:+.4}   crank offset {:.6} mm (running {:.6}){}",
        result.ratio_products[0],
        result.ratio_products[1],
        result.ratio,
        result.offset_nominal,
        result.offset,
        match result.binding_mesh {
            Some(m) => format!("   held open by mesh {}", m + 1),
            None => String::new(),
        }
    );
    println!(
        "  speeds  crank {:.1}  wobble {:+.3}  output {:+.4} rpm",
        result.crank_speed, result.gears[1].gear.speed, result.gears[3].gear.speed
    );
    println!(
        "  efficiency {:.3} % forward, {:.3} % back-driven{}   (the two meshes alone, crank held: {:.4})",
        result.efficiency.forward * 100.0,
        result.efficiency.backward * 100.0,
        if result.efficiency.backward == 0.0 {
            "  (self-locking)"
        } else {
            ""
        },
        result.fixed_carrier_efficiency.forward
    );
    println!(
        "  backlash at the output {:.6} deg (min {:.6}, max {:.6})   at the crank {:.4} deg",
        result.backlash.forward.nominal,
        result.backlash.forward.minimum,
        result.backlash.forward.maximum,
        result.backlash.backward.nominal
    );

    for (index, mesh) in result.meshes.iter().enumerate() {
        let members: Vec<&gear_core::train::HulaGear> =
            result.gears[index * 2..index * 2 + 2].iter().collect();
        println!(
            "\n  mesh {}  {}   alpha_w {:.3} deg   shaper z{}",
            index + 1,
            members
                .iter()
                .map(|g| format!(
                    "{} z{} x{:+.4}",
                    if g.ring { "ring" } else { "pinion" },
                    g.teeth,
                    g.gear.profile_shift
                ))
                .collect::<Vec<_>>()
                .join("  "),
            mesh.report.operating_pressure_angle,
            stage.cutter[index].teeth
        );
        println!(
            "    far-side gap {:.4} mm (as cut {:.4})   contact ratio {:.4}",
            mesh.clearance, mesh.clearance_as_cut, mesh.report.contact_ratios.transverse
        );
        println!(
            "    backlash {:.5} / {:.5} deg   interference: trochoid {}  involute {}  tip {} ({:+.4} deg)",
            mesh.report.backlash[0].nominal,
            mesh.report.backlash[1].nominal,
            mesh.trochoid_interference,
            mesh.involute_interference,
            mesh.tip_interference,
            mesh.tip_margin
        );
        // What the teeth are worth, which a stage of this kind needs as much as
        // the geometry: the reduction multiplies the mesh loss, and it multiplies
        // the torque on the way as well — the output pair carries the whole of it.
        println!(
            "    sigma_H {:.1} MPa at the pitch point   rho {:.4} mm",
            mesh.report.contact_stress_at_pitch_point.peak, mesh.report.relative_radius
        );
        for gear in &members {
            println!(
                "    z{:<4} T {:>10.4} Nm  b {:>7.3} mm  sigma_F {:>8}  sigma_H {:>7.1} MPa",
                gear.teeth,
                gear.gear.torque,
                gear.gear.face_width,
                gear.gear
                    .bending_stress
                    .peak
                    .map_or_else(|| "—".to_string(), |s| format!("{s:.1}")),
                gear.gear.contact_stress.peak,
            );
            for note in gear.gear.clamps.iter().chain(&gear.gear.notes) {
                println!("    ! z{}: {}", gear.teeth, words().render(note));
            }
        }
    }
    for note in &result.notes {
        println!("  ! {}", words().render(note));
    }
}

/// An outline as plain points, with its **arcs expanded rather than chorded**.
///
/// A tip and a root are exact arcs carried as vertex bulges, so reading only the
/// vertices replaces each with its chord — which on a 19 mm tip sits six microns
/// inside the true surface. That is small, and it is four times the interference
/// this harness was built to measure.
fn flatten(v: &[gear_core::Vertex]) -> Vec<(f64, f64)> {
    const PER_ARC: usize = 64;
    let mut out = Vec::with_capacity(v.len() * 2);
    for i in 0..v.len() {
        let (a, b) = (v[i], v[(i + 1) % v.len()]);
        out.push((a.x, a.y));
        if a.bulge.abs() < 1e-15 {
            continue;
        }
        // `bulge = tan(θ/4)`, so the included angle, the radius and the centre
        // all follow; the sign of the bulge carries the sense through `R`.
        let theta = 4.0 * a.bulge.atan();
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let chord = dx.hypot(dy);
        if chord < 1e-15 {
            continue;
        }
        let radius = chord / (2.0 * (theta / 2.0).sin());
        let h = radius * (theta / 2.0).cos();
        let centre = (
            (a.x + b.x) / 2.0 - dy / chord * h,
            (a.y + b.y) / 2.0 + dx / chord * h,
        );
        let start = (a.y - centre.1).atan2(a.x - centre.0);
        let r = radius.abs();
        for k in 1..PER_ARC {
            let t = start + theta * (k as f64) / (PER_ARC as f64);
            out.push((centre.0 + r * t.cos(), centre.1 + r * t.sin()));
        }
    }
    out
}

/// A closed outline, as segments bucketed by the angle they span about the
/// origin.
///
/// **Containment, not a radius table.** An earlier version of this compared a
/// point's radius against the bore's radius interpolated at the same angle,
/// which is not a distance between two curves and is not even defined where a
/// fillet doubles back in angle. It reported a seventieth of a millimetre of
/// interference on a pair that has none, which is how it was caught: the control
/// is not decoration.
///
/// A radial ray from a point stays in one angular bucket, so the crossing test
/// that decides inside from outside is exact and costs one bucket. The distance
/// wants a few buckets either side, since the nearest segment need not span the
/// point's own angle.
struct Boundary {
    segments: Vec<[f64; 4]>,
    buckets: Vec<Vec<usize>>,
}

impl Boundary {
    const BUCKETS: usize = 3600;

    fn new(points: &[(f64, f64)]) -> Self {
        let mut segments = Vec::with_capacity(points.len());
        for i in 0..points.len() {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            segments.push([a.0, a.1, b.0, b.1]);
        }
        let mut buckets = vec![Vec::new(); Self::BUCKETS];
        let index = Self::bucket_of;
        for (i, s) in segments.iter().enumerate() {
            // The angular span of a segment, the short way round: a segment of a
            // gear outline never spans half a turn.
            let (t0, t1) = (s[1].atan2(s[0]), s[3].atan2(s[2]));
            let mut d = (t1 - t0).rem_euclid(std::f64::consts::TAU);
            if d > std::f64::consts::PI {
                d -= std::f64::consts::TAU;
            }
            // Clamped into range first, so the conversion below is exact and
            // the lint is waived for a value that cannot be out of it.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let steps = ((d.abs() / std::f64::consts::TAU) * Self::BUCKETS as f64)
                .ceil()
                .clamp(1.0, Self::BUCKETS as f64) as usize;
            for k in 0..=steps.max(1) {
                let t = t0 + d * (k as f64) / (steps.max(1) as f64);
                buckets[index(t)].push(i);
            }
        }
        // **Once per bucket.** A short segment samples into the same bucket at
        // both ends, and a segment counted twice by the crossing test turns an
        // odd number of crossings into an even one — which is to say, turns
        // inside into outside for every point on that ray.
        for b in &mut buckets {
            b.sort_unstable();
            b.dedup();
        }
        Self { segments, buckets }
    }

    /// The bucket an angle falls in.
    ///
    /// `rem_euclid` puts it in `[0, τ)` and the scale puts it in
    /// `[0, BUCKETS)`, so the conversion is in range by construction; the
    /// remainder catches the one value that rounds up to the end.
    fn bucket_of(theta: f64) -> usize {
        let scaled = (theta.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU)
            * Self::BUCKETS as f64;
        // Clamped into range first, as above.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bucket = scaled.trunc().clamp(0.0, Self::BUCKETS as f64 - 1.0) as usize;
        bucket
    }

    /// Positive inside the outline, negative outside, in millimetres.
    fn clearance(&self, p: (f64, f64)) -> f64 {
        let theta = p.1.atan2(p.0);
        let r = p.0.hypot(p.1);
        // Inside or out: a ray straight out from the centre through `p` crosses
        // the outline an odd number of times beyond `p` exactly when `p` is
        // within it.
        let (c, s) = (theta.cos(), theta.sin());
        let mut crossings = 0;
        for &i in &self.buckets[Self::bucket_of(theta)] {
            let g = self.segments[i];
            // Cross the ray x = r' (c, s), r' > r, with the segment.
            let (a, b) = ((g[0], g[1]), (g[2], g[3]));
            let (na, nb) = (a.0 * -s + a.1 * c, b.0 * -s + b.1 * c);
            if (na > 0.0) == (nb > 0.0) {
                continue;
            }
            let t = na / (na - nb);
            let hit = (a.0 + t * (b.0 - a.0)) * c + (a.1 + t * (b.1 - a.1)) * s;
            if hit > r {
                crossings += 1;
            }
        }
        let inside = crossings % 2 == 1;

        let mut least = f64::INFINITY;
        let here = Self::bucket_of(theta);
        for k in 0..=60usize {
            let b = (here + Self::BUCKETS + k - 30) % Self::BUCKETS;
            for &i in &self.buckets[b] {
                let g = self.segments[i];
                let (dx, dy) = (g[2] - g[0], g[3] - g[1]);
                let len2 = dx * dx + dy * dy;
                let t = if len2 > 0.0 {
                    (((p.0 - g[0]) * dx + (p.1 - g[1]) * dy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let d = (p.0 - (g[0] + t * dx)).hypot(p.1 - (g[1] + t * dy));
                least = least.min(d);
            }
        }
        if inside {
            least
        } else {
            -least
        }
    }
}

/// Roll an internal pair through one tooth and report where the flanks touch.
///
/// **A contact ratio counts tooth pairs on the line of action; it does not say
/// whether the rest of the teeth are clear of each other.** The two questions
/// are separate, and at one tooth of difference the second is the one in doubt.
///
/// The assembly phase is swept rather than assumed, because an assembly that has
/// a phase is one that can be built, and the outlines' own conventions are not
/// this harness's business.
fn roll_pair(ring: &gear_core::ring::Ring, pinion: &gear_core::Gear, a: f64, title: &str) {
    const TOLERANCE: f64 = 1e-5;
    let pin_pts = flatten(&pinion.outline(TOLERANCE));
    let bore = Boundary::new(&flatten(&ring.outline(TOLERANCE)));

    let (z1, z2) = (f64::from(pinion.mean().params.teeth), f64::from(ring.teeth));
    // Only the points that could reach the bore are worth testing: everything
    // well inside the ring's tip circle is clear whatever the phase.
    let near: Vec<(f64, f64)> = pin_pts
        .iter()
        .copied()
        .filter(|(x, y)| (x + a).hypot(*y) > ring.ra - 1.0 || x.hypot(*y) > pinion.mean().ra - 1.0)
        .collect();

    let worst = |psi: f64, phi: f64| -> (f64, f64) {
        let (c1, s1) = (phi.cos(), phi.sin());
        let ring_angle = phi * z1 / z2 + psi;
        let (cr, sr) = ((-ring_angle).cos(), (-ring_angle).sin());
        let mut least = f64::INFINITY;
        let mut at = 0.0;
        for (x, y) in &near {
            let (fx, fy) = (x * c1 - y * s1 + a, x * s1 + y * c1);
            let p = (fx * cr - fy * sr, fx * sr + fy * cr);
            let gap = bore.clearance(p);
            if gap < least {
                least = gap;
                at = fy.atan2(fx);
            }
        }
        (least, at)
    };

    let pitch = std::f64::consts::TAU / z2;
    let mut best = (f64::NEG_INFINITY, 0.0);
    for i in 0..600 {
        let psi = pitch * f64::from(i) / 600.0;
        let g = worst(psi, 0.0).0;
        if g > best.0 {
            best = (g, psi);
        }
    }
    let psi = best.1;

    println!("{title}   centre distance {a:.4} mm");
    println!(
        "  best assembly phase {:.4} deg of a {:.3} deg ring pitch, clear by {:+.5} mm there",
        psi.to_degrees(),
        pitch.to_degrees(),
        best.0
    );

    let mut overall = (f64::INFINITY, 0.0);
    for step in 0..180 {
        let phi = (std::f64::consts::TAU / z1) * f64::from(step) / 180.0;
        let (gap, at) = worst(psi, phi);
        if gap < overall.0 {
            overall = (gap, at);
        }
    }
    println!(
        "  rolled through one tooth: least clearance {:+.5} mm at {:.1} deg from the line of centres{}",
        overall.0,
        overall.1.to_degrees(),
        if overall.0 < -10.0 * TOLERANCE {
            "   THE TEETH FOUL"
        } else {
            "   (touching, as a mesh does)"
        }
    );
}

/// The same reduction at every tooth difference, and what each one can reach.
///
/// **A reduction does not say how to get it.** `R = z²/d²`, so `z = d·z₀` gives
/// `z₀²` at any difference `d` — the same ratio, the same pitch diameters and
/// the same crank offset, reached with `d` times the teeth at a `d`th of the
/// module. Whether that matters is the question this answers, and the answer is
/// not small.
///
/// For each difference it searches what a designer would: the addendum, the
/// shaper, and the division of each mesh's shift. What it reports is the best
/// design that is *admissible* — contact continuous, no interference of any of
/// the three kinds, nothing clamped — which is the only kind worth comparing.
///
/// # Which shifts these are
///
/// Not the basic ones, and not
/// [`contact::efficient_split`](gear_core::contact::efficient_split)'s either.
///
/// The **sum** of a mesh's two shifts is never free here: the crank offset is
/// what it is, and that is the arrangement's defining constraint. Only the
/// division is left, and it is searched rather than solved — because the
/// unconstrained optimum is not a design. On these stages the mesh loses least
/// at a division of `+2.85`, `+2.05`, `−2.55` for one, two and four teeth of
/// difference, and **none of the three is admissible**: contact has gone
/// discontinuous or the tips have fouled long before. Every row below sits
/// instead on the constraints, at `ε ≈ 1.00` with the tip margin at zero, which
/// is where a bound answers rather than an optimum.
///
/// So the closed form is not the tool for this arrangement. It finds where the
/// loss is stationary, and here the loss is still falling when the geometry runs
/// out — the useful statement is which bound stops it, and that is what these
/// rows are.
fn hula_band(z0: u32, clearance_in_modules: f64) {
    use gear_core::train::{solve_hula_stage, HulaStage, StageTorques};

    let lib = gear_io::default_library();

    println!(
        "hula, reduction {} : 1 — the same ratio at every tooth difference\n",
        z0 * z0
    );
    println!(
        "{:>3} {:>6} {:>7} {:>5} {:>6} {:>7} {:>10} {:>8} {:>8} {:>7} {:>9}",
        "d", "z", "module", "h_a", "shaper", "x", "meshes", "stage", "alpha_w", "eps", "backlash"
    );
    for d in 1..=9u32 {
        let n = z0 * d;
        let module = 1.0 / f64::from(d);
        let teeth = [n + d, n, n - d, n];
        let mut best: Option<(f64, u32, f64, gear_core::train::HulaResult)> = None;
        for addendum in [0.8, 0.7, 0.6, 0.5, 0.4] {
            for cutter in [10u32, 14, 20, 28] {
                if cutter + 2 >= n {
                    continue;
                }
                for i in -30..=40 {
                    let x = f64::from(i) * 0.05;
                    let mut stage = HulaStage {
                        module: [module; 2],
                        clearance: clearance_in_modules * module,
                        running_clearance: 0.02 * module,
                        tolerance_plus: 0.02 * module,
                        tolerance_minus: 0.02 * module,
                        ..HulaStage::default()
                    };
                    for (gear, count) in stage.gears.iter_mut().zip(teeth) {
                        gear.teeth = count;
                        gear.addendum = addendum;
                        gear.profile_shift = gear_core::params::Auto::fixed(x);
                    }
                    for c in &mut stage.cutter {
                        c.teeth = cutter;
                    }
                    let Ok(r) = solve_hula_stage(&stage, 1000.0, StageTorques::just(2.0), &lib)
                    else {
                        continue;
                    };
                    let admissible = r.meshes.iter().all(|m| {
                        m.report.contact_ratios.transverse >= 1.0
                            && !m.tip_interference
                            && !m.trochoid_interference
                            && !m.involute_interference
                    }) && r.gears.iter().all(|g| g.gear.as_asked());
                    if !admissible {
                        continue;
                    }
                    if best
                        .as_ref()
                        .is_none_or(|(_, _, _, b)| r.efficiency.forward > b.efficiency.forward)
                    {
                        best = Some((x, cutter, addendum, r));
                    }
                }
            }
        }
        match best {
            None => println!("{d:>3} {n:>6} {module:>7.3}   nothing admissible"),
            Some((x, cutter, h, r)) => println!(
                "{d:>3} {n:>6} {module:>7.3} {h:>5.1} {cutter:>6} {x:>+7.2} {:>9.4}% {:>7.2}% {:>8.2} {:>7.4} {:>9.5}",
                r.fixed_carrier_efficiency.forward * 100.0,
                r.efficiency.forward * 100.0,
                r.meshes[0].report.operating_pressure_angle,
                r.meshes[0].report.contact_ratios.transverse,
                r.backlash.forward.nominal
            ),
        }
    }
}

/// The control: an ordinary internal pair, whose answer is known.
///
/// Standard proportions, no shift, at its own zero-backlash centre distance. Its
/// flanks touch and nothing penetrates, so whatever this reports is the
/// harness's own error and the figure everything else is read against.
fn mesh_sweep(z_ring: u32, z_pinion: u32, ring_addendum: f64, pinion_addendum: f64) {
    use gear_core::ring::{mesh_with, Cutter, Ring};
    let params = |teeth: u32, addendum: f64| GearParams {
        teeth,
        addendum,
        ..GearParams::default()
    };
    let cutter = Cutter {
        teeth: z_ring.saturating_sub(5).max(6),
        ..Cutter::default()
    };
    let ring = Ring::cut_by(&params(z_ring, ring_addendum), &cutter);
    let pinion = gear_core::Gear::new(params(z_pinion, pinion_addendum));
    let Some(m) = mesh_with(&ring, pinion.mean()) else {
        eprintln!("z{z_ring} and z{z_pinion} do not mesh");
        return;
    };
    roll_pair(
        &ring,
        &pinion,
        m.centre_distance,
        &format!(
            "control  ring z{z_ring}  pinion z{z_pinion}   contact ratio {:.4}   alpha_w {:.2} deg   troch {} inv {} tip {} ({:+.4} deg)",
            m.contact_ratio,
            m.alpha_w.to_degrees(),
            m.trochoid_interference,
            m.involute_interference,
            m.tip_interference,
            m.tip_margin.to_degrees()
        ),
    );
}

/// The same roll, on the hula pair the **stage** produces.
///
/// Through `solve_hula_stage` rather than the arrangement alone, because the
/// offset answers to the tips as well as to the far-side gap and a harness
/// rolling the stage before that bound was applied would be measuring one
/// nobody builds.
fn hula_sweep(n: u32, clearance: f64, mesh_index: usize) {
    use gear_core::ring::Ring;
    use gear_core::train::{solve_hula_stage, HulaStage, StageTorques};

    let lib = gear_io::default_library();
    let teeth = [n + 1, n, n - 1, n];
    let mut stage = HulaStage {
        clearance,
        ..HulaStage::default()
    };
    for (gear, count) in stage.gears.iter_mut().zip(teeth) {
        gear.teeth = count;
    }
    let result = match solve_hula_stage(&stage, 1000.0, StageTorques::just(2.0), &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("that stage has no geometry: {e}");
            return;
        }
    };
    let (a, b) = (mesh_index * 2, mesh_index * 2 + 1);
    let (ring_i, pinion_i) = if result.gears[a].ring { (a, b) } else { (b, a) };
    let params = |i: usize| GearParams {
        module: stage.module[mesh_index],
        teeth: result.gears[i].teeth,
        profile_shift: result.gears[i].gear.profile_shift,
        addendum: stage.gears[i].addendum,
        dedendum: stage.gears[i].dedendum,
        thickness_mod: stage.thickness_mod[mesh_index],
        ..GearParams::default()
    };
    let ring = Ring::cut_by(&params(ring_i), &stage.cutter[mesh_index]);
    let pinion = gear_core::Gear::new(params(pinion_i));
    let m = &result.meshes[mesh_index];
    roll_pair(
        &ring,
        &pinion,
        result.offset_nominal,
        &format!(
            "hula mesh {}  ring z{} x{:+.4}  pinion z{} x{:+.4}   gap asked {clearance} got {:.4} mm   alpha_w {:.2} deg   tip {} ({:+.4} deg)",
            mesh_index + 1,
            result.gears[ring_i].teeth,
            result.gears[ring_i].gear.profile_shift,
            result.gears[pinion_i].teeth,
            result.gears[pinion_i].gear.profile_shift,
            m.clearance,
            m.report.operating_pressure_angle,
            m.tip_interference,
            m.tip_margin
        ),
    );
}

/// A two-stage geartrain, end to end — milestone 6's gate.
/// Write a geartrain out as TOML, read it back, and solve both.
///
/// The point is the *comparison*, not the file: an export that loses a field
/// produces a train that still solves, just to different numbers, and only
/// putting the two answers side by side shows it. With a path, the file is left
/// there to be looked at and hand-edited; without one it is written to a
/// temporary file and removed.
fn train_file_report(path: Option<&str>) {
    use gear_core::params::Auto;
    use gear_core::train::{
        solve_train, Actuation, PlanetaryStage, SpurStage, Stage, StageGear, Train, WormStage,
    };
    use gear_io::TrainDocument;

    let lib = gear_io::default_library();
    let doc = TrainDocument {
        name: "Elevation drive".to_string(),
        train: Train {
            input_speed: 3000.0,
            input_torque: 2.0,
            back_driving_torque: 0.0,
            operating_torque: 2.0,
            reversed_bending: false,
            actuation: Actuation::Continuous {
                operating_speed: 2400.0,
                runtime_hours: 1000.0,
            },
            stages: vec![
                Stage::Spur(SpurStage {
                    additional_helix: 15.0,
                    gears: [
                        StageGear {
                            teeth: 17,
                            face_width: Auto::automatic(0.0),
                            ..StageGear::default()
                        },
                        StageGear {
                            teeth: 43,
                            ..StageGear::default()
                        },
                    ],
                    ..SpurStage::default()
                }),
                Stage::Worm(WormStage::default()),
                Stage::Planetary(Box::<PlanetaryStage>::default()),
            ],
        },
    };

    let text = match gear_io::train::to_toml(&doc) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("could not write the geartrain: {e}");
            return;
        }
    };
    let back = match gear_io::train::from_toml(&text) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("could not read it back: {e}");
            return;
        }
    };

    let written = path.unwrap_or("/tmp/gear-cli-train.toml");
    if std::fs::write(written, &text).is_ok() {
        println!(
            "wrote {written}  ({} lines, {} bytes)",
            text.lines().count(),
            text.len()
        );
    }
    if path.is_none() {
        let _ = std::fs::remove_file(written);
    }
    println!("name   {:?} -> {:?}", doc.name, back.name);
    println!(
        "stages {} -> {}",
        doc.train.stages.len(),
        back.train.stages.len()
    );

    let (a, b) = (
        solve_train(&doc.train, &lib),
        solve_train(&back.train, &lib),
    );
    match (a, b) {
        (Ok(a), Ok(b)) => {
            println!("\n  quantity                 exported            re-imported   same");
            let rows: [(&str, f64, f64); 5] = [
                ("total ratio", a.total_ratio, b.total_ratio),
                ("output speed rpm", a.output_speed, b.output_speed),
                ("output torque Nm", a.output_torque, b.output_torque),
                (
                    "efficiency forward",
                    a.total_efficiency.forward,
                    b.total_efficiency.forward,
                ),
                (
                    "backlash out deg",
                    a.backlash.forward.nominal,
                    b.backlash.forward.nominal,
                ),
            ];
            let mut all = true;
            for (name, x, y) in rows {
                // Bit-identical or not at all: these come from the same
                // arithmetic on numbers that either survived the file or did
                // not. A tolerance here would hide exactly what is being asked.
                let same = x.to_bits() == y.to_bits();
                all &= same;
                println!(
                    "  {name:<22} {x:>16.9} {y:>16.9}   {}",
                    if same { "yes" } else { "NO" }
                );
            }
            println!(
                "\n  {}",
                if all {
                    "the file is the train: every figure identical to the last bit"
                } else {
                    "SOMETHING WAS LOST IN THE FILE - see the rows marked NO"
                }
            );
        }
        (a, b) => println!("a train did not solve: {:?} / {:?}", a.err(), b.err()),
    }
}

fn train_report(mixed: bool) {
    use gear_core::params::Auto;
    use gear_core::train::{
        solve_train, Actuation, SpurStage, Stage, StageGear, StageResult, Train, WormStage,
    };

    let lib = gear_io::default_library();
    let auto_width = |teeth: u32| StageGear {
        teeth,
        face_width: Auto::automatic(0.0),
        ..StageGear::default()
    };
    let train = Train {
        input_speed: 3000.0,
        input_torque: 2.0,
        back_driving_torque: 0.0,
        operating_torque: 2.0,
        reversed_bending: false,
        actuation: Actuation::Continuous {
            operating_speed: 2400.0,
            runtime_hours: 1000.0,
        },
        stages: if mixed {
            vec![
                Stage::Spur(SpurStage {
                    gears: [auto_width(17), auto_width(43)],
                    ..SpurStage::default()
                }),
                Stage::Worm(WormStage {
                    wheel: gear_core::train::WormMember {
                        material: "Brass C360".into(),
                        ..gear_core::train::WormMember::default()
                    },
                    ..WormStage::default()
                }),
            ]
        } else {
            vec![
                Stage::Spur(SpurStage {
                    gears: [auto_width(17), auto_width(43)],
                    ..SpurStage::default()
                }),
                Stage::Spur(SpurStage {
                    additional_helix: 15.0,
                    gears: [auto_width(13), auto_width(31)],
                    ..SpurStage::default()
                }),
            ]
        },
    };

    let r = match solve_train(&train, &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("train did not solve: {e}");
            return;
        }
    };

    println!(
        "train  in {:.0} rpm / {:.3} Nm peak   ->   out {:.1} rpm / {:.3} Nm peak",
        train.input_speed, train.input_torque, r.output_speed, r.output_torque
    );
    println!(
        "       operating {:.3} Nm{}   back-driving {:.3} Nm at the output",
        train.cyclic_torque(),
        r.operating_torque_percent
            .map_or(String::new(), |p| format!(" ({p:.1} % of peak)")),
        train.back_driving_torque
    );
    println!(
        "       total ratio {:.4}:1   total efficiency {:.3} % forward / {:.3} % backward{}",
        r.total_ratio,
        100.0 * r.total_efficiency.forward,
        100.0 * r.total_efficiency.backward,
        if r.total_efficiency.self_locking() {
            "  (cannot be back-driven)"
        } else {
            ""
        }
    );
    for n in &r.notes {
        println!("       note: {}", words().render(n));
    }
    println!(
        "       backlash at the output shaft  {:.5} deg  (min {:.5}, max {:.5})",
        r.backlash.forward.nominal, r.backlash.forward.minimum, r.backlash.forward.maximum
    );
    println!(
        "       backlash at the input shaft   {:.5} deg  (min {:.5}, max {:.5})",
        r.backlash.backward.nominal, r.backlash.backward.minimum, r.backlash.backward.maximum
    );

    for (k, s) in r.stages.iter().enumerate() {
        match (&train.stages[k], s) {
            (Stage::Spur(st), StageResult::Spur(res)) => print_spur_stage(k, st, res),
            (Stage::Worm(st), StageResult::Worm(res)) => print_worm_stage(k, st, res),
            _ => println!("\nstage {}: kind and result disagree", k + 1),
        }
    }
}

fn print_spur_stage(k: usize, st: &gear_core::train::SpurStage, s: &gear_core::train::SpurResult) {
    println!(
        "\nstage {}  spur  z {}/{}  beta {} deg  ratio {:.4}  a_w {:.4} mm{}",
        k + 1,
        st.gears[0].teeth,
        st.gears[1].teeth,
        st.additional_helix,
        s.ratio,
        s.centre_distance,
        if s.coprime { "  coprime" } else { "" }
    );
    println!(
        "  contact ratio  transverse {:.4}   overlap {:.4}   total {:.4}{}",
        s.contact_ratios.transverse,
        s.contact_ratios.overlap,
        s.contact_ratios.total,
        if st.additional_helix != 0.0 && !s.contact_ratios.has_full_axial_overlap() {
            "   <- no full axial overlap"
        } else {
            ""
        }
    );
    println!(
        "  efficiency {:.3} % forward / {:.3} % backward",
        100.0 * s.efficiency.forward,
        100.0 * s.efficiency.backward
    );
    // One pressure, printed once. The pair shares a patch, a normal force and an
    // `E*`, so there is no second number to print per gear — what a gear has of
    // its own is the allowable, and therefore `b_min`.
    println!(
        "  contact at the pitch point  sigma_H {:.1} / {:.1} MPa peak/cyclic   rho {:.3} mm",
        s.contact_stress_at_pitch_point.peak,
        s.contact_stress_at_pitch_point.cyclic,
        s.relative_radius
    );
    println!(
        "  {:<6} {:>8} {:>8} {:>10} {:>10} {:>21} {:>21} {:>9} {:>21}",
        "gear",
        "x",
        "b mm",
        "T fwd Nm",
        "T bwd Nm",
        "sigma_F peak/cyclic",
        "sigma_H peak/cyclic",
        "rpm",
        "cycles bend/contact"
    );
    for (i, g) in s.gears.iter().enumerate() {
        println!(
            "  {:<6} {:>8.4} {:>8.3} {:>10.4} {:>10} {:>9.1} /{:>9.1} {:>9.1} /{:>9.1} {:>9.1} {:>9.3e} /{:>9.3e}",
            i + 1,
            g.profile_shift,
            g.face_width,
            g.torque,
            g.back_driving_torque.map_or("-".into(), |t| format!("{t:.4}")),
            g.bending_stress.peak.unwrap_or(f64::NAN),
            g.bending_stress.cyclic.unwrap_or(f64::NAN),
            g.contact_stress.peak,
            g.contact_stress.cyclic,
            g.speed,
            g.tooth_cycles.bending,
            g.tooth_cycles.contact
        );
    }
    for n in &s.notes {
        println!("  note: {}", words().render(n));
    }
}

fn print_worm_stage(k: usize, st: &gear_core::train::WormStage, s: &gear_core::train::WormResult) {
    println!(
        "\nstage {}  worm  z {}/{}  ratio {:.4}  a {:.4} mm  lead angle {:.4} deg",
        k + 1,
        st.starts,
        st.wheel_teeth,
        s.ratio,
        s.centre_distance,
        s.lead_angle
    );
    println!(
        "  efficiency  forward {:.3} %  backward {:.3} %{}",
        100.0 * s.efficiency.forward,
        100.0 * s.efficiency.backward,
        if s.efficiency.self_locking() {
            "  (self-locking)"
        } else {
            ""
        }
    );
    println!(
        "  contact  peak {:.1} MPa  cyclic {:.1} MPa   patch {:.4} x {:.4} mm   sliding {:.1} mm/s",
        s.contact.peak.max_pressure,
        s.contact.cyclic.max_pressure,
        s.contact.peak.patch_length,
        s.contact.peak.patch_width,
        s.sliding_velocity
    );
    println!(
        "  {:<6} {:>8} {:>10} {:>10} {:>9} {:>21}   material",
        "member", "b mm", "T fwd Nm", "T bwd Nm", "rpm", "cycles bend/contact"
    );
    for (name, m) in ["worm", "wheel"].iter().zip(&s.members) {
        println!(
            "  {name:<6} {:>8.3} {:>10.4} {:>10} {:>9.1} {:>9.3e} /{:>9.3e}   {}",
            m.face_width,
            m.torque,
            m.back_driving_torque
                .map_or("-".into(), |t| format!("{t:.4}")),
            m.speed,
            m.tooth_cycles.bending,
            m.tooth_cycles.contact,
            m.material.name
        );
    }
    println!("  bending not reported, flank type ZI - see docs/reference.md#crossed-axes");
    for n in &s.notes {
        println!("  note: {}", words().render(n));
    }
}

/// A worked mesh end to end: load, bending, contact, efficiency, face width.
///
/// This is the whole of milestone 5 in one view, and the first thing that
/// actually consumes the material library. Both gears are rated, because the
/// pinion is not automatically the worse one — it sees the higher contact
/// stress but the wheel may have the weaker root.
fn strength_report(
    z1: u32,
    z2: u32,
    torque: f64,
    material_name: &str,
    helix: f64,
    rim: Option<f64>,
) {
    use gear_core::contact::{efficiency, ContactPath, Drive};
    use gear_core::material::contact_modulus;
    use gear_core::mesh::{Mesh, MeshKind};
    use gear_core::metrology::base_helix_angle;
    use gear_core::strength::{
        bending_section, bending_stress, contact_stress, min_face_width_bending,
        min_face_width_contact, Load, RimSupport, StressConcentration, PARALLEL_AXES,
    };

    let lib = gear_io::default_library();
    let Some(mat) = lib.get(material_name) else {
        eprintln!("no material named {material_name:?}; try `gear-cli materials`");
        return;
    };

    // Meshing helical gears have equal and opposite hands.
    let g1 = Tooth::new(GearParams {
        teeth: z1,
        helix_angle: helix,
        ..Default::default()
    });
    let g2 = Tooth::new(GearParams {
        teeth: z2,
        helix_angle: -helix,
        ..Default::default()
    });
    let Ok(mesh) = Mesh::new(&g1, &g2, MeshKind::External) else {
        eprintln!("z={z1}/{z2} cannot mesh");
        return;
    };
    let Some(path) = ContactPath::new(&g1, g2.ra, &mesh) else {
        eprintln!("z={z1}/{z2} has no usable path of contact");
        return;
    };

    // A face width to evaluate at. Any value does: the minimum face widths
    // below are independent of it, which is asserted in the test suite.
    const B: f64 = 10.0;
    let load = Load::new(torque, B);

    println!(
        "mesh   z {z1}/{z2}  module {}  a_w {:.4} mm",
        g1.params.module, mesh.a_w
    );
    println!(
        "       operating pressure angle {:.3} deg  contact ratio {:.4}",
        mesh.alpha_w.to_degrees(),
        path.contact_ratio
    );
    if helix != 0.0 {
        println!(
            "helix  beta {helix} deg  base helix beta_b {:.3} deg  virtual teeth {:.2}/{:.2}",
            base_helix_angle(&g1).to_degrees(),
            g1.virtual_spur().z,
            g2.virtual_spur().z
        );
    }
    println!(
        "load   {torque} Nm on gear 1  ->  F_n {:.1} N along the line of action",
        load.transverse_line_of_action(&g1)
    );
    println!(
        "       F_t {:.1} N at the reference circle,  face width {B} mm",
        load.tangential(&g1)
    );
    println!(
        "material  {}  [{}]",
        mat.name,
        if mat.weakest_basis().is_measured() {
            "all values measured"
        } else {
            "contains estimates - see `gear-cli materials`"
        }
    );
    println!(
        "       E {:.0} MPa   nu {:.2}   ultimate {:.1} MPa   fatigue {:.1} MPa",
        mat.elastic_modulus.value,
        mat.poissons_ratio.value,
        mat.ultimate_allowable.value,
        mat.fatigue_allowable.value
    );
    println!();

    // --- bending, each gear at its own highest point of single-pair contact
    println!("bending");
    println!(
        "  {:<6} {:>8} {:>8} {:>9} {:>10} {:>10}",
        "gear", "Y_F", "Y_S", "sigma_F", "b_min fat", "b_min ult"
    );
    let reversed = Mesh::new(&g2, &g1, MeshKind::External).ok();
    for (label, g, p) in [
        (1u32, &g1, Some(path)),
        (
            2,
            &g2,
            reversed.and_then(|m| ContactPath::new(&g2, g1.ra, &m)),
        ),
    ] {
        let Some(p) = p else {
            println!("  {label:<6} no contact path");
            continue;
        };
        let Some(sec) = bending_section(g, p.contact_ratio) else {
            println!("  {label:<6} no root section (severed tooth?)");
            continue;
        };
        let load_g = load.across_mesh(&g1, g);
        let ys = sec.stress_correction(StressConcentration::Iso6336);
        // `Y_B` only where a rim was named on the command line: a rim nobody
        // described rates at 1 and is not the same claim as a thick one.
        let rim_support = rim.map(|s| RimSupport::external(s, g.ra - g.rf));
        let Some(sf) = bending_stress(&sec, g, &load_g, StressConcentration::Iso6336, rim_support)
        else {
            println!(
                "  {label:<6} {:>8.4} {:>8} {:>9} - stress correction undefined (tangency on the flank)",
                sec.form_factor, "-", "-"
            );
            continue;
        };
        println!(
            "  {label:<6} {:>8.4} {:>8.4} {:>7.1} MPa {:>8.3} mm {:>8.3} mm",
            sec.form_factor,
            ys.unwrap_or(1.0),
            sf,
            min_face_width_bending(sf, B, mat.fatigue_allowable.value),
            min_face_width_bending(sf, B, mat.ultimate_allowable.value),
        );
        // `Y_B`, printed only where a rim was named: on a gear with none it is
        // exactly 1, and a column of ones is noise.
        if let Some(r) = rim_support {
            println!("         Y_B {:.4} (s_R/h_t {:.3})", r.factor(), r.ratio());
            if !r.in_range() {
                println!(
                    "         note: ISO 6336-3 says a backup ratio at or below 0.5 shall be avoided"
                );
            }
        }
        if !sec.notch_parameter_in_range() {
            println!(
                "         note: notch parameter q_s = {:.2} is outside the ISO fit's range, so",
                sec.notch_parameter
            );
            println!("               Y_S was clamped and the stress is UNDER-predicted");
        }
    }

    // --- contact. One pressure at any instant, two ratings: each gear is judged
    // where its own dedendum carries the load alone.
    let e_star = contact_modulus(mat, mat);
    if let Some(cs) = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star) {
        println!("\ncontact   E* {e_star:.0} MPa (like on like)");
        println!("  at the pitch point        {:>7.1} MPa", cs.at_pitch_point);
        for i in 0..2 {
            println!(
                "  gear {} single-pair        {:>7.1} MPa   -> rated at {:>7.1} MPa",
                i + 1,
                cs.at_single_pair[i],
                cs.governing(i)
            );
        }
        println!("  relative radius           {:>7.3} mm", cs.relative_radius);
        println!(
            "  b_min against fatigue     {:>7.3} mm",
            min_face_width_contact(cs.worst, B, mat.fatigue_allowable.value)
        );
        println!(
            "  b_min against ultimate    {:>7.3} mm",
            min_face_width_contact(cs.worst, B, mat.ultimate_allowable.value)
        );
    }

    // --- efficiency
    println!("\nefficiency (equal in both directions for a parallel-axis mesh)");
    for mu in [0.02, 0.04, 0.06, 0.10] {
        println!(
            "  mu {mu:.2}   {:.3} %",
            100.0 * efficiency(&path, &mesh, &g1, mu, Drive::Forward)
        );
    }
}

/// The shipped material library, with each value's provenance.
///
/// The `basis` column is the point of this view: the library deliberately
/// contains estimates as well as measurements, and the difference should be
/// visible at a glance rather than buried in the TOML.
fn materials() {
    let lib = gear_io::default_library();
    println!(
        "{:<20} {:>8} {:>9} {:>6} {:>10} {:>9}",
        "material", "rho", "E", "nu", "ultimate", "fatigue"
    );
    println!(
        "{:<20} {:>8} {:>9} {:>6} {:>10} {:>9}",
        "", "kg/m3", "MPa", "", "MPa", "MPa"
    );
    println!("{}", "-".repeat(67));

    for m in &lib.materials {
        // A single letter per value, so a row reads as a confidence pattern:
        // `d` datasheet, `D` derived, `c` chart, `e` estimated.
        let tag = |v: &gear_core::material::Value| match v.basis {
            gear_core::material::Basis::Overridden => 'o',
            gear_core::material::Basis::Datasheet => 'd',
            gear_core::material::Basis::Derived => 'D',
            gear_core::material::Basis::Chart => 'c',
            gear_core::material::Basis::Estimated => 'e',
        };
        println!(
            "{:<20} {:>7.0}{} {:>8.0}{} {:>5.2}{} {:>9.1}{} {:>8.1}{}",
            m.name,
            m.density.value,
            tag(&m.density),
            m.elastic_modulus.value,
            tag(&m.elastic_modulus),
            m.poissons_ratio.value,
            tag(&m.poissons_ratio),
            m.ultimate_allowable.value,
            tag(&m.ultimate_allowable),
            m.fatigue_allowable.value,
            tag(&m.fatigue_allowable),
        );
    }

    println!("\nbasis: d datasheet   D derived   c read off a chart   e estimated   o overridden");
    println!("each entry is one material in one state; the condition names it\n");

    for m in &lib.materials {
        println!("{}  [{}]", m.name, m.grade);
        println!("  condition: {}", m.condition);
        println!("  source:    {}", m.source);
        for (label, v) in [
            ("density", &m.density),
            ("modulus", &m.elastic_modulus),
            ("poisson", &m.poissons_ratio),
            ("ultimate", &m.ultimate_allowable),
            ("fatigue", &m.fatigue_allowable),
        ] {
            if let Some(note) = &v.note {
                println!("  {label:<9} {note}");
            }
        }
        println!();
    }
}

fn show(p: GearParams) {
    let g = Tooth::new(p);
    println!(
        "module {}  z {}  alpha {}  x {:+}  beta {}  k {}",
        p.module, p.teeth, p.pressure_angle, p.profile_shift, p.helix_angle, p.thickness_mod
    );
    println!("  pitch radius   {:12.6}", g.r);
    println!("  base radius    {:12.6}", g.rb);
    println!("  tip radius     {:12.6}", g.ra);
    println!("  root radius    {:12.6}", g.rf);
    println!("  tooth thick.   {:12.6}  (transverse, at pitch)", g.st);
    println!("  fillet radius  {:12.6}", g.rho);
    println!(
        "  L              {:12.6}  ({})",
        g.l,
        if g.undercut {
            "UNDERCUT"
        } else {
            "no undercut"
        }
    );
    println!("  junction r     {:12.6}", g.r_j);
    println!("  severed        {:12}", g.severed);
    if g.clamps.any() {
        println!("  clamps:");
        for n in &g.clamps.notes {
            println!("    - {}", words().render(n));
        }
    }
    let pts = gear_core::gear::Gear::new(g.params).profile(400);
    println!("  profile points {:12}", pts.len());
}

fn sweep() {
    let mut total = 0u32;
    let mut undercut = 0u32;
    let mut severed = 0u32;
    let mut clamped = 0u32;
    for z in 3..=60u32 {
        for xi in -6..=10i32 {
            for alpha in [14.5_f64, 20.0, 25.0] {
                for beta in [0.0_f64, 20.0] {
                    let g = Tooth::new(GearParams {
                        teeth: z,
                        profile_shift: f64::from(xi) * 0.1,
                        pressure_angle: alpha,
                        helix_angle: beta,
                        ..Default::default()
                    });
                    total += 1;
                    undercut += u32::from(g.undercut);
                    severed += u32::from(g.severed);
                    clamped += u32::from(g.clamps.any());
                }
            }
        }
    }
    println!("{total} cases: {undercut} undercut, {severed} severed, {clamped} clamped");
}

/// Emit the same case grid as `tools/dump_ref.py` so the port can be compared
/// point by point against the Python reference it was derived from.
fn dump() {
    for z in [3u32, 5, 8, 9, 11, 13, 17, 23, 31, 47, 60] {
        for x in [-0.5_f64, -0.2, 0.0, 0.2, 0.5, 0.9] {
            for an in [14.5_f64, 20.0, 25.0] {
                for beta in [0.0_f64, 20.0, -30.0] {
                    for rr in [0.0_f64, 0.38] {
                        let g = Tooth::new(GearParams {
                            module: 1.0,
                            pressure_angle: an,
                            teeth: z,
                            profile_shift: x,
                            angular_shift: 0.0,
                            index_offset: 0.0,
                            helix_angle: beta,
                            addendum: 1.0,
                            dedendum: 1.25,
                            root_radius: rr,
                            thickness_mod: 1.0,
                        });
                        println!(
                            "S\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{}\t{}\t{}",
                            g.r, g.rb, g.ra, g.rf, g.st, g.rho, g.bc, g.ac, g.l,
                            g.u_j, g.s_j, g.r_j, g.theta0, g.theta_a, g.psi_b,
                            u8::from(g.undercut), u8::from(g.severed), g.clamps.notes.len()
                        );
                        let (r, th) = g.half_profile(200);
                        let j = |v: &Vec<f64>| {
                            v.iter()
                                .map(|x| format!("{x:.17e}"))
                                .collect::<Vec<_>>()
                                .join(",")
                        };
                        println!("R\t{}", j(&r));
                        println!("T\t{}", j(&th));
                    }
                }
            }
        }
    }
}

/// Two-sided cutter verification over a parameter grid.
fn verify(limit: usize) {
    use gear_core::verify::{check_cut, fillet_envelope_error, sdf_matches_polyline};
    let mut n = 0usize;
    let mut worst_pen = 0.0_f64;
    let mut worst_dev = 0.0_f64;
    let mut worst_env = 0.0_f64;
    let mut worst_sdf = 0.0_f64;
    let (mut pen_case, mut dev_case) = (String::new(), String::new());
    'outer: for z in [3u32, 5, 8, 9, 11, 13, 17, 23, 31, 47] {
        for xi in [-5i32, -2, 0, 2, 5, 9] {
            for an in [14.5_f64, 20.0, 25.0] {
                for beta in [0.0_f64, 20.0] {
                    for rr in [0.0_f64, 0.25, 0.38] {
                        let p = GearParams {
                            teeth: z,
                            profile_shift: f64::from(xi) * 0.1,
                            pressure_angle: an,
                            helix_angle: beta,
                            root_radius: rr,
                            ..Default::default()
                        };
                        let g = Tooth::new(p);
                        let rep = check_cut(&g, 150);
                        let env = fillet_envelope_error(&g, 150, 4000);
                        let sdf = sdf_matches_polyline(&g, 400, 4000);
                        let tag = format!(
                            "z={z} x={:+.1} a={an} b={beta} rho={rr}",
                            f64::from(xi) * 0.1
                        );
                        if rep.penetration > worst_pen {
                            worst_pen = rep.penetration;
                            pen_case = tag.clone();
                        }
                        if rep.deviation > worst_dev {
                            worst_dev = rep.deviation;
                            dev_case = tag;
                        }
                        worst_env = worst_env.max(env);
                        worst_sdf = worst_sdf.max(sdf);
                        n += 1;
                        if n >= limit {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
    println!("{n} cases");
    println!("  worst penetration        {worst_pen:.6e} mm   {pen_case}");
    println!("  worst deviation          {worst_dev:.6e} mm   {dev_case}");
    println!("  worst fillet envelope    {worst_env:.6e} mm");
    println!("  worst sdf vs polyline    {worst_sdf:.6e} mm");
}

/// Write a DXF to stdout, for inspecting or importing into CAD.
fn dxf(teeth: u32, x: f64, tol: f64) {
    let g = Tooth::new(GearParams {
        teeth,
        profile_shift: x,
        ..Default::default()
    });
    print!(
        "{}",
        gear_io::gear_to_dxf(
            &g,
            &gear_io::DxfOptions {
                chord_tolerance: tol,
                reference_circles: true,
            }
        )
    );
}

/// Emit an HTML page showing the bending construction, for visual checking.
fn bending_report() {
    let cases = [
        (9u32, 0.0),
        (13, 0.0),
        (17, 0.0),
        (17, 0.5),
        (30, 0.0),
        (60, 0.0),
        (12, -0.3),
        (12, 0.4),
    ];
    println!("<h2>The construction, tooth by tooth</h2>");
    println!(
        "<p class=\"lede\">Load at the tooth tip. \
        Grey dashed: tip, pitch, base and root circles. \
        <span class=\"k tangent\">Amber</span>: the 30° tangents and their tangency points. \
        <span class=\"k chord\">Red</span>: the critical section chord s_Fn. \
        <span class=\"k loadline\">Blue</span>: the load line, from the contact point to the \
        centreline. <span class=\"k arm\">Green</span>: the moment arm h_Fe.</p>"
    );
    println!("<div class=\"grid\">");
    for (teeth, profile_shift) in cases {
        let p = GearParams {
            teeth,
            profile_shift,
            ..Default::default()
        };
        println!("<figure>{}", diagram::tooth_diagram(p, 210.0));
        println!(
            "<figcaption>{}</figcaption></figure>",
            diagram::tooth_caption(p)
        );
    }
    println!("</div>");
    println!("<h2>Form factor against tooth count</h2>");
    println!("{}", diagram::form_factor_chart(660.0, 380.0));
}

/// Precision study of the bending-model matrix. Text, so the numbers can be
/// read before anything is drawn from them.
fn matrix_report() {
    // **Both kinds of member, through one harness.** A ring is the same four
    // studies rather than a second set beside them, which is the only way the
    // two answers can be compared at all — and comparing them is the point,
    // because the crate rates a ring by the construction it chose for an
    // external tooth.
    for member in [matrix::Member::External, matrix::Member::Internal] {
        let pop = matrix::population_for(member);
        println!(
            "\n################ {} ({} designs, tangent at {:.0}°) ################\n",
            member.name().to_uppercase(),
            pop.len(),
            member.tangent_angle()
        );

        println!("== 1. continuity ==");
        println!(
            "{:<24} {:>18} {:>10} {:>22}",
            "model", "worst z step", "at z", "worst x step (fine)"
        );
        let fine_at = match member {
            matrix::Member::External => 120,
            matrix::Member::Internal => 60,
        };
        for m in matrix::MATRIX {
            let (step, at) = matrix::continuity_in_tooth_count(m, member);
            let xs = matrix::continuity_in_profile_shift(m, member, fine_at);
            println!(
                "{:<24} {:>17.4}% {:>10} {:>21.5}%",
                m.name(member),
                100.0 * step,
                at,
                100.0 * xs
            );
        }

        println!("\n== 2. rank agreement (Spearman) ==");
        for i in 0..matrix::MATRIX.len() {
            for j in (i + 1)..matrix::MATRIX.len() {
                let (r, n) =
                    matrix::rank_correlation(matrix::MATRIX[i], matrix::MATRIX[j], member, &pop);
                println!(
                    "  {:<22} vs {:<22} rho = {r:.6}  (n={n})",
                    matrix::MATRIX[i].name(member),
                    matrix::MATRIX[j].name(member)
                );
            }
        }

        for thresh in [0.0_f64, 0.01] {
            println!(
                "\n== 3. gradient sign agreement, ignoring effects below {:.1}% ==",
                100.0 * thresh
            );
            println!(
                "  {:<48} {:>7} {:>7} {:>7}",
                "", "shift", "fillet", "dedend"
            );
            for i in 0..matrix::MATRIX.len() {
                for j in (i + 1)..matrix::MATRIX.len() {
                    let g = matrix::gradient_agreement(
                        matrix::MATRIX[i],
                        matrix::MATRIX[j],
                        member,
                        &pop,
                        thresh,
                    );
                    println!(
                        "  {:<22} vs {:<22} {:6.1}% {:6.1}% {:6.1}%",
                        matrix::MATRIX[i].name(member),
                        matrix::MATRIX[j].name(member),
                        100.0 * g[0],
                        100.0 * g[1],
                        100.0 * g[2]
                    );
                }
            }
        }

        println!("\n== 4. divergence across the matrix, by tooth count (x=0) ==");
        println!("{:>6} {:>12}", "z", "spread");
        let counts: &[u32] = match member {
            matrix::Member::External => &[9, 12, 17, 25, 40, 70, 120, 250],
            matrix::Member::Internal => &[40, 48, 60, 72, 90, 120, 160, 220],
        };
        for &teeth in counts {
            let p = GearParams {
                teeth,
                ..Default::default()
            };
            if let Some(d) = matrix::divergence(member, p) {
                println!("{teeth:>6} {:>11.2}%", 100.0 * d);
            }
        }
        let mut worst = (0.0_f64, GearParams::default());
        for p in &pop {
            if let Some(d) = matrix::divergence(member, *p) {
                if d > worst.0 {
                    worst = (d, *p);
                }
            }
        }
        println!(
            "   worst over the population: {:.2}% at z={} x={:+.1} alpha={}",
            100.0 * worst.0,
            worst.1.teeth,
            worst.1.profile_shift,
            worst.1.pressure_angle
        );

        // **5. Where the default parts from the standard's construction.** The
        // crate rates on the inscribed parabola; ISO specifies the tangent. This
        // is the size of that choice, on the number a designer actually feels.
        println!(
            "\n== 5. parabola against the {:.0}° tangent ==",
            member.tangent_angle()
        );
        let d = matrix::parting(member, &pop);
        #[allow(clippy::cast_precision_loss)]
        let pct = |k: usize| 100.0 * k as f64 / d.n as f64;
        println!("  designs rated by both        {}", d.n);
        println!(
            "  parabola tangency on flank   {} ({:.1}%)",
            d.on_flank,
            pct(d.on_flank)
        );
        println!(
            "  Y_F   parabola/tangent       {:.3} .. {:.3}   mean {:.3}",
            d.form[0], d.form[1], d.form[2]
        );
        println!(
            "  Y_F·Y_S parabola/tangent     {:.3} .. {:.3}   mean {:.3}",
            d.factor[0], d.factor[1], d.factor[2]
        );
        println!(
            "  mean q_s                     {:.3} parabola, {:.3} tangent",
            d.notch[0], d.notch[1]
        );
        println!(
            "  outside the Y_S band         {} parabola ({:.1}%), {} tangent ({:.1}%)",
            d.notch_out[0],
            pct(d.notch_out[0]),
            d.notch_out[1],
            pct(d.notch_out[1])
        );
    }

    // Which end of a ring's tooth is the thick one — the question the cantilever
    // picture rests on, and one worth measuring rather than asserting.
    println!("\n== 6. a ring tooth along its generated flank (tip -> root) ==");
    for teeth in [40u32, 90, 160] {
        let t = matrix::ring_flank_thickness(
            GearParams {
                teeth,
                ..Default::default()
            },
            4,
        );
        let cells: Vec<String> = t
            .iter()
            .map(|(r, w)| format!("r {r:.3} w {w:.4}"))
            .collect();
        println!("  z={teeth:<4} {}", cells.join("   "));
    }

    // **The fillet radius the notch factor is fed.** The crate reads it at the
    // junction when the parabola tangency is on the flank; the source the ring
    // model comes from defines it as the minimum over the whole fillet. `q_s`
    // is linear in it, so the gap between the two is the gap in `q_s`.
    println!("\n== 7. fillet radius: at the junction vs the fillet's minimum ==");
    println!(
        "{:<9} {:>5} {:>7} {:>12} {:>10} {:>9} {:>9}",
        "member", "z", "x", "at junction", "minimum", "ratio", "min at"
    );
    for member in [matrix::Member::External, matrix::Member::Internal] {
        let counts: &[u32] = match member {
            matrix::Member::External => &[17, 40, 100],
            matrix::Member::Internal => &[40, 90, 160],
        };
        for &teeth in counts {
            for shift in [-0.3_f64, 0.0, 0.3] {
                let p = GearParams {
                    teeth,
                    profile_shift: shift,
                    ..Default::default()
                };
                if let Some((j, m, at)) = matrix::fillet_radius_readings(member, p, 2000) {
                    println!(
                        "{:<9} {teeth:>5} {shift:>+7.1} {j:>12.4} {m:>10.4} {:>9.3} {:>9.2}",
                        member.name(),
                        j / m,
                        at
                    );
                }
            }
        }
    }
}

/// Compare the three load cases on a few ordinary meshes.
///
/// (a) worst case  -- load at the tip, this tooth carrying everything
/// (b) HPSTC       -- load at the highest point of single-pair contact
/// (c) shared      -- worst point of the mesh cycle with load sharing applied
fn loadcase_report() {
    use gear_core::contact::{ContactPath, LoadSharing};
    use gear_core::mesh::{Mesh, MeshKind};
    use gear_core::strength::{root_section, StressConcentration};

    let meshes = [
        ("pinion 17 : 17", 17u32, 17u32, 0.0_f64),
        ("pinion 17 : 43", 17, 43, 0.0),
        ("pinion 13 : 60", 13, 60, 0.0),
        ("pinion 25 : 25", 25, 25, 0.0),
        ("pinion 12 : 30, x=+0.4", 12, 30, 0.4),
        ("pinion 20 : 20, x=-0.2", 20, 20, -0.2),
    ];

    println!(
        "{:<24} {:>6} {:>9} {:>9} {:>9} {:>8} {:>8}",
        "mesh", "eps", "(a) tip", "(b) HPSTC", "(c) shared", "b vs a", "c vs a"
    );

    for (name, z1, z2, x) in meshes {
        let p1 = GearParams {
            teeth: z1,
            profile_shift: x,
            ..Default::default()
        };
        let p2 = GearParams {
            teeth: z2,
            profile_shift: -x,
            ..Default::default()
        };
        let (g1, g2) = (Tooth::new(p1), Tooth::new(p2));
        let Ok(m) = Mesh::new(&g1, &g2, MeshKind::External) else {
            println!("{name}: mesh failed");
            continue;
        };
        let Some(path) = ContactPath::new(&g1, g2.ra, &m) else {
            println!("{name}: path failed");
            continue;
        };

        // The bending factor is proportional to stress for a fixed torque, so
        // ratios of (factor x load fraction) are ratios of stress.
        let factor = |roll: f64| {
            root_section(&g1, roll).and_then(|s| s.bending_factor(StressConcentration::Iso6336))
        };

        let Some(a) = factor(path.roll_at(path.tip())) else {
            println!("{name}: tip factor failed");
            continue;
        };
        let Some(b) = factor(path.roll_at(path.highest_single_pair())) else {
            println!("{name}: hpstc factor failed");
            continue;
        };

        // (c): the worst point of the whole cycle once sharing is applied.
        let mut c = 0.0_f64;
        for i in 0..=400 {
            let t = f64::from(i) / 400.0;
            let xi = -path.approach + t * (path.approach + path.recess);
            if let Some(f) = factor(path.roll_at(xi)) {
                c = c.max(f * path.load_fraction(xi, LoadSharing::LinearRamp));
            }
        }

        println!(
            "{name:<24} {:>6.3} {a:>9.4} {b:>9.4} {c:>9.4} {:>7.1}% {:>7.1}%",
            path.contact_ratio,
            100.0 * (b - a) / a,
            100.0 * (c - a) / a
        );
    }
}

/// A worm pair, end to end: geometry, sliding, and both drive directions.
fn worm_report(starts: u32, wheel_teeth: u32, worm_diameter: f64, shaft_angle_deg: f64) {
    use gear_core::contact::{Directional, Drive};
    use gear_core::mesh::MeshSide;
    use gear_core::screw::{Screw, ScrewParams};

    let params = ScrewParams {
        starts,
        wheel_teeth,
        worm_pitch_diameter: worm_diameter,
        shaft_angle: shaft_angle_deg.to_radians(),
        ..Default::default()
    };
    let s = match Screw::new(&params) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot build that pair: {e:?}");
            return;
        }
    };

    println!(
        "worm   z {}/{}  module {}  alpha_n {:.1} deg  shaft angle {:.1} deg",
        starts,
        wheel_teeth,
        params.normal_module,
        params.normal_pressure_angle.to_degrees(),
        shaft_angle_deg
    );
    println!(
        "       ratio {:.4}:1   centre distance {:.4} mm",
        s.ratio, s.centre_distance
    );
    println!();
    println!("geometry");
    println!(
        "  lead angle          worm {:8.4} deg    wheel {:8.4} deg",
        s.lead_angle.to_degrees(),
        s.wheel_lead_angle.to_degrees()
    );
    println!(
        "  helix angle         worm {:8.4} deg    wheel {:8.4} deg",
        s.worm_helix_angle.to_degrees(),
        s.wheel_helix_angle.to_degrees()
    );
    println!(
        "  pitch diameter      worm {:8.4} mm     wheel {:8.4} mm",
        s.worm_pitch_diameter, s.wheel_pitch_diameter
    );
    println!(
        "  lead {:.4} mm   axial module {:.5} mm",
        s.lead, s.axial_module
    );
    println!(
        "  sliding at the pitch point   {:.4} x the worm's pitch line speed",
        s.sliding_ratio
    );

    println!();
    println!("efficiency          worm driving   wheel driving");
    for mu in [0.0, 0.02, 0.04, 0.06, 0.10] {
        let e = Directional::of(|d| s.efficiency(mu, d));
        let back = if e.self_locking() {
            "  self-locking".to_string()
        } else {
            format!("{:12.3} %", e.backward * 100.0)
        };
        println!("  mu {mu:.2}        {:10.3} % {back}", e.forward * 100.0);
    }
    let threshold = s.self_locking_friction();
    println!("  self-locks at mu >= {threshold:.4}   (cos alpha_n tan gamma)");

    // Contact is the strength figure a worm stage reports. There is deliberately
    // no bending stress here; docs/reference.md#crossed-axes says why.
    let lib = gear_io::default_library();
    let (worm_material, wheel_material) = ("4340 Hardened Steel", "Brass C360");
    let (Some(m1), Some(m2)) = (lib.get(worm_material), lib.get(wheel_material)) else {
        return;
    };
    let e_star = gear_core::material::contact_modulus(m1, m2);
    let mu = 0.06;
    let torque_in = 2.0;
    let torque_out = torque_in * s.ratio * s.efficiency(mu, Drive::Forward);

    println!();
    println!("contact   {worm_material} on {wheel_material},  E* {e_star:.0} MPa");
    println!("  worm torque {torque_in:.3} Nm  ->  wheel torque {torque_out:.3} Nm at mu {mu}");
    let (flat, sharp) = match s.contact_curvatures() {
        Some(c) => c,
        None => {
            println!("  the flanks do not touch at a point");
            return;
        }
    };
    println!("  relative curvature   along {flat:.6} /mm   across {sharp:.6} /mm");
    if let Some(c) = s.contact(torque_out, MeshSide::Second, mu, e_star) {
        println!(
            "  patch  {:.4} x {:.4} mm   (rated on the wheel's torque)",
            c.semi_major() * 2.0,
            c.semi_minor() * 2.0
        );
        println!("  peak pressure               {:.1} MPa", c.max_pressure);
        println!(
            "  sliding speed at 3000 rpm   {:.1} mm/s",
            s.sliding_ratio * 3000.0 / 60.0 * std::f64::consts::TAU * s.worm_pitch_diameter / 2.0
        );
    }
    println!("  bending                     not reported - see docs/reference.md#crossed-axes");
    println!(
        "  flank type   ZI (involute helicoid). A ZN worm's contact stress is\n\
         {:24}1-15 % lower, rising with lead angle - see docs/reference.md#crossed-axes",
        ""
    );
}

/// A worm stage end to end: geometry, both directions, contact and backlash.
fn worm_stage_report(starts: u32, wheel_teeth: u32, worm_diameter: f64, torque: f64) {
    use gear_core::train::{solve_worm_stage, StageTorques, WormMember, WormStage};

    let stage = WormStage {
        starts,
        wheel_teeth,
        sizing: gear_core::train::FirstMemberSizing::PitchDiameter(worm_diameter),
        wheel: WormMember {
            material: "Brass C360".into(),
            ..WormMember::default()
        },
        ..WormStage::default()
    };
    let lib = gear_io::default_library();
    let r = match solve_worm_stage(&stage, StageTorques::just(torque), &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot solve that stage: {e}");
            return;
        }
    };

    println!(
        "worm stage  z {starts}/{wheel_teeth}  module {}  ratio {:.4}:1  a {:.4} mm",
        stage.module, r.ratio, r.centre_distance
    );
    println!(
        "  lead angle {:.4} deg   wheel helix {:.4} deg   lead {:.4} mm",
        r.lead_angle, r.wheel_helix_angle, r.lead
    );
    println!();
    println!("  member      torque Nm   face mm   d mm      material");
    for (name, m) in ["worm", "wheel"].iter().zip(&r.members) {
        println!(
            "  {name:<10} {:9.4} {:9.3} {:9.4}   {}",
            m.torque, m.face_width, m.pitch_diameter, m.material.name
        );
    }
    println!();
    println!(
        "  efficiency   forward {:.3} %   backward {:.3} %{}",
        r.efficiency.forward * 100.0,
        r.efficiency.backward * 100.0,
        if r.efficiency.self_locking() {
            "  (self-locking)"
        } else {
            ""
        }
    );
    println!(
        "  contact      {:.1} MPa   patch {:.4} x {:.4} mm",
        r.contact.peak.max_pressure, r.contact.peak.patch_length, r.contact.peak.patch_width
    );
    println!(
        "  backlash     at the wheel {:.5} deg (min {:.5}, max {:.5})   at the worm {:.5} deg",
        r.backlash.forward.nominal,
        r.backlash.forward.minimum,
        r.backlash.forward.maximum,
        r.backlash.backward.nominal
    );
    println!("  bending      not reported - see docs/reference.md#crossed-axes");
    println!(
        "  flank type   ZI (involute helicoid); a ZN worm's contact stress is\n\
         {:15}1-15 % lower, rising with lead angle - see docs/reference.md#crossed-axes",
        ""
    );
    for note in &r.notes {
        println!("  ! {}", words().render(note));
    }
}

/// The ring tooth counts a sun and planet pair admits, and what each costs in
/// planet shift.
///
/// Prints every candidate rather than picking one, because the choice is a
/// designer's: the geometric ideal needs no shift but rarely spaces the planets
/// evenly, and the one that does costs a shift. Both facts are on the same row.
fn planetary_report(sun: u32, planet: u32, planets: u32, sun_shift: f64, ring_shift: f64) {
    use gear_core::plane::BasicRack;
    use gear_core::planetary::{ring_candidates, shift_bracket, solve, Set, Teeth};

    let module = 1.0;
    let set = Set {
        rack: BasicRack::new(module, 20.0, 0.0),
        teeth: Teeth {
            sun,
            planet,
            ring: 0,
        },
        planets,
        // The ring search is the planet's: its completeness rests on the
        // planet's shift rising with the ring's count.
        shift: [sun_shift, 0.0, ring_shift],
        absorber: gear_core::planetary::Member::Planet,
        // A planet's tip diameter at a standard addendum, which is what the
        // clearance column is measured against.
        planet_tip_diameter: module * (f64::from(planet) + 2.0),
    };

    println!(
        "planetary  z_sun {sun}  z_planet {planet}  N {planets}  \
         x_sun {sun_shift}  x_ring {ring_shift}  module {module}  alpha 20 deg"
    );
    println!(
        "ideal ring (needs no planet shift): {}",
        Teeth::ideal_ring(sun, planet)
    );

    // Everything the involute domain admits, whatever shift it costs. The range
    // is deliberately wide: this is the "what is possible" listing.
    let all = ring_candidates(
        &set,
        (f64::NEG_INFINITY, f64::INFINITY),
        4 * (sun + 2 * planet),
    );
    if all.is_empty() {
        println!("\nno ring tooth count admits a solution for that sun and planet");
        return;
    }

    println!(
        "\n{:>6} {:>10} {:>12} {:>10} {:>9} {:>7} {:>7} {:>11}",
        "z_ring", "x_planet", "c2c mm", "residual", "a_w sun", "even", "simult", "clearance"
    );
    for (ring, l) in &all {
        let clearance = l
            .planet_clearance
            .map_or_else(|| "     n/a".to_string(), |c| format!("{c:8.3}"));
        println!(
            "{ring:>6} {:>10.4} {:>12.6} {:>10.1e} {:>9.3} {:>7} {:>7} {clearance:>11}",
            l.shift[gear_core::planetary::Member::Planet.index()],
            l.centre_distance,
            l.residual,
            l.alpha_w_sun.to_degrees(),
            if l.equal_spacing { "yes" } else { "no" },
            if l.simultaneous_meshing { "yes" } else { "no" },
        );
    }

    // The bracket is why the list stops where it does, so say so with numbers.
    let widest = all.last().map(|(z, _)| *z).unwrap_or(0);
    let beyond = Set {
        teeth: Teeth {
            ring: widest + 1,
            ..set.teeth
        },
        ..set
    };
    print!("\nwhy it stops: z_ring {} ", widest + 1);
    match shift_bracket(&beyond) {
        None => println!("has no admissible planet shift at all"),
        Some((lo, hi)) => {
            let inside = solve(&beyond).is_some();
            println!(
                "admits x_planet in [{lo:.4}, {hi:.4}] but {}",
                if inside {
                    "was excluded by the sweep limit"
                } else {
                    "no shift in it equalises the two centre distances"
                }
            );
        }
    }
}

/// A planetary stage, end to end, in all six arrangements.
fn planetary_stage_report(sun: u32, planet: u32, ring: u32, planets: u32, helix: f64) {
    use gear_core::planetary::{Arrangement, PlanetaryShaft};
    use gear_core::train::{solve_planetary_stage, PlanetaryStage, StageGear};

    let lib = gear_io::default_library();
    let base = PlanetaryStage {
        helix_angle: helix,
        planets,
        sun: StageGear {
            teeth: sun,
            ..StageGear::default()
        },
        planet: StageGear {
            teeth: planet,
            ..StageGear::default()
        },
        ring: StageGear {
            teeth: ring,
            profile_shift: gear_core::Auto::fixed(0.0),
            ..StageGear::default()
        },
        ..PlanetaryStage::default()
    };

    println!(
        "planetary stage  z {sun}/{planet}/{ring}  N={planets}  helix {helix} deg  \
         module {}  alpha {} deg",
        base.module, base.pressure_angle
    );
    let all = [
        PlanetaryShaft::Sun,
        PlanetaryShaft::Carrier,
        PlanetaryShaft::Ring,
    ];
    let name = |m: PlanetaryShaft| match m {
        PlanetaryShaft::Sun => "sun",
        PlanetaryShaft::Carrier => "carrier",
        PlanetaryShaft::Ring => "ring",
    };

    let mut shown = false;
    for &input in &all {
        for &fixed in &all {
            if input == fixed {
                continue;
            }
            let stage = PlanetaryStage {
                arrangement: Arrangement { input, fixed },
                ..base.clone()
            };
            match solve_planetary_stage(
                &stage,
                3000.0,
                gear_core::train::StageTorques::just(2.0),
                &lib,
            ) {
                Err(e) => println!("  {:>7} in, {:>7} held: {e}", name(input), name(fixed)),
                Ok(r) => {
                    if !shown {
                        println!(
                            "\ncommon centre distance {:.6} mm (residual {:.1e})  \
                             planet shift {:+.4}",
                            r.centre_distance_nominal,
                            r.planet.shift_residual,
                            r.planet.gear.profile_shift
                        );
                        println!(
                            "eps_a  sun-planet {:.3}   planet-ring {:.3}   \
                             eta_0 {:.4}   even spacing {}   planet gap {}",
                            r.sun_planet.contact_ratios.transverse,
                            r.planet_ring.contact_ratios.transverse,
                            r.fixed_carrier_efficiency.forward,
                            r.equal_spacing,
                            r.planet_clearance
                                .map_or_else(|| "n/a".into(), |g| format!("{g:.3} mm"))
                        );
                        println!(
                            "sigma_H at pitch  sun-planet {:.1} MPa   planet-ring {:.1} MPa",
                            r.sun_planet.contact_stress_at_pitch_point.peak,
                            r.planet_ring.contact_stress_at_pitch_point.peak
                        );
                        println!(
                            "sigma_F  sun {}   planet {}   ring {}",
                            r.sun
                                .bending_stress
                                .peak
                                .map_or_else(|| "-".into(), |v| format!("{v:.1} MPa")),
                            r.planet
                                .gear
                                .bending_stress
                                .peak
                                .map_or_else(|| "-".into(), |v| format!("{v:.1} MPa")),
                            r.ring
                                .bending_stress
                                .peak
                                .map_or_else(|| "-".into(), |v| format!("{v:.1} MPa")),
                        );
                        println!(
                            "\n{:>9} {:>9} {:>9} {:>12} {:>11}",
                            "input", "held", "output", "ratio", "efficiency"
                        );
                        shown = true;
                    }
                    println!(
                        "{:>9} {:>9} {:>9} {:>12.4} {:>10.3} %",
                        name(input),
                        name(fixed),
                        name(r.output),
                        r.ratio,
                        r.efficiency.forward * 100.0
                    );
                }
            }
        }
    }
    if !shown {
        return;
    }
    let stage = base;
    if let Ok(r) = solve_planetary_stage(
        &stage,
        3000.0,
        gear_core::train::StageTorques::just(2.0),
        &lib,
    ) {
        println!();
        for note in &r.notes {
            println!("note: {}", words().render(note));
        }
    }
}

/// A crossed gear pair, swept over how the shaft angle divides between the two
/// members.
///
/// The split is the design freedom a worm does not have: a worm's diameter is
/// chosen and its lead angle follows, while two gears have their diameters fixed
/// by tooth count and helix, so `β₁` is what there is to choose. Nothing else
/// about the pair changes — it is the same screw geometry either way.
fn crossed_report(z1: u32, z2: u32, shaft_angle: f64) {
    use gear_core::train::{solve_worm_stage, FirstMemberSizing, StageTorques, WormStage};

    let lib = gear_io::default_library();
    println!(
        "crossed gear pair  z {z1}/{z2}  shaft angle {shaft_angle} deg  module 1  alpha 20 deg  \
         mu 0.06"
    );
    println!(
        "\n{:>7} {:>7} {:>9} {:>9} {:>10} {:>10} {:>11} {:>10} {:>13}",
        "beta1", "beta2", "d1 mm", "d2 mm", "a mm", "slide/v1", "eta fwd", "sigma_H", "epsilon"
    );
    let mut any = false;
    for i in 0..=10 {
        #[allow(clippy::cast_precision_loss)]
        let beta1 = shaft_angle * (i as f64 / 10.0);
        let stage = WormStage {
            shaft_angle,
            starts: z1,
            wheel_teeth: z2,
            sizing: FirstMemberSizing::HelixAngle(beta1),
            ..WormStage::default()
        };
        let Ok(g) = stage.geometry() else {
            println!("{beta1:>7.1} {:>7} — no such pair", shaft_angle - beta1);
            continue;
        };
        match solve_worm_stage(&stage, StageTorques::just(2.0), &lib) {
            Err(e) => println!(
                "{beta1:>7.1} {:>7.1}  {e}",
                g.wheel_helix_angle.to_degrees()
            ),
            Ok(r) => {
                any = true;
                println!(
                    "{beta1:>7.1} {:>7.1} {:>9.4} {:>9.4} {:>10.4} {:>10.4} {:>10.3} % {:>9.1} {:>13}",
                    g.wheel_helix_angle.to_degrees(),
                    g.worm_pitch_diameter,
                    g.wheel_pitch_diameter,
                    g.centre_distance,
                    g.sliding_ratio,
                    r.efficiency.forward * 100.0,
                    r.contact.peak.max_pressure,
                    r.crossed.as_ref().map_or_else(
                        || "—".to_string(),
                        |c| format!("{:.9}", c.contact_ratio)
                    )
                );
            }
        }
    }
    if any {
        println!(
            "\nA worm is the same geometry with the first member's diameter chosen instead \
             of its helix;\nthe split above is the freedom two gears have and a worm does not."
        );
    }
}
