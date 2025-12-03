# WASM Gerber Viewer

WASM/WebGL2-based Gerber file viewer for PCB visualization.

[website](https://wasm-gerber-viewer.vercel.app/)

## Features

- High-performance rendering when huge Gerber files(>10MB) are uploaded
- WebGL2 hardware-accelerated rendering via WASM
- Touch support for mobile devices

## Limitations

This project focuses on high-performance rendering, but it does not render accurately.

As it is a Work In Progress, some Gerber syntax may not be fully supported.

## Requirements

- **Rust** - [Install Rust](https://rustup.rs/)
- **wasm-pack** - Install via: `cargo install wasm-pack`
- **Python 3** - For running the local HTTP server

## Quick Start

```bash
git clone https://github.com/dsafdsaf132/wasm_gerber_viewer.git
cd wasm_gerber_viewer

# Build WASM module
wasm-pack build wasm --target web --out-dir pkg --release

# Start development server
python3 -m http.server 8000
```

Open `http://localhost:8000` and upload Gerber files.

## Project Structure

```
wasm_gerber_viewer/
├── index.html                             # Main page
├── js/                                    # JavaScript files
│   └── main.js                            # Main application (GerberViewer)
├── css/                                   # Stylesheets
│   └── style.css                          # Application styles
└── wasm/                                  # Rust/WASM module
    ├── Cargo.toml                         # Rust dependencies
    └── src/                               # Rust source
        ├── lib.rs                         # WASM entry point (GerberProcessor)
        ├── shape.rs                       # Geometry data structures
        ├── parser.rs                      # Parser entry point and main logic
        ├── parser/                        # Gerber file parsing submodules
        │   ├── geometry.rs                # Geometric operations and primitives
        │   ├── state.rs                   # Parser state and configuration
        │   ├── aperture.rs                # Aperture definitions and parsing
        │   └── aperture_macro.rs          # Aperture macro definitions and parsing
        ├── renderer.rs                    # Renderer core logic
        └── renderer/                      # WebGL2 rendering submodules
            ├── shader.rs                  # Shader compilation and WebGL constants
            ├── camera.rs                  # Camera and viewport transformations
            └── buffer.rs                  # GPU buffer and framebuffer structures
```

## Browser Requirements

Modern browsers with WebGL2 support:

- Chrome 80+, Firefox 75+, Safari 15+, Edge 80+

## Work in Progress

The following Gerber commands are not yet implemented

- **%AB** - Aperture Block definitions
- **%LR** - Layer Rotation transformations

## License

[MIT License](LICENSE)
