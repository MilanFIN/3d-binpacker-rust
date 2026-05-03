__kernel void vector_add(__global const float *a,
                         __global const float *b,
                         __global float *c)
{
    uint gid = get_global_id(0);
    c[gid] = a[gid] + b[gid];
}