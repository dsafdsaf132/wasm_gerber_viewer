# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WASM Gerber Viewer is a high-performance web-based Gerber file (RS-274X) viewer using WebGL2 for rendering and Rust/WASM for parsing and geometry processing.

## Architecture

### Technology Stack
- **Frontend**: JavaScript (ES6 modules), WebGL2
- **Backend**: Rust compiled to WebAssembly (WASM)
- **Rendering**: Native WebGL2 with shader programs
- **Geometry**: Polygon triangulation in Rust/WASM with JavaScript fallback

### Project Structure

```
wasm_gerber_viewer/
├── js/
│   └── main.js                    # Main JavaScript application (UI/WebGL)
├── wasm/                          # Rust WebAssembly module
│   ├── src/
│   │   ├── lib.rs                 # WASM entry point (GerberProcessor)
│   │   ├── parser.rs              # RS-274X Gerber format parser
│   │   ├── renderer.rs            # WebGL2 renderer (shaders, rendering)
│   │   └── shape.rs               # Geometry types (Primitive, GerberData)
│   └── Cargo.toml                 # Rust dependencies
├── index.html                     # Main HTML file
└── CLAUDE.md                      # This file
```

## Core Modules

### 1. GerberProcessor (lib.rs)
- WASM entry point exposed to JavaScript
- Methods: `new()`, `init()`, `add_layer()`, `parse()`, `render()`, `composite()`, `get_boundary()`
- Manages layers and WebGL rendering state
- Delegates parsing to parser and rendering to renderer

### 2. Gerber Parser (parser.rs)
- Implements RS-274X Gerber format specification
- **Key Features**:
  - Aperture definition parsing (C, R, O, P shapes)
  - Aperture macro parsing and instantiation
  - G-code command processing (G01, G02, G03, G36, G37, G70, G71, G74, G75, G90, G91)
  - Coordinate data extraction and conversion
  - Region fill (G36/G37) with triangulation
  - Single-quadrant and multi-quadrant arc support

- **Important Functions**:
  - `parse_gerber()` - Main entry point
  - `execute_interpolation()` - Linear/arc drawing
  - `parse_macro()` - Aperture macro processing
  - `triangulate_outline()` - Region triangulation

### 3. WebGL Renderer (renderer.rs)
- **Shader Programs** (5 types):
  - Triangle: Basic filled polygons
  - Circle: Instanced circle rendering
  - Arc: Complex arc with angle range checking
  - Thermal: Thermal via patterns with gaps
  - Texture: FBO compositing for alpha blending

- **Rendering Pipeline**:
  1. Create FBOs (Framebuffer Objects) for each layer
  2. Render primitives to FBOs using shader programs
  3. Composite FBOs to canvas with alpha blending
  4. Support for both additive (clear) and darken (dark) polarity

- **GLSL Shaders**:
  - All shaders defined as compile-time constants (no runtime compilation)
  - Optimized with `lowp` precision for mobile devices
  - Instance rendering for efficient batch processing

### 4. Shape Types (shape.rs)
- `Primitive`: Enum for geometric primitives (Line, Arc, Flash, Region)
- `GerberData`: Container for parsed layer data
- `Boundary`: Bounding box information
- Triangle, Circle, Arc, Thermal collections for rendering

## Working with the Parser

### Adding New G-Codes
1. Add case in `execute_interpolation()` function (parser.rs ~2000)
2. Update `state.interpolation_mode` or `state.coordinate_mode` as needed
3. Test with Gerber files using that G-code

### Fixing Arc Rendering Issues (G02, G03)
- Single-quadrant mode: Arc center selection from 4 candidates (±I, ±J)
  - Must satisfy: radius consistency and sweep angle ≤ 90°
  - Located in arc rendering code (~2395-2432)
- Multi-quadrant mode: Direct offset calculation (start + I, start + J)
- See `find_correct_arc_center_single_quadrant()` logic for validation criteria

### Aperture Macro Parsing
- Macros stored in `HashMap<String, ApertureMacro>`
- Statements parsed as comma-separated primitives
- Primitives (codes 1-22) interpreted with variable substitution
- Variable expressions evaluated with `evaluate_expression()`

## Working with the Renderer

### Adding New Shader Programs
1. Define vertex/fragment shader as constants (renderer.rs ~26+)
2. Create `ShaderProgram` in `create_shader_programs()`
3. Add buffer/VAO setup in corresponding render function
4. Update compositing logic if needed

### WebGL State Management
- **FBOs**: One per active layer for deferred rendering
- **VAOs**: One per shape type (triangle, circle, arc, thermal)
- **Transform Matrix**: Updated per render call (zoom, pan, aspect ratio)
- **Blending**: Configured for additive composition with alpha

## Performance Considerations

### Optimizations
- ✅ **Shader Compilation**: Compile-time constants, init-time only
- ✅ **Instance Rendering**: Used for circles, arcs (batch processing)
- ✅ **Iterator Chains**: Zero-cost abstractions in Rust
- ✅ **String Handling**: Minimized allocations with `&str` where possible
- ✅ **FBO Reuse**: Layers maintain persistent framebuffers

### Known Limitations
- Clippy warnings (6 remaining): Mostly dead code (unused methods/fields for future use)
- `parse_command()`: 10 arguments (architectural design)

## Debugging Tips

### Parser Issues
- Enable logging in `execute_interpolation()` to trace shape generation
- Check coordinate format specification (FS command) for proper scaling
- Verify aperture definitions are parsed before use (ADD commands)

### Rendering Issues
- Check FBO attachment status in `Renderer::new()`
- Verify transform matrix calculation in `Camera::get_transform_matrix()`
- Use WebGL error checking in shader compilation

### Common Problems
- **G03 + G75 Arc Errors**: Multi-candidate center selection required (see arc rendering section)
- **Missing Shapes**: Verify aperture D-code is selected before drawing
- **Wrong Colors**: Check layer polarity (dark/clear) and blending mode

## Building and Testing

```bash
# Build WASM module
cd wasm
cargo build --release

# Check for warnings
cargo clippy

# Run from project root
python3 -m http.server 8000
# Visit: http://localhost:8000
```

## Code Style

- Rust: Follow clippy suggestions (run `cargo clippy`)
- Function comments: JSDoc-style for public WASM functions
- Variable names: snake_case in Rust, camelCase in JavaScript
- Constants: UPPER_CASE (especially shader sources and WebGL constants)

## Recent Changes

### Latest Fix (Commit 4608504)
- **G03+G75 Arc Rendering**: Implemented proper single-quadrant arc center selection
  - Tests 4 candidate centers to find correct one
  - Validates radius consistency and sweep angle constraints
- **Clippy Optimization**: Reduced warnings from 17 to 6
  - Iterator chains instead of index-based loops
  - Removed redundant String allocations
  - Added Default trait derive

See git log for complete commit history.
