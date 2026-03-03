# rube

Software rendered voxel engine using sparse-64 ray marching.

## Graphics Features
- Ray marched voxels
- Directional light hard shadows
- Noisy per voxel global illumination

# Perf
This section contains data about the performance of the application so that I may
refer back to it after optimization.

## Naive

### Memory
An empty winit application with no `softbuffer` surface.
```
======================================================================
rube-bin [39604]: 64-bit    Footprint: 13 MB (16384 bytes per page)
======================================================================

  Dirty      Clean  Reclaimable    Regions    Category
    ---        ---          ---        ---    ---
2608 KB        0 B        48 KB          1    MALLOC_NANO
2016 KB        0 B          0 B          8    MALLOC_SMALL
1527 KB        0 B          0 B        902    __DATA
1008 KB        0 B          0 B          7    CoreUI image data
1008 KB        0 B          0 B         15    MALLOC_TINY
 880 KB        0 B      1392 KB          8    MALLOC_MEDIUM
 674 KB        0 B          0 B        335    __DATA_DIRTY
 594 KB        0 B          0 B          1    page table
 537 KB        0 B          0 B        681    __AUTH
 528 KB        0 B          0 B         27    ColorSync
 384 KB        0 B          0 B         27    untagged (VM_ALLOCATE)
 336 KB        0 B          0 B         27    MALLOC metadata
 254 KB        0 B          0 B       1980    unused dyld shared cache area
 240 KB        0 B          0 B         15    CoreAnimation
 208 KB        0 B          0 B         10    stack
 128 KB        0 B          0 B          1    Accelerate image backing stores
  96 KB        0 B          0 B          4    CG image
  96 KB        0 B          0 B        921    __AUTH_CONST
  80 KB        0 B          0 B        931    __DATA_CONST
  64 KB        0 B          0 B          2    IOAccelerator
  64 KB        0 B          0 B          2    __TPRO_CONST
  48 KB        0 B          0 B          3    IOKit
  32 KB        0 B          0 B          1    Activity Tracing
  32 KB        0 B          0 B          2    CoreGraphics
  16 KB        0 B          0 B          1    IOAccelerator (graphics)
  16 KB        0 B          0 B          1    os_alloc_once
  16 KB        0 B          0 B          1    Foundation
    0 B      11 MB          0 B         23    mapped file
    0 B     944 KB          0 B        951    __TEXT
    0 B      80 KB          0 B          5    __LINKEDIT
    0 B        0 B          0 B          1    __FONT_DATA
    0 B        0 B          0 B          1    __INFO_FILTER
    0 B        0 B          0 B          1    __CTF
    ---        ---          ---        ---    ---
  13 MB      12 MB      1440 KB       6898    TOTAL

Auxiliary data:
    phys_footprint: 13 MB
    phys_footprint_peak: 13 MB
```

An empty winit application with the `softbuffer` surface presenting each frame.
```
======================================================================
rube-bin [39815]: 64-bit    Footprint: 55 MB (16384 bytes per page)
======================================================================

  Dirty      Clean  Reclaimable    Regions    Category
    ---        ---          ---        ---    ---
  42 MB        0 B          0 B          3    IOSurface
2624 KB        0 B        32 KB          1    MALLOC_NANO
1888 KB        0 B          0 B          7    MALLOC_SMALL
1527 KB        0 B          0 B        902    __DATA
1008 KB        0 B          0 B          7    CoreUI image data
 960 KB        0 B       720 KB          5    MALLOC_MEDIUM
 928 KB        0 B          0 B         13    MALLOC_TINY
 674 KB        0 B          0 B        335    __DATA_DIRTY
 578 KB        0 B          0 B          1    page table
 537 KB        0 B          0 B        681    __AUTH
 528 KB        0 B          0 B         27    ColorSync
 384 KB        0 B          0 B         27    untagged (VM_ALLOCATE)
 336 KB        0 B          0 B         27    MALLOC metadata
 254 KB        0 B          0 B       1980    unused dyld shared cache area
 240 KB        0 B          0 B         15    CoreAnimation
 208 KB        0 B          0 B          8    stack
 128 KB        0 B          0 B          1    Accelerate image backing stores
  96 KB        0 B          0 B          4    CG image
  96 KB        0 B          0 B        921    __AUTH_CONST
  80 KB        0 B          0 B        931    __DATA_CONST
  64 KB        0 B          0 B          2    IOAccelerator
  64 KB        0 B          0 B          2    __TPRO_CONST
  48 KB        0 B          0 B          6    IOKit
  32 KB        0 B          0 B          1    Activity Tracing
  32 KB        0 B          0 B          2    CoreGraphics
  16 KB        0 B          0 B          1    IOAccelerator (graphics)
  16 KB        0 B          0 B          1    os_alloc_once
  16 KB        0 B          0 B          1    Foundation
    0 B      11 MB          0 B         23    mapped file
    0 B     992 KB          0 B        951    __TEXT
    0 B      80 KB          0 B          5    __LINKEDIT
    0 B        0 B          0 B          1    __FONT_DATA
    0 B        0 B          0 B          1    __INFO_FILTER
    0 B        0 B          0 B          1    __CTF
    ---        ---          ---        ---    ---
  55 MB      12 MB       752 KB       6896    TOTAL

Auxiliary data:
    phys_footprint: 55 MB
    phys_footprint_peak: 55 MB
```

The full render pipeline.
```
======================================================================
rube-bin [39974]: 64-bit    Footprint: 114 MB (16384 bytes per page)
======================================================================

  Dirty      Clean  Reclaimable    Regions    Category
    ---        ---          ---        ---    ---
  58 MB        0 B          0 B          7    MALLOC_LARGE
  42 MB        0 B          0 B          3    IOSurface
3120 KB        0 B        48 KB          1    MALLOC_NANO
2288 KB        0 B          0 B          9    MALLOC_SMALL
1543 KB        0 B          0 B        902    __DATA
1008 KB        0 B          0 B          7    CoreUI image data
 976 KB        0 B          0 B         13    MALLOC_TINY
 912 KB        0 B        49 MB          5    MALLOC_MEDIUM
 691 KB        0 B          0 B          1    page table
 674 KB        0 B          0 B        335    __DATA_DIRTY
 640 KB        0 B          0 B         26    stack
 556 KB        0 B          0 B        681    __AUTH
 528 KB        0 B          0 B         27    ColorSync
 384 KB        0 B          0 B         43    untagged (VM_ALLOCATE)
 336 KB        0 B          0 B         27    MALLOC metadata
 272 KB        0 B          0 B         17    CoreAnimation
 267 KB        0 B          0 B       1980    unused dyld shared cache area
 128 KB        0 B          0 B          1    Accelerate image backing stores
  96 KB        0 B          0 B          5    CG image
  96 KB        0 B          0 B        921    __AUTH_CONST
  80 KB        0 B          0 B        931    __DATA_CONST
  64 KB        0 B          0 B          2    IOAccelerator
  64 KB        0 B          0 B          2    __TPRO_CONST
  48 KB        0 B          0 B          6    IOKit
  32 KB        0 B          0 B          1    Activity Tracing
  32 KB        0 B          0 B          2    CoreGraphics
  16 KB        0 B          0 B          1    IOAccelerator (graphics)
  16 KB        0 B          0 B          1    os_alloc_once
  16 KB        0 B          0 B          1    Foundation
    0 B        0 B        33 MB          2    MALLOC_LARGE_REUSABLE
    0 B      11 MB          0 B         24    mapped file
    0 B    1088 KB          0 B        951    __TEXT
    0 B      80 KB          0 B          5    __LINKEDIT
    0 B        0 B          0 B          1    __FONT_DATA
    0 B        0 B          0 B          1    __INFO_FILTER
    0 B        0 B          0 B          1    __CTF
    ---        ---          ---        ---    ---
 114 MB      12 MB        83 MB       6945    TOTAL

Auxiliary data:
    phys_footprint: 114 MB
    phys_footprint_peak: 114 MB
```

### Frame

| metric | value |
| :--- | :--- |
| total | 1951 |
| fps avg | 63.88 |
| min | 4.6740 ms |
| max | 41.7340 ms |
| avg | 15.6540 ms |

### Scope

| name | src_file | src_line | total_ns | total_perc | counts | mean_ns | min_ns | max_ns | std_ns |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| generate | leaf | map | rube/src/indirect.rs | 115 | 5468667906 | 17.567957 | 1950 | 2804445 | 1279791 | 6771500 | 888427.986845 |
| accumulate | samples | rube/src/indirect.rs | 183 | 855818889 | 2.749296 | 1950 | 438881 | 264875 | 898375 | 83843.948218 |
| rube::indirect::indirect_pass | rube/src/indirect.rs | 95 | 18475707465 | 59.352741 | 1950 | 9474721 | 5726792 | 15450959 | 1798029.819835 |
| rube::handle_input | rube/src/lib.rs | 54 | 202010 | 0.000649 | 1573 | 128 | 41 | 8625 | 537.564875 |
| write | pixels | rube/src/indirect.rs | 204 | 1020367584 | 3.277905 | 1950 | 523265 | 319125 | 1166625 | 129974.455224 |
| rube::update_and_render | rube/src/lib.rs | 103 | 27410053444 | 88.054100 | 1950 | 14056437 | 10406209 | 37523667 | 1914899.842273 |
| shadow | occlusion | rube/src/indirect.rs | 134 | 3385119944 | 10.874612 | 1950 | 1735958 | 527792 | 3604000 | 587259.879202 |
| generate | monte | carlo | seeds | rube/src/indirect.rs | 161 | 420846851 | 1.351960 | 1950 | 215818 | 199792 | 262541 | 7344.385398 |
| global | illumination | rube/src/indirect.rs | 168 | 6409553214 | 20.590527 | 1950 | 3286950 | 2158417 | 7757250 | 732601.411359 |
| temporal | filter | rube/src/indirect.rs | 192 | 831593554 | 2.671473 | 1950 | 426458 | 67000 | 1547750 | 169915.433073 |
| rube::march::march_pass | rube/src/march.rs | 22 | 8938122583 | 28.713492 | 1951 | 4581303 | 3045000 | 25748792 | 699649.139663 |

## Size Opt

### Memory
The full render pipeline.
```
======================================================================
rube-bin [55750]: 64-bit    Footprint: 97 MB (16384 bytes per page)
======================================================================

  Dirty      Clean  Reclaimable    Regions    Category
    ---        ---          ---        ---    ---
  42 MB        0 B          0 B          3    IOSurface
  34 MB        0 B          0 B          8    MALLOC_LARGE
8512 KB        0 B        32 MB          4    MALLOC_MEDIUM
2384 KB        0 B        32 KB          1    MALLOC_NANO
1888 KB        0 B          0 B          6    MALLOC_SMALL
1527 KB        0 B          0 B        902    __DATA
1008 KB        0 B          0 B          7    CoreUI image data
 880 KB        0 B          0 B         10    MALLOC_TINY
 674 KB        0 B          0 B        335    __DATA_DIRTY
 642 KB        0 B          0 B          1    page table
 592 KB        0 B          0 B         24    stack
 537 KB        0 B          0 B        681    __AUTH
 528 KB        0 B          0 B         27    ColorSync
 384 KB        0 B          0 B         43    untagged (VM_ALLOCATE)
 336 KB        0 B          0 B         27    MALLOC metadata
 254 KB        0 B          0 B       1980    unused dyld shared cache area
 240 KB        0 B        16 KB         16    CoreAnimation
 128 KB        0 B          0 B          1    Accelerate image backing stores
  96 KB        0 B          0 B          4    CG image
  96 KB        0 B          0 B        921    __AUTH_CONST
  80 KB        0 B          0 B        931    __DATA_CONST
  64 KB        0 B          0 B          2    IOAccelerator
  64 KB        0 B          0 B          2    __TPRO_CONST
  48 KB        0 B          0 B          5    IOKit
  32 KB        0 B          0 B          1    Activity Tracing
  32 KB        0 B          0 B          2    CoreGraphics
  16 KB        0 B          0 B          1    IOAccelerator (graphics)
  16 KB        0 B          0 B          1    os_alloc_once
  16 KB        0 B          0 B          1    Foundation
    0 B      11 MB          0 B         23    mapped file
    0 B    1072 KB          0 B        951    __TEXT
    0 B      80 KB          0 B          5    __LINKEDIT
    0 B        0 B          0 B          1    __FONT_DATA
    0 B        0 B          0 B          1    __INFO_FILTER
    0 B        0 B          0 B          1    __CTF
    ---        ---          ---        ---    ---
  97 MB      12 MB        32 MB       6931    TOTAL
```

Auxiliary data:
    phys_footprint: 97 MB
    phys_footprint_peak: 99 MB

### Frame

| metric | value |
| :--- | :--- |
| total | 1951 |
| fps avg | 79.92 |
| min | 5.5030 ms |
| max | 36.4030 ms |
| avg | 12.5120 ms |

### Scope

| name | src_file | src_line | total_ns | total_perc | counts | mean_ns | min_ns | max_ns | std_ns |
| generate | leaf | map | rube/src/indirect.rs | 96 | 465443643 | 1.861291 | 1950 | 238689 | 142958 | 717584 | 46370.855702 |
| temporal | filter | rube/src/indirect.rs | 193 | 406034514 | 1.623717 | 1950 | 208222 | 44583 | 497166 | 81551.484820 |
| rube::indirect::indirect_pass | rube/src/indirect.rs | 68 | 11735153133 | 46.928429 | 1950 | 6018027 | 3998917 | 17611833 | 901706.237438 |
| rube::handle_input | rube/src/lib.rs | 41 | 64013 | 0.000256 | 1348 | 47 | 41 | 833 | 35.648650 |
| rube::update_and_render | rube/src/lib.rs | 93 | 20531409770 | 82.104323 | 1950 | 10528928 | 8482125 | 21358208 | 956441.811630 |
| shadow | occlusion | rube/src/indirect.rs | 118 | 3445359544 | 13.777861 | 1950 | 1766851 | 519750 | 3218417 | 605004.637305 |
| global | illumination | rube/src/indirect.rs | 162 | 6355479845 | 25.415321 | 1950 | 3259220 | 2140583 | 14828250 | 770955.657429 |
| accumulate | samples | rube/src/indirect.rs | 177 | 444979158 | 1.779455 | 1950 | 228194 | 167334 | 517625 | 27175.101115 |
| write | pixels | rube/src/indirect.rs | 207 | 562900693 | 2.251018 | 1950 | 288667 | 245417 | 396459 | 24641.043746 |
| rube::march::march_pass | rube/src/march.rs | 19 | 8794870802 | 35.170352 | 1950 | 4510190 | 3085584 | 7327750 | 493101.555393 |
