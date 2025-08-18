Implemented and working for EN and JP.

Contains name and description flavor text for each item.

2 byte pointer header space before data
first 2 byte pointer header space points to first data segment
# data segment
starts with 2 pointers to start of name and description strings
name and description strings are ended with a null byte