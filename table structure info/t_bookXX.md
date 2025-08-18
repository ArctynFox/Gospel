Currently cannot decode JP files.

2 byte pointer header space before data
first 2 byte pointer header space points to first book's data segment
second 2 bytes refer to the start of that book's content text
repeats until address from first 2 byte pointer

# data segment
starts with book title string
text (the entire book contents are one string)
strings are ended with a null byte
file can end without a null byte to end string (FC DT02 book07)

# content string
01 denotes new line
02 denotes wait for input
03 denotes new page
07 XX to denote color change, where XX is the new color value
	09 is dark gray
	10 is black
23 XX ... 78 23 YY ... 79 denotes image at (X, Y); ... denotes variable length. these should be read as a String and parsed to u32
23 NN ... 46 denotes the image to use (persists)
23 46 clears image
23 NN 53 denotes a size change, this can happen in-line so it needs to be handled in the string