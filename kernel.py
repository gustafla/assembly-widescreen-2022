import math
import sys

# https://www.rastergrid.com/blog/2010/09/efficient-gaussian-blur-with-linear-sampling/
def gaussian(x, c):
    return (1 / math.sqrt(2 * math.pi * c * c)) * math.exp(-(x*x)/(2*c*c))

width = int(sys.argv[1])
width2 = int(sys.argv[2])

kernel = [gaussian(x, width2) for x in range(0, width)]

print(f"array<f32, {width}>(", end="")
for x in kernel:
    print(f"{x:.6f}, ", end="");
print(")")
