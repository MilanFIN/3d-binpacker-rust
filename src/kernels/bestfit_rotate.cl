
float calculate_score(float x, float y, float z, float w, float h, float d, float bin_index) {
    // Simple calculation for now
    // Add 1.0 to ensure score is always > 0 for valid placements
    return x + y + z + 1.0f;
}

__kernel void bestfit_rotate(
    float box_w, float box_h, float box_d,
    __global const float *spaces,
    __global float *results
) {
    int gid = get_global_id(0);
    
    // Space array contents: (x, y, z, w, h, d, bin_index)
    int space_offset = gid * 7;
    float space_x = spaces[space_offset];
    float space_y = spaces[space_offset + 1];
    float space_z = spaces[space_offset + 2];
    float space_w = spaces[space_offset + 3];
    float space_h = spaces[space_offset + 4];
    float space_d = spaces[space_offset + 5];
    float bin = spaces[space_offset + 6];

    // Result index for this space (6 slots reserved per space)
    int res_offset = gid * 6;

    float score = calculate_score(space_x, space_y, space_z, space_w, space_h, space_d, bin);

    // 1. default orientation (x, y, z)
    results[res_offset] = (box_w <= space_w && box_h <= space_h && box_d <= space_d) ? score : 0.0f;
    // 2. (x, z, y)
    results[res_offset + 1] = (box_w <= space_w && box_d <= space_h && box_h <= space_d) ? score : 0.0f;
    // 3. (y, x, z)
    results[res_offset + 2] = (box_h <= space_w && box_w <= space_h && box_d <= space_d) ? score : 0.0f;
    // 4. (y, z, x)
    results[res_offset + 3] = (box_h <= space_w && box_d <= space_h && box_w <= space_d) ? score : 0.0f;
    // 5. (z, x, y)
    results[res_offset + 4] = (box_d <= space_w && box_w <= space_h && box_h <= space_d) ? score : 0.0f;
    // 6. (z, y, x)
    results[res_offset + 5] = (box_d <= space_w && box_h <= space_h && box_w <= space_d) ? score : 0.0f;
}
