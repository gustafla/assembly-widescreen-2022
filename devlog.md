## 2022-07-18

I thought about vertex inputs, texture mapping and how to draw multiple objects
per pass.

## 2022-07-19

I decided to not have any texture mapping for first release. I implemented
an instanced multi model draw loop, but the transformations are not working
as they should. Tomorrow I will compare with my previous wgpu repo and
hope to find the bug.

## 2022-07-20

The previous day's issue turned out to be a problem with triangle face
orientation. The root cause was that I was thinking about the Z-axis the
wrong way, so my triangle generation code was wrong. Apart from that,
the yesterday's instanced draws work and I added 3 more full screen
render passes to lay the groundwork for SSAO and bloom.

Tomorrow I need to write the SSAO and bloom implementations in WGSL.
