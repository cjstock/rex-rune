# Theme source samples

Inputs for `src/theme_source/tests.rs`, which derives a chrome palette from each and checks it
with `theme_source::legible` against the palette the reference derivation (`derive.py` on the
throwaway `prototype/terminal-chrome` branch) printed for it.

| File | What it covers |
| --- | --- |
| `dracula` | dark, nothing adjusted |
| `nord` | dark; danger lightened (step 1) and ink lifted (step 4) |
| `aura` | warning on danger's hue (step 2) |
| `x-berlin` | monochrome: canonical amber and red (step 3) |
| `fairyfloss`, `hot-dog-stand` | mid-tone: the ground steps toward black (step 5) |
| `great-pumpkin` | light mid-tone: the ground steps toward white (step 5) |
| `catppuccin-latte`, `github-light-default` | light |
| `matugen-no-dank16-{dark,light}` | plain matugen without dank16: chrome keyed, no status keys |
| `matugen-error-only-{dark,light}` | the same with M3 `error` as `chrome-danger` |

## Licences

The Ghostty themes (`aura`, `catppuccin-latte`, `dracula`, `fairyfloss`,
`github-light-default`, `great-pumpkin`, `hot-dog-stand`, `nord`, `x-berlin`) are copied
unchanged from Ghostty's bundled themes, which come from the
[iTerm2-Color-Schemes](https://github.com/mbadolato/iTerm2-Color-Schemes) collection under the
MIT License below. As that licence notes, each individual theme's copyright belongs to its
author.

```
MIT License

Copyright (c) 2011 to Present Mark Badolato

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

This license covers the iTerm-Color-Schemes repository collection of themes.

The copyright/license for each individual theme belongs to the author of that theme.
```

The four `matugen-*` files were written for this project from one desktop's matugen output (no
third-party content).
