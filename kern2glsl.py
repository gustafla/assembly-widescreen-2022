import sys

nums = list()
for line in sys.stdin:
    nums += [float(x) for x in line.split()]

print("#define KERNEL_SIZE " + str(len(nums)))
print("float KERNEL[KERNEL_SIZE];")
for i in range(0, len(nums)):
    print("KERNEL[" + str(i) + "] = " + str(nums[i]) + ";")
