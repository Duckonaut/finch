# finch
Compile a directory into a C header (or a header + C file) to access it as global data.

## Usage
```
Usage: finch [OPTIONS] <DIRECTORY> [OUTPUT]

Arguments:
  <DIRECTORY>  The directory to compile.
  [OUTPUT]     The output file name.

Options:
  -c, --c-file           Generate a C file as well.
  -p, --prefix <PREFIX>  The prefix to add to the output struct.
  -h, --help             Print help
```
