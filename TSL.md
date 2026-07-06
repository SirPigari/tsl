# TSL - Text Shader Language

TSL is a small shading language for coloring and transforming terminal text.
You write a `main` function that runs once per character in your input text,
and it returns a color, a character, or both. The compiler (`tslc`) turns
`.tsl` source into `.ctsl` bytecode, and the renderer applies it to a text
file to produce ANSI-colored output.

---

## Hello World

Here is a rainbow - every character gets colored by its horizontal position:

```tsl
fn main(t, i, len, x) -> color
  return hsl(x, 1.0, 0.55)
end
```

Compile and run it:

```
tslc rainbow.tsl               # produces rainbow.ctsl
tslc render rainbow.ctsl input.txt output.txt
```

The output file contains ANSI escape sequences. View it in a terminal with `cat output.txt`.
Input can be anything, for testing i recommend you to use just a block of `#`.

---

## How it works

For each character in the input text, TSL calls your `main` function and uses
the return value to decide what to write to the output:

- Return a `color` -> the character is recolored with that foreground color.
- Return a `char` -> the character is replaced; original color is kept.
- Return `(char, color)` -> replace character and set foreground color.
- Return `(color, color)` -> set foreground and background colors.
- Return `(char, color, color)` -> replace character, set fg and bg.

---

## Implicit parameters

`main` always has these parameters available, in order. You can name them
whatever you want; the position is what matters. Parameters you don't need
can be left off the end.

| Position | Conventional name | Description                                      |
|----------|-------------------|--------------------------------------------------|
| 0        | `t`               | Character's position in input, 0.0 to 1.0        |
| 1        | `i`               | Character index (0-based, float)                 |
| 2        | `len`             | Total number of characters                       |
| 3        | `x`               | Column position, 0.0 to 1.0 within the line      |
| 4        | `y`               | Row position, 0.0 to 1.0 within the text         |
| 5        | `col_i`           | Column index (pixels/chars, float)               |
| 6        | `row_i`           | Row index (pixels/chars, float)                  |
| 7        | `c`               | Current character as a Unicode codepoint (float) |
| 8        | `original`        | Original foreground color (a `color` value)      |

You can also call these as zero-argument functions from any function, not just
`main`: `t()`, `i()`, `len()`, `x()`, `y()`, `col_i()`, `row_i()`,
`char_code()` (same as `c`), `original()`, `time()`, `seed()`.

---

## Types

TSL has these types:

- `float` (also `number`) - 32-bit float. Default type when no type is specified.
- `int`  - integer, stored as float internally (lol).
- `bool` - boolean.
- `char` - a character, stored as its Unicode codepoint.
- `color` - RGB color, each channel 0.0 to 1.0.
- `vec2`, `vec3`, `vec4` - float vectors.
- `matrix2`, `matrix3`, `matrix4` - float matrices.
- Arrays - written as `float[]`, `color[]`, `vec3[]`, etc.

All parameter types default to `float` if no type is given.

---

## Syntax

### Functions

```tsl
fn name(params) -> return_type
  body
end
```

The return type is optional for helper functions. The entry point is the
function named `main`.

```tsl
fn lerp(float a, float b, float t) -> float
  return a + (b - a) * t
end

fn main(t, x) -> color
  let v = lerp(0.3, 1.0, x)
  return gray(v)
end
```

### Variables

```tsl
let x = 1.0
x = x + 1.0
```

All variables are local. There are no globals.

### Control flow

```tsl
if condition do
  ...
end

if condition do
  ...
else
  ...
end

while condition do
  ...
end

for k in 0..10 do
  ...
end
```

`break` and `continue` work inside loops.

The `do` is required. The `in` keyword is used only in `for`. Range is
`start..end` (start inclusive, end exclusive), both must be scalar.

### Ternary

```tsl
let v = condition ? expr_if_true : expr_if_false
```

### Operators

```
+  -  *  /  %
==  !=  <  >  <=  >=
&&  ||  !
```

Arithmetic works component-wise on vectors, matrices, and colors. You can
multiply a matrix by a vector or a scalar by anything.

### Character literals

Single characters can be written as `'A'` and are equal to their codepoint as
a float. Escape sequences `\n`, `\r`, `\t`, `\0`, `\\`, `\'` work inside them.

```tsl
if c == 'A' do
  return rgb(255, 200, 50)
end
```

### Hex literals

```tsl
let v = 0xFF
```

### Comments

```tsl
# this is a comment
// this is also a comment
```

### Includes

```tsl
include "utils.tsl"
```

Included files are only expanded once even if referenced multiple times.
Paths are relative to the file doing the including.

---

## Extern variables

Externs are parameters passed in from the outside at render time. They let
you configure shaders without recompiling.

```tsl
extern float gain = 0.8
extern color tint = rgb(120, 200, 255)
extern bool enabled = true
extern char symbol
```

The default value after `=` is used if the caller does not supply one.
Externs without a default are required.

Tuple externs pass a character+color combo or a two-color pair:

```tsl
extern (char, color) accent = ('*', rgb(255, 90, 30))
extern (color, color) scheme = (rgb(255, 0, 0), rgb(0, 0, 30))
extern (char, color, color) glyph = ('X', rgb(255, 200, 0), rgb(10, 10, 30))
```

From the command line you pass externs with `--extern name=value`:

```
tslc render shader.ctsl input.txt out.txt --extern gain=0.5 --extern tint=#ff8040
```

Accepted value formats: float (`1.5`), bool (`true`/`false`), hex color
(`#rrggbb` or `#rgb`), char literal (`'A'` or `"A"`), or a parenthesized
tuple (`('X', #ff0000, #000020)`).

---

## Parameter qualifiers

Function parameters can be qualified with `in`, `out`, `inout`, or `const`.
These affect whether the caller's variable is written back after the call.

- `in` / `const` - read-only inside the function. Default.
- `out` - written by the function; caller's variable receives the result.
- `inout` - read and written; the caller's variable is updated.

The argument for an `out` or `inout` parameter must be a plain local variable,
not an expression.

```tsl
fn tone(inout float v, out float mag, const float k) -> char
  v = v * k
  mag = abs(v)
  return mix(48, 57, clamp(mag, 0.0, 1.0))
end

fn main(t) -> char
  let a = sin(t * 6.283)
  let m = 0.0
  return tone(a, m, 0.75)
end
```

---

## Arrays

```tsl
let palette = [255, 128, 64, 32]
let v = palette[i % 4]
palette[0] = 99
```

Array indices are clamped to the valid range - no out-of-bounds crash.
Arrays can also be declared as parameter types:

```tsl
fn pick(float[] values, float idx) -> char
  return values[idx]
end
```

---

## Vectors and matrices

```tsl
let n = normalize(vec3(x - 0.5, y - 0.5, 0.35))
let l = vec3(0.35, 0.6, 0.72)
let d = dot(n, l)
```

Vector components can be accessed by swizzle:

```tsl
let xy = n.xy         // vec2
let z  = n.z          // scalar
let zyx = n.zyx       // vec3 in reverse order
```

Valid component names: `x y z w` (or equivalently `r g b a` or `s t p q`).
Up to 4 components can be swizzled at once.

Matrix types: `matrix2`, `matrix3`, `matrix4`. Constructed in row-major order:

```tsl
let m = matrix3(
  1.0, 0.0, 0.0,
  0.0, 0.0, -1.0,
  0.0, 1.0,  0.0
)
let v2 = m * vec3(1.0, 0.0, 0.0)
```

Matrix-matrix multiplication does proper matrix math. Matrix * vector is
a matrix-vector product. Anything * scalar scales every element.

---

## Color operations

Colors are RGB, each channel in 0.0..1.0. Arithmetic on colors is
component-wise.

```tsl
let c = rgb(255, 128, 0)        // from 0-255 values
let c = rgba(255, 128, 0, 128)  // with alpha premultiplied
let c = hsl(hue, sat, lit)      // all in 0.0..1.0
let c = hsv(hue, sat, val)      // all in 0.0..1.0
let c = gray(0.5)               // gray(v) = rgb(v, v, v) in 0.0..1.0

let ch = mixc(a, b, 0.5)       // interpolate two colors
```

Channel accessors (return a scalar):

```tsl
let r = c.r
let g = c.g
let b = c.b
```

---

## Built-in functions

### Math

| Function                | Description                                                  |
|-------------------------|--------------------------------------------------------------|
| `abs(x)`                | Absolute value                                               |
| `sign(x)`               | -1, 0, or 1                                                  |
| `floor(x)`              | Round down                                                   |
| `ceil(x)`               | Round up                                                     |
| `round(x)`              | Round to nearest                                             |
| `fract(x)`              | Fractional part                                              |
| `sqrt(x)`               | Square root                                                  |
| `pow(x, y)`             | x to the power y                                             |
| `exp(x)`                | e^x                                                          |
| `log(x)`                | Natural log                                                  |
| `log2(x)`               | Log base 2                                                   |
| `sin(x)`                | Sine (radians)                                               |
| `cos(x)`                | Cosine (radians)                                             |
| `tan(x)`                | Tangent (radians)                                            |
| `asin(x)`               | Arc sine                                                     |
| `acos(x)`               | Arc cosine                                                   |
| `atan(x)`               | Arc tangent                                                  |
| `atan2(y, x)`           | Two-argument arc tangent                                     |
| `min(a, b)`             | Minimum                                                      |
| `max(a, b)`             | Maximum                                                      |
| `clamp(x, lo, hi)`      | Clamp x between lo and hi                                    |
| `mix(a, b, t)`          | Linear interpolate: a + (b-a)*t. Works on scalars or colors. |
| `step(edge, x)`         | 0 if x < edge, else 1                                        |
| `smoothstep(e0, e1, x)` | Smooth Hermite between e0 and e1                             |

### Vector / geometry

| Function             | Description                                                 |
|----------------------|-------------------------------------------------------------|
| `length(a, b)`       | Distance between two vectors (or sqrt(a^2+b^2) for scalars) |
| `dot(a, b)`          | Dot product                                                 |
| `cross(a, b)`        | Cross product (vec3 only)                                   |
| `normalize(v)`       | Unit vector                                                 |
| `reflect(i, n)`      | Reflect incident vector i around normal n                   |
| `refract(i, n, eta)` | Refract incident vector                                     |

### Color constructors

`rgb(r, g, b)`, `rgba(r, g, b, a)`, `hsl(h, s, l)`, `hsv(h, s, v)`,
`gray(v)`, `mixc(a, b, t)`

### Character tests

| Function      | Description             |
|---------------|-------------------------|
| `is_space(c)` | True if c is whitespace |
| `is_digit(c)` | True if c is 0-9        |
| `is_alpha(c)` | True if c is a-z or A-Z |
| `is_upper(c)` | True if c is A-Z        |
| `is_lower(c)` | True if c is a-z        |

These return 1.0 for true and 0.0 for false.

### Randomness

| Function       | Description                                       |
|----------------|---------------------------------------------------|
| `rand()`       | Random float in 0.0..1.0, changes each call       |
| `rand(seed)`   | Deterministic hash of seed, same result per frame |
| `rand(lo, hi)` | Random float between lo and hi                    |

The seed is per-character and based on the render seed and time, so `rand()`
gives different values per character per frame.

### Time and context

| Function                  | Description                         |
|---------------------------|-------------------------------------|
| `time()`                  | Seconds since program start (float) |
| `seed()`                  | Render seed as float                |
| `t()`, `i()`, `x()`, etc. | Same as the implicit parameters     |

---

## Output modes

When rendering, you pick how color is encoded in the output:

- `ansi24` - True color (24-bit RGB ANSI). Default.
- `ansi8` - 256-color ANSI.
- `ascii` - No color codes, plain text.

```
tslc render --mode ansi8 shader.ctsl input.txt output.txt
```

---

## tslc command reference

```
tslc <input.tsl> [output.ctsl]       Compile a shader
tslc -a <input.tsl>                  Compile and print the AST
tslc bc <input.ctsl>                 Disassemble bytecode
tslc render [options] <shader> <input.txt> <output.txt>
```

Render options:

```
--time <seconds>    Set the time parameter (default 0)
--mode <mode>       ansi24, ansi8, ascii (default ansi24)
--charset <set>     ascii, unicode
--seed <n>          Set the random seed
--extern <n=v>      Pass an extern value (repeatable)
```

If you give `tslc` a `.ctsl` file as the first positional arg, it will figure
out what you want automatically (disassemble if that's all you give it, or
render if you give it three positional args).

---

## Limits

The VM enforces these to prevent runaway shaders:

- 200,000 instructions per character.
- Call depth: 32.
- Stack depth: 256.
- Locals per function: 64.

---

## More examples

### Syntax highlighting by character class

```tsl
fn main(t, i, len, x, y, col_i, row_i, c) -> color
  if c >= '0' && c <= '9' do
    return rgb(255, 180, 40)
  end
  if c >= 'A' && c <= 'Z' do
    return rgb(80, 220, 220)
  end
  if c >= 'a' && c <= 'z' do
    return rgb(180, 200, 255)
  end
  return rgb(110, 110, 120)
end
```

### Pulsing grayscale

```tsl
fn main(t) -> color
  let phase  = time() * 3.0 + t * 6.283
  let bright = mix(0.35, 1.0, (sin(phase) + 1.0) * 0.5)
  return gray(bright)
end
```

### Replace characters and set fg/bg from an extern

```tsl
extern (char, color) accent = ('*', rgb(255, 90, 30))

fn main(t, i, len, x, y, col_i, row_i, c, original) -> (char, color, color)
  let wave = (sin(time() * 2.0 + x * 8.0 + y * 4.0) + 1.0) * 0.5
  let fg = mixc(original, accent, wave)
  let bg = rgb(8, 12, 20)
  if c == 32 do
    return (42, fg, bg)
  end
  return (c, fg, bg)
end
```

Note: when a function returns a tuple, the first element is the character
(as a codepoint), the second is the foreground color, and the third (if
present) is the background color. The extern `accent` here is a `(char,color)`
tuple that the shader unpacks implicitly - the VM knows the type from the
extern declaration and handles it as a glyph value.

### 3D shading with vectors and matrices

```tsl
fn main(t, i, len, x, y) -> color
  let n = normalize(vec3(x - 0.5, y - 0.5, 0.35))
  let l = normalize(vec3(0.35, 0.6, 0.72))
  let v = normalize(vec3(0.0, 0.0, 1.0))

  let ndotl = max(dot(n, l), 0.0)
  let r = reflect(-l, n)
  let spec = pow(max(dot(r, v), 0.0), 16.0)

  let basis = matrix3(
    1.0, 0.0,  0.0,
    0.0, 0.0, -1.0,
    0.0, 1.0,  0.0
  )
  let bent = basis * n
  let c = normalize(cross(n, l))

  let rr = clamp((bent.x + 1.0) * 0.5 + spec * 0.4, 0.0, 1.0)
  let gg = clamp((bent.y + 1.0) * 0.5 * ndotl, 0.0, 1.0)
  let bb = clamp((c.z + 1.0) * 0.5, 0.0, 1.0)

  return rgb(rr * 255, gg * 255, bb * 255)
end
```
