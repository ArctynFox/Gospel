Implemented.

a book's data segment is 12 bytes long and starts sequentially from the first byte of the file
file ends in 12 bytes of FF
# data segment
2 bytes refer to item id
2 bytes null
2 bytes to refer to the file number within the DT archive (For FC, t_book01 is 17 00, t_book02 is 18 00, so on... See json file created by Factoria)
2 bytes to refer to the DT archive number (For FC, 02 00)
2 bytes to denote the book id of this book within the file referred to above
2 bytes null