
__kernel void firstfit(
    float box_w, float box_h, float box_d,
    __global const float *spaces,
    __global float *results
) {
    int gid = get_global_id(0);
    // Assuming Space struct in memory is float[3]: {w, h, d}
    int space_offset = gid * 3;
    float space_w = spaces[space_offset + 0];
    float space_h = spaces[space_offset + 1];
    float space_d = spaces[space_offset + 2];

    // Result is 1 element per space [fits]
    int res_offset = gid;



    // 1. (x, y, z)
    if (box_w <= space_w && box_h <= space_h && box_d <= space_d) {
        results[res_offset] = 1.0f;
        return;
    }
    else {
        results[res_offset] = 0.0f;
        return;
    }

}

