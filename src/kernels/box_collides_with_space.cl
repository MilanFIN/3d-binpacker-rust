__kernel void box_collides_with_space(
    float box_x, float box_y, float box_z, float box_w, float box_h, float box_d,
    __global const float* space_x,
    __global const float* space_y,
    __global const float* space_z,
    __global const float* space_w,
    __global const float* space_h,
    __global const float* space_d,
    __global int* result
) {
    int i = get_global_id(0);

    // Check collision based on AABB logic
    int collision = 
        (box_x < space_x[i] + space_w[i]) &&
        (box_y < space_y[i] + space_h[i]) &&
        (box_z < space_z[i] + space_d[i]) &&
        (box_x + box_w > space_x[i]) &&
        (box_y + box_h > space_y[i]) &&
        (box_z + box_d > space_z[i]);

    result[i] = collision;
}
