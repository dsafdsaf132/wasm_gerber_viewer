# WASM Gerber Viewer

WASM/WebGL2-based Gerber file viewer for PCB visualization.

[Web site](wasm-gerber-viewer.vercel.app)

## Features

- High-performance rendering when huge Gerber files(>10MB) are uploaded
- WebGL2 hardware-accelerated rendering via WASM

## Limitations

This project focuses on high-performance rendering, but it does not render accurately.

As it is a Work In Progress, some Gerber syntax may not be fully supported.

## Quick Start

```bash
git clone https://github.com/dsafdsaf132/wasm_gerber_viewer.git
cd wasm_gerber_viewer
python3 -m http.server 8000
```

Open `http://localhost:8000` and upload files.

## Project Structure

```
wasm_gerber_viewer/
├── index.html                             # Main page
├── js/                                    # JavaScript files
│   └── main.js                            # Main application (GerberViewerApp)
├── css/                                   # Stylesheets
│   └── style.css                          # Application styles
└── wasm/                                  # Rust/WASM module
    ├── Cargo.toml                         # Rust dependencies
    ├── src/                               # Rust source
    │   ├── lib.rs                         # WASM entry point (GerberProcessor)
    │   ├── parser.rs                      # Gerber file parser
    │   ├── renderer.rs                    # WebGL2 renderer
    │   └── shape.rs                       # Geometry data structures
    └── pkg/                               # WASM build output
        ├── wasm_gerber_processor.js       # JS bindings
        └── wasm_gerber_processor_bg.wasm  # WASM binary
```

## Build Instructions

```bash
# Build WASM module (requires Rust and wasm-pack)
cd wasm
wasm-pack build --target web --out-dir pkg --release
```

## Browser Requirements

Modern browsers with WebGL2 support:

- Chrome 80+, Firefox 75+, Safari 15+, Edge 80+

## Work in Progress

The following Gerber commands are not yet implemented

- **%AB** - Aperture Block definitions
- **%LM** - Layer Mirroring (X, Y, XY axes)
- **%LR** - Layer Rotation transformations
- **%LS** - Layer Scaling transformations
- **%SR** - Step and Repeat operations

## License

[MIT License](LICENSE)
