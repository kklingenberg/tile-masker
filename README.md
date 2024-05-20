# Tile Masker

This is a narrow-purpose tool that masks colours from PNG files over an HTTP
API. The colours masked are turned transparent or changed to another opaque
colour.

This does not currently work with other file formats.

## Usage

### Serving from a volume

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

### Reverse-proxying

It's also possible to proxy files from another host, using the `--base-url` or
`-b` option instead of `-v`. For example, given a PNG-file serving host
`https://some.other/host/`:

```bash
docker run --rm -d -p 10000:10000 plotter/tile-masker -b https://some.other/host/
```

Then fetch a file from <http://localhost:10000/some/path/to/file.png>, which is
fetched by tile-masker from <https://some.other/host/some/path/to/file.png>.

### Swapping colours

It's possible to swap colours in the original image with others specified
through the `mask` parameter. Each term in the `mask` parameter takes the form:

```text
source[-[target]]
```

Where `source` is the colour in the source as an RGB hexadecimal, and `target`
is the colour that will replace it also as an RGB hexadecimal. If omitted, the
target is set to transparent black. For example, the following request:

<http://localhost:10000/some/path/to/file.png?mask=ff0000,00ff00-888888>

would replace all occurrences of bright red (`#ff0000`) in the source file with
transparent pixels, and all occurrences of bright green (`#00ff00`) with opaque
pixels coloured gray (`#888888`).
