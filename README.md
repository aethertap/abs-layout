# Use the `books.json` metadata to make Audio Book Shelf dirs from OpenAudible

That's what this does. Run it like this:

```bash
abs-layout my-download-dir -o my-audiobookshelf-dir
```

where:
- `my-download-dir` is the path to the directory that has `books.json` from OpenAudible
- `my-audiobookshelf-dir` is the path where you want the stuff to land.

This program doesn't actually *do* anything - it just prints a shell script to
stdout that you can dump into a `.sh` file, ***read first*** then use to
completely screw everything up because you didn't read it first.

Buf if you *did* read it first and it looks good, you can run it to make
a directory tree of symlinks to your OpenAudible converted books that
AudioBookShelf can probably read. 

I haven't tried that last part yet, so maybe.

good luck.


