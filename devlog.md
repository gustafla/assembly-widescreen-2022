## 2022-07-18

I thought about vertex inputs, texture mapping and how to draw multiple objects
per pass.

## 2022-07-19

I decided to not have any texture mapping for first release. I implemented
an instanced multi model draw loop, but the transformations are not working
as they should. Tomorrow I will compare with my previous wgpu repo and
hope to find the bug.
