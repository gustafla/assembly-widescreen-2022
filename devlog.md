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

## 2022-07-21

I've written bloom and SSAO implementation based on learnopengl.org
However, it does not work as expected yet. It might be that I have
transformations wrong, a mistake in the shaders, or that my buffer's
precision is not sufficient.

## 2022-07-22

I'm pretty sure that the SSAO issue was caused by lack of depth precision.
I decided to remove it entirely from current iteration, as I don't yet
have a solid lighting model and anything resembing my intended look.

I refactored the pass system for readability and vram savings.

Next I need to make my scene/models good, and implement a non-placeholder
lighting system.

## 2022-07-24

I had a few hours to work on the light infrastructure and I also got to remove
one of the shader source files by introducing control parameters to bloom

## 2022-07-25

I hastily... borrowed, some Oren-Nayar lighting code as I'm aiming to just get
results and not to learn and struggle a week before deadline. It's apparently
not even correct, but it looks fine for my purposes. I also got to implementing
a better gaussian blur kernel for bloom (kernel.py).

Next time I need to work on the models/meshes a bit more. They don't have to
be great, but at least a little bit more like real trees.

Other tasks are lights, color, camera, timeline, fog, sky, raymarched
surfaces and animations, and maybe particle effects if I have extra time.

## 2022-07-26

I implemented a quite allright looking low-poly tree generator. Terrain grid
still missing but I'm very satisfied with today's progress.
