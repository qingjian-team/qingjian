---
title: Custom themes
order: 3
description: "Making your own candidate-window theme: where to put it, changing colours on top of a built-in theme, and how to write gradients and images, shadows, sizes and weights, fonts, text outlines, decorations, input status, the floating status bar, transitions and looping animations, and when changes take effect."
---

A theme decides how the candidate window looks. A theme looks the same on macOS and Windows, and only applies when 设置 → 外观 → 渲染引擎 (Settings → Appearance → Rendering engine) is set to 青简渲染器 (Qingjian renderer).
For choosing a built-in theme, see [Appearance and themes](../personalize/appearance.md#Themes); this page is for people who want to make their own. Theme files are written in JSON.

## Where to put it

Custom themes go in the `themes` folder of the [data folder](data-files.md#Data-files), one folder per theme, each containing a `theme.json`:

```text
themes/
└── pink/
    └── theme.json
```

The simplest approach is to start from a built-in theme and change a few colours. Here is a theme with a pink highlight:

```json
{
  "extends": "qingjian",
  "schema": 1,
  "meta": { "id": "pink", "name": "Pink" },
  "variables": {
    "accent": { "light": "#ffb7d5", "dark": "#b0507a" },
    "hl_text": "#ffffff"
  }
}
```

- `theme.json` can contain comments (`//` to the end of the line, or `/* … */`), and a trailing comma after the last item of an object or list is fine.
- `extends` names the base theme: `qingjian` (青简绿), `system-blue` (系统蓝), `wechat` (微信绿), `sakura` (樱花), `dafeiyu` (大肥鱼), `miku` (初音未来) or `bocchi` (波奇酱). Anything you don't write is taken from the base theme, and the base theme's images don't need to be copied into your folder. Without `extends`, 青简绿 is the base.
- `meta.id` must be the same as the folder name, can't be the same as a built-in theme, and can't be `system`, `light` or `dark`; `meta.name` is the name shown in the theme list.
- Colours are written as `#rrggbb`, or `#rrggbbaa` with opacity; written as `{ "light": …, "dark": … }`, the light and dark appearances each use one.

Colours you can change:

| Name | Used for |
|---|---|
| `surface` | Window background |
| `accent` | Background of the highlighted candidate |
| `text` | Candidate words |
| `gloss` | Glosses, the pinyin line |
| `faint` | Numbers, part of speech, page number |
| `fresh` | Glosses of new words |
| `cloud` | Cloud candidates and the cloud icon |
| `hl_text`, `hl_gloss`, `hl_faint`, `hl_fresh`, `hl_cloud` | The matching colours within the highlighted row |

## Gradients and images

Besides solid colours, parts such as the window and the highlight can be filled with gradients or images; images go in the theme folder, in PNG or SVG. For example, to replace the window background with a stretchable frame image:

```json
"windows": {
  "vertical": {
    "fill": { "image": "images/frame.png", "slice": [16, 16, 16, 16], "scale": 2 }
  }
}
```

- `slice` is the width of the image's four edges (top, right, bottom, left) that aren't stretched, in image pixels: when the window grows, the corners keep their shape and only the middle stretches. Without it, the whole image stretches.
- `scale` says what resolution the image is; write 2 for an image drawn at 2× size.
- SVG images are sharp at any scale and need no `scale`. Their width and height are the displayed size (in points), and `slice` uses the same unit; draw them at their actual size, since oversized drawings use more memory.
- Text in SVG isn't displayed, so convert it to outlines before exporting; bitmaps embedded in an SVG and external files it references aren't displayed either, so use separate PNGs for bitmaps.
- Gradients are written as `{ "linear": 90, "stops": ["#ffb7d5", "#ffffff"] }` (an angle of 90 runs left to right).
- If the background is a light image, text becomes hard to read in the dark appearance; add `"appearance": "light"` to `meta` to keep the theme light at all times.

## Shadows

Parts such as the window and the highlight can have drop shadows (`drop-shadow`) or inner shadows (`inner-shadow`), written in `effects`; several layers can be stacked. For example, to make the window shadow lighter and give the highlight bar a pink drop shadow:

```json
"windows": {
  "vertical": {
    "effects": [{ "type": "drop-shadow", "y": 4, "blur": 10, "color": "#00000030" }]
  }
}
```

- `x` and `y` are offsets (positive is right and down), `blur` is how blurred it is, and `spread` expands the shadow outward (shrinks it inward for inner shadows), all in points; 0 if omitted.
- Shadows follow the shape: a rounded window has a rounded shadow, a border image casts a shadow along its transparent edges, and text can have a drop shadow too.
- The window shadow is the window's own drop shadow; write `"effects": []` to remove it.

## Sizes and weights

`text.styles` has three kinds of text: `candidate` (candidate words), `annotation` (glosses and the pinyin line) and `index` (numbers and the page number). Each can change its size `size`, line height `line_height` and weight `weight`:

```json
"text": {
  "styles": {
    "candidate": { "size": 18, "line_height": 22, "weight": "medium" }
  }
}
```

Weights are `light`, `regular`, `medium`, `semibold`, `bold`, or a number from 100 to 900. If the system font doesn't have a given weight, the nearest one is used.

When 候选字号 (Candidate size) or 译文字号 (Gloss size) is changed in Settings, it overrides the theme's `candidate` and `annotation` sizes, and line heights scale in proportion; `index`, `preedit` and table row heights scale along with the candidate size so that rows line up. Cells in a table row are aligned automatically on the text baseline, so themes don't need to adjust margins for different sizes. The floating status bar scales as a whole in proportion to the candidate size, which themes don't need to handle. Other styles you name yourself in the theme aren't affected.

## Fonts and italics

Each kind of text can use a different font, written as `family`; italics are `"style": "italic"`. A `family` written under `text` is used by all text that doesn't set its own font:

```json
"text": {
  "family": ["PingFang SC", "system"],
  "styles": {
    "candidate": { "size": 18, "line_height": 22, "family": ["Songti SC", "STSong", "system"] },
    "annotation": { "size": 12, "line_height": 15, "family": "Georgia", "style": "italic" }
  }
}
```

- `family` is one font name, or several in order, of which the first installed one is used. `system` means the font chosen under 设置 → 外观 → 字体 (Settings → Appearance → Font) (the system font if none is chosen); it's also used when none of the fonts are installed.
- Write the font family name, the English name shown in the system's font list, such as `Songti SC`, `Microsoft YaHei` or `Georgia`. macOS and Windows have different fonts, so list names for both if it should look good on both.
- Characters a font doesn't have (such as Chinese characters in an English font) are filled in from the system font automatically.
- If a font has no italic, the regular style is slanted instead; that's also how italics for Chinese characters are produced.

A theme can also bring its own fonts: put them in the theme folder, list them at the top level of `theme.json`, and set `family` to the font's own family name:

```json
"fonts": [{ "file": "fonts/LXGWWenKai-Regular.ttf" }],
"text": { "styles": { "candidate": { "size": 18, "line_height": 22, "family": "LXGW WenKai" } } }
```

TTF, OTF and TTC are supported, up to 32 MB per file. Only bundle fonts whose license allows redistribution (such as LXGW WenKai under the OFL); for fonts that come with the system, just write the name and don't copy them into the theme.

## Text outlines

Candidate words, glosses and the pinyin line can have an outline, written on the text in question: `"stroke": { "width": 1.5, "color": "#c2386b" }`, with `width` in points.
The outline widens outward from the glyphs and doesn't change where the text sits. The colour can depend on whether the row is highlighted, for example to outline only the highlighted candidate:

```json
"stroke": { "width": 1.5, "color": { "if": "highlighted", "then": "#c2386b", "else": "#00000000" } }
```

Text shadows and glows use the shadow syntax: add `effects` to the text; for example `[{ "type": "drop-shadow", "blur": 4, "color": "#e85a8c" }]` gives a pink glow.
Colour emoji aren't outlined.

## Decorations that stick out of the window

A decoration image can poke out past the edge of the candidate window (such as a little character lying on a corner of the window). Add an absolutely positioned frame to the window's `children`; negative `inset` values (distances from the top, right, bottom and left edges of the window) extend it outside:

```json
{ "type": "frame", "position": "absolute", "inset": [-24, -24, "auto", "auto"], "width": 48, "height": 48,
  "fill": { "image": "images/cat.png", "scale": 2 } }
```

The example sticks out 24 points at the top right. The candidate window is still positioned at the cursor by the window itself, so decorations don't affect its position; near the edge of the screen, a decoration may end up off-screen. Decorations don't block the mouse.

## Separating part of speech and meaning

By default the gloss is laid out on one line (`n. pure letters · …`). To put the part of speech and the meaning on separate lines, write two annotation nodes in the candidate, using `tones` to choose which kinds of content each one draws and `senses` to draw only the first few senses:

```json
{ "type": "annotation", "bind": "annotation", "font": "annotation", "tones": ["pos"], "senses": 1, "height": 15,
  "gloss": "@gloss", "fresh": "@fresh", "faint": "@faint" },
{ "type": "annotation", "bind": "annotation", "font": "annotation", "tones": ["gloss", "fresh"], "senses": 1,
  "gloss": "@gloss", "fresh": "@fresh", "faint": "@faint" }
```

- `tones` can be: `pos` (part of speech), `gloss` (meaning), `fresh` (meaning of a new word), `separator` (the ` · ` between senses), `faint` (Japanese kana readings), `code` (auxiliary code). Without it, everything is drawn.
- Some candidates have no part of speech (such as emoji); give the part-of-speech line a `height` so it keeps its space even when empty and the columns line up.
- The colours of the part of speech and separator can be set separately with `pos` and `separator`; otherwise `faint` is used.

## Showing input status

The candidate window can show whether you're in Chinese or English, Simplified or Traditional, full-width or half-width punctuation, and the name of the input scheme. The theme decides what text to show:
use `when` so that only one of two texts appears, and `"bind": "mode.scheme"` for the scheme name:

```json
{ "type": "text", "text": "中", "when": "!mode.english", "font": "index", "color": "@gloss" },
{ "type": "text", "text": "英", "when": "mode.english", "font": "index", "color": "@gloss" },
{ "type": "text", "text": "繁", "when": "mode.traditional", "font": "index", "color": "@gloss" },
{ "type": "text", "bind": "mode.scheme", "font": "index", "color": "@faint" }
```

- `mode.english`: English mode; `mode.traditional`: Traditional output; `mode.full_width`: full-width punctuation. A leading `!` means "not".
- `mode.scheme` is the scheme name, such as 全拼, 小鹤双拼 or 五笔（86） + 全拼; in `when` it means "there is a scheme name".
- These nodes go in the window's `children`; `children` has to be written out in full (writing only the new nodes replaces the original contents entirely), so copy one from a built-in theme's `theme.json` and edit it.

## Floating status bar (Windows)

The floating status bar at the bottom right of the screen on Windows can also be drawn by the theme, written the same way as the candidate window: write `root` under `status`, and use `repeat` bound to `cells` to lay out the cells one by one (currently four: Chinese/English, punctuation, symbol panel and the settings gear; clicking a cell toggles that item):

```json
"components": {
  "status-cell": { "type": "frame", "padding": [4, 8], "children": [
    { "type": "text", "bind": "text", "font": "candidate",
      "color": { "if": "emphasized", "then": "@cloud", "else": "@gloss" } },
    { "type": "icon", "icon": "gear", "when": "gear", "size": 15, "color": "@gloss" }
  ] }
},
"status": {
  "root": { "type": "frame", "fill": "@surface", "radius": 8, "children": [
    { "type": "frame", "width": 24, "height": 24, "fill": { "image": "images/sakura.svg" } },
    { "type": "repeat", "bind": "cells", "component": "status-cell" }
  ] }
}
```

- Available in each cell: `text` (the cell's text), `emphasized` (the active mode, or full-width punctuation), `gear` (the settings gear cell), `first` / `last`.
- The whole status bar can also use `mode.english` and the others from "Showing input status" to decide what to draw.
- Decorations can sit between cells; clicking a decoration counts as the cell before it.
- Without `root`, the status bar is drawn as usual, using only the colours and sizes in `status`. macOS has no floating status bar.

## Transition animation

When the highlight moves to another candidate, the highlight bar can slide over instead of jumping (all three built-in themes do this). Give the highlight bar a name (`id`) and add `transition`:

```json
{ "type": "frame", "id": "highlight", "transition": { "duration": 120, "easing": "ease-out" },
  "when": "highlighted", "fill": "@accent", "radius": 4 }
```

- Parts with the same name in two consecutive displays transition from their old position, size and opacity to the new ones; colours don't transition.
- `duration` is the length in milliseconds; `easing` is the speed curve: `linear`, `ease`, `ease-in` (slow, then fast), `ease-out` (fast, then slow), `ease-in-out`,
  or four numbers `[x1, y1, x2, y2]` (like CSS `cubic-bezier`).
- Animations don't play when 过渡动画 (Transition animation) is off in Settings, or when the system's reduce-motion setting is on (macOS: System Settings → Accessibility → Display; Windows: Settings → Accessibility → Visual effects → Animation effects off).

## Looping animation

Decorations can keep moving gently, such as drifting petals or a breathing glow. Add `animation` to a node and describe one cycle with keyframes:

```json
"animation": {
  "duration": 2400, "easing": "ease-in-out",
  "keyframes": [
    { "at": 0, "y": 0, "rotate": 0 },
    { "at": 0.5, "y": 3, "rotate": 20 },
    { "at": 1, "y": 0, "rotate": 0 }
  ]
}
```

- `duration` is the length of one cycle in milliseconds, and `at` is the keyframe's position within the cycle (0 to 1).
- What can move: `x`, `y` (translation, in points), `rotate` (rotation angle, around its own centre), `scale` and `opacity`. Anything not written in a keyframe stays put.
- What moves is that part together with everything inside it, without pushing other parts around.
- It loops forever by default; `"loop": false` plays it once and stops on the last frame.
- It starts from the beginning when the candidate window appears and doesn't restart while you type; it doesn't move when 过渡动画 (Transition animation) is off or reduce motion is on.

## When changes take effect

- A newly added theme appears among the theme cards after you reopen Settings.
- Changes to the theme in use: after saving `theme.json`, they take effect within about a second while you're typing, so you can see your edits as you go.
- If a theme file has mistakes, that theme doesn't appear in the list; if the theme in use can't be read, the candidate window falls back to 青简绿. The reason is recorded in the runtime log (see [Runtime logs](data-files.md#Runtime-logs)).
