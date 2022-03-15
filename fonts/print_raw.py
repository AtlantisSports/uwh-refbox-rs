from array import array
import sys
import re

path = sys.argv[1]

regex = re.compile('font_([0-9]+)x([0-9]+).raw')
match = regex.match(path.split('/')[-1])

if match is None:
    print("Filename invalid")
    exit()

width = int(match.group(1))
height = int(match.group(2))

data = array('B')

with open(path, "rb") as f:
    data.fromfile(f, width * height * 6)

data_string = ""

for b in data:
    data_string += "{:08b}".format(b)

print_string = ""

for i in range(8):
    print_string += '+'
    for j in range(width):
        print_string += '-'
print_string += '+\n'

for i in range(0, len(data_string), width * height * 8):
    block = data_string[i:i+(width * height * 8)]
    for j in range(0, len(block), width * 8):
        row = block[j:j+(width * 8)]
        print_string += '|'
        for k in range(0, len(row), width):
            print_string += row[k:k+width].replace('0', ' ').replace('1', '#')
            print_string += '|'
        print_string += '\n'
    for i in range(8):
        print_string += '+'
        for j in range(width):
            print_string += '-'
    print_string += '+\n'

print(print_string)
