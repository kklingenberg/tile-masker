# Tile Masker

This is a narrow-purpose tool that masks colours from PNG files over an HTTP
API. The colours masked are turned transparent.

This does not currently work with other file formats.

## Usage

Assuming there's a folder of PNG files `/tiles` (or any other):

```bash
docker run --rm -d -p 10000:10000 -v /tiles:/tiles plotter/tile-masker -v /tiles
```

Then fetch a file with optional masking:

<http://localhost:10000/some/path/within/tiles/to/file.png?mask=ff0000,00ff00>

The above query would mask the colours `#ff0000` and `#00ff00` from the original
file and produce a new PNG file with those pixels replaced with transparent
pixels. If the `mask` parameter is omitted or empty no masking is done on the
file.
