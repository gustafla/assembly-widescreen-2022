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

## 2022-07-27

I added a simple terrain grid generator using a noise function.

## 2022-07-28

I added a one shot static random tree placement. Perhaps could be made dynamic
and cull behind view and far-away trees, but I must priorize effort on visual
effects and design.

Later on during the same day, I implemented a shadow map for one light.
This will be useful to achieve my desired look with volumetric lighting.

... And I added the volumetric lighting I so desire. It looks as good as I
hoped for.

This evening is quite productive! I also added a raw image to mesh converter
to be able to display greets.

## 2022-07-29

This day was kind of exhausting. I toiled with compute shader particle systems
and shadow map filtering, but neither of those turned out like I hoped.

All in all, it's not all losses. I did get a leaf particle system thing to
run on the CPU. It'll have to do for this release.

## 2022-08-01

I added more controls via rocket and changed my lighting model to the proven,
good old Blinn-Phong. The "borrowed" Oren-Nayar code produces NaNs or infs
which propagate through my lighting pass and I don't have time to debug it.

Added fog as well, to hide distant artifacts.

Tomorrow I will fill a rough timeline and then it needs tweaking and
raymarched surfaces I want to have must be implemented too.

# 2022-08-02

I've implemented the raymarched surfaces I wanted, and have filled about
half of the timeline so far. I wasted a bunch of time trying to get raymarched
soft shadows to work, it was an unnecessary detour with no payoff. For the next
release I will probably just have more shadow maps and a separate pass for
raymarched surfaces which I will render for each light.

Tomorrow I need to finish this thing and hope that it runs without problems.
Then the day after that I can apply final polish and submit it to Assembly.

# 2022-08-03

Directing cameras, fixing an SDF bug and laying out greets took me
an entire day of work, but I'm happy to say that the demo is now essentially
ready to be shown in the widescreen compo.

Tomorrow I'm planning to make final adjustments and submit the demo.
Also capturing it on video could be useful for taking screenshots etc.

I may even retry adding soft shadows because the SDF should be fixed now.
