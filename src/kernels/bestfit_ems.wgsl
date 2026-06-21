// =============================================================================
// Best-Fit EMS bin packing – WebGPU compute shader
// Ported from bestfit_ems.cl.template
// =============================================================================

// Pipeline-overridable constants (set at pipeline creation time)
override MAX_BINS: u32            = 64u;
override MAX_SPACES_PER_BIN: u32  = 512u;

// =============================================================================
// Data structures
// =============================================================================

struct Box_ {
    w: f32,
    h: f32,
    d: f32,
    weight: f32,
}

struct Space_ {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    h: f32,
    d: f32,
}


// Uniform params block – must be padded to a multiple of 16 bytes
struct Params {
    num_boxes:        u32,
    bin_w:            f32,
    bin_h:            f32,
    bin_d:            f32,
    bin_weight_limit: f32,
    rotation_mask:    u32,
    batch_offset:     u32,
    _pad1:            u32,
}

// =============================================================================
// Bindings
// =============================================================================

@group(0) @binding(0) var<storage, read>       boxes:            array<Box_>;
@group(0) @binding(1) var<storage, read>       orders:           array<i32>;
@group(0) @binding(2) var<storage, read_write> scores:           array<f32>;
@group(0) @binding(3) var<uniform>             params:           Params;

// Per-invocation working storage (indexed with gid as the outer dimension)
// spaces_store : [gid * MAX_BINS * MAX_SPACES_PER_BIN + b * MAX_SPACES_PER_BIN + s]
// space_counts : [gid * MAX_BINS + b]
// used_volumes : [gid * MAX_BINS + b]
// bin_wts      : [gid * MAX_BINS + b]
@group(0) @binding(4) var<storage, read_write> spaces_store: array<Space_>;
@group(0) @binding(5) var<storage, read_write> space_counts: array<u32>;
@group(0) @binding(6) var<storage, read_write> used_volumes: array<f32>;
@group(0) @binding(7) var<storage, read_write> bin_wts:      array<f32>;

// =============================================================================
// Helper functions
// =============================================================================

fn check_collision(
    bx: f32, by: f32, bz: f32, bw: f32, bh: f32, bd: f32,
    sx: f32, sy: f32, sz: f32, sw: f32, sh: f32, sd: f32,
) -> bool {
    return bx < sx + sw && bx + bw > sx &&
           by < sy + sh && by + bh > sy &&
           bz < sz + sd && bz + bd > sz;
}

fn is_contained(
    s1x: f32, s1y: f32, s1z: f32, s1w: f32, s1h: f32, s1d: f32,
    s2x: f32, s2y: f32, s2z: f32, s2w: f32, s2h: f32, s2d: f32,
) -> bool {
    return s1x >= s2x && s1y >= s2y && s1z >= s2z &&
           s1x + s1w <= s2x + s2w &&
           s1y + s1h <= s2y + s2h &&
           s1z + s1d <= s2z + s2d;
}

// =============================================================================
// Kernel
// =============================================================================

@compute @workgroup_size(1)
fn best_fit_ems(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x + params.batch_offset;

    let num_boxes        = params.num_boxes;
    let bin_w            = params.bin_w;
    let bin_h            = params.bin_h;
    let bin_d            = params.bin_d;
    let bin_weight_limit = params.bin_weight_limit;
    let rotation_mask    = params.rotation_mask;

    // Base offsets into per-invocation storage
    let spaces_base = gid * MAX_BINS * MAX_SPACES_PER_BIN;
    let cb  = gid * MAX_BINS;   // counts / volumes / weights base

    // ------------------------------------------------------------------
    // Initialize state
    // ------------------------------------------------------------------
    var bins_used: u32 = 1u;

    space_counts[cb + 0u]     = 1u;
    used_volumes[cb + 0u]     = 0.0;
    bin_wts[cb + 0u]          = 0.0;
    spaces_store[spaces_base] = Space_(0.0, 0.0, 0.0, bin_w, bin_h, bin_d);

    for (var b: u32 = 1u; b < MAX_BINS; b++) {
        space_counts[cb + b] = 0u;
        used_volumes[cb + b] = 0.0;
        bin_wts[cb + b]      = 0.0;
    }

    // ------------------------------------------------------------------
    // Packing loop
    // ------------------------------------------------------------------
    for (var i: u32 = 0u; i < num_boxes; i++) {

        let box_id = u32(orders[gid * num_boxes + i]);
        let bx     = boxes[box_id];

        // Precompute 4 orientations  [w, h, d]
        var ow: array<f32, 4>;
        var oh: array<f32, 4>;
        var od: array<f32, 4>;
        ow[0] = bx.w; oh[0] = bx.h; od[0] = bx.d;
        ow[1] = bx.w; oh[1] = bx.d; od[1] = bx.h;
        ow[2] = bx.h; oh[2] = bx.w; od[2] = bx.d;
        ow[3] = bx.d; oh[3] = bx.h; od[3] = bx.w;

        var placed: bool = false;

        // ----- 1. Find best fit -----
        var best_bin:    i32 = -1;
        var best_space:  i32 = -1;
        var best_orient: i32 = -1;
        var best_score:  f32 = 3.4028235e+38;

        for (var b: u32 = 0u; b < bins_used; b++) {
            if (bin_weight_limit > 0.0 && bin_wts[cb + b] + bx.weight > bin_weight_limit) {
                continue;
            }

            let bsb = spaces_base + b * MAX_SPACES_PER_BIN;
            let sc  = space_counts[cb + b];

            for (var s: u32 = 0u; s < sc; s++) {
                let sp = spaces_store[bsb + s];

                for (var o: i32 = 0; o < 4; o++) {
                    if (o == 1 && (rotation_mask & 1u) == 0u) { continue; }
                    if (o == 2 && (rotation_mask & 2u) == 0u) { continue; }
                    if (o == 3 && (rotation_mask & 4u) == 0u) { continue; }

                    let w = ow[o]; let h = oh[o]; let d = od[o];

                    if (w <= sp.w && h <= sp.h && d <= sp.d) {
                        // Score: distance from origin + bin penalty (match Java first-bin-best-fit)
                        let score = sp.x + sp.y + sp.z + f32(b) * 100000.0;
                        if (score < best_score) {
                            best_score  = score;
                            best_bin    = i32(b);
                            best_space  = i32(s);
                            best_orient = o;
                        }
                    }
                }
            }

            // First-bin-best-fit semantics: stop after the first bin that has a fit
            if (best_bin == i32(b)) { break; }
        }

        // ----- 2. Place box -----
        if (best_bin >= 0) {
            placed = true;
            let b   = u32(best_bin);
            let si  = u32(best_space);
            let o   = best_orient;
            let bsb = spaces_base + b * MAX_SPACES_PER_BIN;

            let sp    = spaces_store[bsb + si];
            let box_w = ow[o]; let box_h = oh[o]; let box_d = od[o];
            let box_x = sp.x;  let box_y = sp.y;  let box_z = sp.z;

            used_volumes[cb + b] += box_w * box_h * box_d;
            bin_wts[cb + b]      += bx.weight;

            // Remove used space (swap with last)
            var sc: u32 = space_counts[cb + b];
            sc--;
            space_counts[cb + b] = sc;
            spaces_store[bsb + si] = spaces_store[bsb + sc];

            // A. Add BSP splits from the placed space
            if (sp.w - box_w > 0.0 && sc < MAX_SPACES_PER_BIN) {
                spaces_store[bsb + sc] = Space_(sp.x + box_w, sp.y, sp.z, sp.w - box_w, sp.h, sp.d);
                sc++; space_counts[cb + b] = sc;
            }
            if (sp.h - box_h > 0.0 && sc < MAX_SPACES_PER_BIN) {
                spaces_store[bsb + sc] = Space_(sp.x, sp.y + box_h, sp.z, sp.w, sp.h - box_h, sp.d);
                sc++; space_counts[cb + b] = sc;
            }
            if (sp.d - box_d > 0.0 && sc < MAX_SPACES_PER_BIN) {
                spaces_store[bsb + sc] = Space_(sp.x, sp.y, sp.z + box_d, sp.w, sp.h, sp.d - box_d);
                sc++; space_counts[cb + b] = sc;
            }

            // B. Prune intersecting spaces (EMS) – iterate backwards
            var k: i32 = i32(space_counts[cb + b]) - 1;
            loop {
                if (k < 0) { break; }
                let sc_now = space_counts[cb + b];
                if (u32(k) >= sc_now) { k--; continue; }

                let other = spaces_store[bsb + u32(k)];

                if (check_collision(box_x, box_y, box_z, box_w, box_h, box_d,
                                    other.x, other.y, other.z, other.w, other.h, other.d)) {
                    // Remove (swap with last)
                    let last = space_counts[cb + b] - 1u;
                    space_counts[cb + b] = last;
                    spaces_store[bsb + u32(k)] = spaces_store[bsb + last];

                    // Split into up to 6 sub-spaces
                    if (box_x + box_w < other.x + other.w) {
                        let ns = space_counts[cb + b];
                        if (ns < MAX_SPACES_PER_BIN) {
                            spaces_store[bsb + ns] = Space_(box_x + box_w, other.y, other.z,
                                (other.x + other.w) - (box_x + box_w), other.h, other.d);
                            space_counts[cb + b] = ns + 1u;
                        }
                    }
                    if (box_x > other.x) {
                        let ns = space_counts[cb + b];
                        if (ns < MAX_SPACES_PER_BIN) {
                            spaces_store[bsb + ns] = Space_(other.x, other.y, other.z,
                                box_x - other.x, other.h, other.d);
                            space_counts[cb + b] = ns + 1u;
                        }
                    }
                    if (box_y + box_h < other.y + other.h) {
                        let ns = space_counts[cb + b];
                        if (ns < MAX_SPACES_PER_BIN) {
                            spaces_store[bsb + ns] = Space_(other.x, box_y + box_h, other.z,
                                other.w, (other.y + other.h) - (box_y + box_h), other.d);
                            space_counts[cb + b] = ns + 1u;
                        }
                    }
                    if (box_y > other.y) {
                        let ns = space_counts[cb + b];
                        if (ns < MAX_SPACES_PER_BIN) {
                            spaces_store[bsb + ns] = Space_(other.x, other.y, other.z,
                                other.w, box_y - other.y, other.d);
                            space_counts[cb + b] = ns + 1u;
                        }
                    }
                    if (box_z + box_d < other.z + other.d) {
                        let ns = space_counts[cb + b];
                        if (ns < MAX_SPACES_PER_BIN) {
                            spaces_store[bsb + ns] = Space_(other.x, other.y, box_z + box_d,
                                other.w, other.h, (other.z + other.d) - (box_z + box_d));
                            space_counts[cb + b] = ns + 1u;
                        }
                    }
                    if (box_z > other.z) {
                        let ns = space_counts[cb + b];
                        if (ns < MAX_SPACES_PER_BIN) {
                            spaces_store[bsb + ns] = Space_(other.x, other.y, other.z,
                                other.w, other.h, box_z - other.z);
                            space_counts[cb + b] = ns + 1u;
                        }
                    }
                    // Do NOT decrement k – re-check the swapped-in element
                } else {
                    k--;
                }
            }

            // C. Prune fully contained spaces  O(N²)
            var ii: i32 = i32(space_counts[cb + b]) - 1;
            loop {
                if (ii < 0) { break; }
                let sc2 = space_counts[cb + b];
                if (u32(ii) >= sc2) { ii--; continue; }

                let s1 = spaces_store[bsb + u32(ii)];

                // Remove degenerate spaces
                if (s1.w <= 0.0 || s1.h <= 0.0 || s1.d <= 0.0) {
                    let last2 = space_counts[cb + b] - 1u;
                    space_counts[cb + b] = last2;
                    spaces_store[bsb + u32(ii)] = spaces_store[bsb + last2];
                    ii--;
                    continue;
                }

                var contained: bool = false;
                let sc3 = space_counts[cb + b];
                for (var jj: u32 = 0u; jj < sc3; jj++) {
                    if (jj == u32(ii)) { continue; }
                    let s2 = spaces_store[bsb + jj];
                    if (is_contained(s1.x, s1.y, s1.z, s1.w, s1.h, s1.d,
                                     s2.x, s2.y, s2.z, s2.w, s2.h, s2.d)) {
                        contained = true;
                        break;
                    }
                }
                if (contained) {
                    let last3 = space_counts[cb + b] - 1u;
                    space_counts[cb + b] = last3;
                    spaces_store[bsb + u32(ii)] = spaces_store[bsb + last3];
                }
                ii--;
            }
        }

        // ----- 3. Open new bin if box could not be placed -----
        if (!placed) {
            if (bins_used >= MAX_BINS) {
                scores[gid] = -2.0; // overflow error code
                return;
            }

            let b   = bins_used;
            bins_used++;
            let bsb = spaces_base + b * MAX_SPACES_PER_BIN;

            // Pick valid orientation
            var new_orient: i32 = 0;
            for (var o: i32 = 0; o < 4; o++) {
                if (o == 1 && (rotation_mask & 1u) == 0u) { continue; }
                if (o == 2 && (rotation_mask & 2u) == 0u) { continue; }
                if (o == 3 && (rotation_mask & 4u) == 0u) { continue; }
                if (ow[o] <= bin_w && oh[o] <= bin_h && od[o] <= bin_d) {
                    new_orient = o;
                    break;
                }
            }

            let nbw = ow[new_orient]; let nbh = oh[new_orient]; let nbd = od[new_orient];

            used_volumes[cb + b] = nbw * nbh * nbd;
            bin_wts[cb + b]      = bx.weight;
            var sc: u32          = 0u;

            if (nbw < bin_w) {
                spaces_store[bsb + sc] = Space_(nbw, 0.0, 0.0, bin_w - nbw, bin_h, bin_d);
                sc++;
            }
            if (nbh < bin_h) {
                spaces_store[bsb + sc] = Space_(0.0, nbh, 0.0, bin_w, bin_h - nbh, bin_d);
                sc++;
            }
            if (nbd < bin_d) {
                spaces_store[bsb + sc] = Space_(0.0, 0.0, nbd, bin_w, bin_h, bin_d - nbd);
                sc++;
            }
            space_counts[cb + b] = sc;
        }
    }

    // ------------------------------------------------------------------
    // Scoring  (mirrors OpenCL version exactly)
    // ------------------------------------------------------------------
    var score: f32 = 0.0;
    let bin_vol = bin_w * bin_h * bin_d;

    if (bins_used == 1u) {
        score = used_volumes[cb + 0u];
    } else {
        for (var b: u32 = 0u; b < bins_used - 1u; b++) {
            score += used_volumes[cb + b];
        }
        if (bin_vol > 0.0) {
            score /= f32(bins_used - 1u) * bin_vol;
        }
    }

    scores[gid] = score;
}
