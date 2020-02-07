import numpy as np
import sys

if len(sys.argv) != 3:
    print("Usage: convert_to_raw.py <INPUT_FILE> <OUTPUT_FILE>")
    exit(1)

binary_text = list(open(sys.argv[1]).read())

bools = np.empty(len(binary_text), dtype=bool)
i = 0
for val in binary_text:
    if val == '1':
        bools[i] = True
        i += 1
    elif val == '0':
        bools[i] = False
        i += 1
    elif val == ' ':
        continue
    elif val == '\n':
        continue
    else:
        raise ValueError("Invalid input character: {}".format(val))

byte_arr = np.packbits(bools[:(i-1)])

with open(sys.argv[2], 'wb') as f:
    f.write(byte_arr)
