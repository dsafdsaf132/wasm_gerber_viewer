use crate::shape::{Boundary, GerberData};
use js_sys::Float32Array;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::{
    WebGl2RenderingContext, WebGlBuffer, WebGlFramebuffer, WebGlProgram, WebGlShader, WebGlTexture,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

// WebGL constants
const COLOR_BUFFER_BIT: u32 = WebGl2RenderingContext::COLOR_BUFFER_BIT;
const TRIANGLES: u32 = WebGl2RenderingContext::TRIANGLES;
const FLOAT: u32 = WebGl2RenderingContext::FLOAT;
const UNSIGNED_INT: u32 = WebGl2RenderingContext::UNSIGNED_INT;
const ARRAY_BUFFER: u32 = WebGl2RenderingContext::ARRAY_BUFFER;
const ELEMENT_ARRAY_BUFFER: u32 = WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER;
const STATIC_DRAW: u32 = WebGl2RenderingContext::STATIC_DRAW;
const VERTEX_SHADER: u32 = WebGl2RenderingContext::VERTEX_SHADER;
const FRAGMENT_SHADER: u32 = WebGl2RenderingContext::FRAGMENT_SHADER;
const BLEND: u32 = WebGl2RenderingContext::BLEND;
const ONE_MINUS_SRC_ALPHA: u32 = WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA;
const ONE: u32 = WebGl2RenderingContext::ONE;
const FUNC_ADD: u32 = WebGl2RenderingContext::FUNC_ADD;
const ZERO: u32 = WebGl2RenderingContext::ZERO;
// Shader sources
const TRIANGLE_VERTEX_SHADER: &str = r#"#version 300 es
in vec2 position;
uniform mat3 transform;
void main() {
    vec3 transformed = transform * vec3(position, 1.0);
    gl_Position = vec4(transformed.xy, 0.0, 1.0);
}
"#;

const TRIANGLE_FRAGMENT_SHADER: &str = r#"#version 300 es
precision lowp float;
uniform vec4 color;
out vec4 fragColor;
void main() {
    fragColor = color;
}
"#;

const CIRCLE_VERTEX_SHADER: &str = r#"#version 300 es
in vec2 position;
in vec2 center_instance;
in float radius_instance;
uniform mat3 transform;
out lowp vec2 vPosition;
void main() {
    vec2 scaledPos = position * radius_instance + center_instance;
    vec3 transformed = transform * vec3(scaledPos, 1.0);
    gl_Position = vec4(transformed.xy, 0.0, 1.0);
    vPosition = position;
}
"#;

const CIRCLE_FRAGMENT_SHADER: &str = r#"#version 300 es
precision lowp float;
in lowp vec2 vPosition;
uniform vec4 color;
out vec4 fragColor;
void main() {
    if (dot(vPosition, vPosition) > 1.0) discard;
    fragColor = color;
}
"#;

const ARC_VERTEX_SHADER: &str = r#"#version 300 es
in vec2 position;
in vec2 center_instance;
in float radius_instance;
in float startAngle_instance;
in float sweepAngle_instance;
in float thickness_instance;
uniform mat3 transform;
out lowp vec2 vPosition;
out lowp float vRadius;
out lowp float vStartAngle;
out lowp float vSweepAngle;
out lowp float vThickness;
void main() {
    float maxRadius = radius_instance + thickness_instance;
    vec2 scaledPos = position * maxRadius + center_instance;
    vec3 transformed = transform * vec3(scaledPos, 1.0);
    gl_Position = vec4(transformed.xy, 0.0, 1.0);
    vPosition = position * maxRadius;
    vRadius = radius_instance;
    vStartAngle = startAngle_instance;
    vSweepAngle = sweepAngle_instance;
    vThickness = thickness_instance;
}
"#;

const ARC_FRAGMENT_SHADER: &str = r#"#version 300 es
precision lowp float;
in lowp vec2 vPosition;
in lowp float vRadius;
in lowp float vStartAngle;
in lowp float vSweepAngle;
in lowp float vThickness;
uniform vec4 color;
out vec4 fragColor;

const float PI = 3.14159265359;
const float TWO_PI = 6.28318530718;

float normalizeAngle(float angle) {
    float normalized = mod(angle, TWO_PI);
    if (normalized < 0.0) {
        normalized += TWO_PI;
    }
    return normalized;
}

void main() {
    float dist = length(vPosition);
    float angle = atan(vPosition.y, vPosition.x);

    angle = normalizeAngle(angle);
    float startAngle = normalizeAngle(vStartAngle);
    float endAngle = normalizeAngle(startAngle + vSweepAngle);

    float innerRadius = vRadius - vThickness * 0.5;
    float outerRadius = vRadius + vThickness * 0.5;

    if (dist < innerRadius || dist > outerRadius) {
        discard;
    }

    bool inRange;
    if (vSweepAngle > 0.0) {
        if (endAngle > startAngle) {
            inRange = angle >= startAngle && angle <= endAngle;
        } else {
            inRange = angle >= startAngle || angle <= endAngle;
        }
    } else {
        if (endAngle < startAngle) {
            inRange = angle <= startAngle && angle >= endAngle;
        } else {
            inRange = angle <= startAngle || angle >= endAngle;
        }
    }

    if (!inRange) {
        discard;
    }

    fragColor = color;
}
"#;

const THERMAL_VERTEX_SHADER: &str = r#"#version 300 es
in vec2 position;
in vec2 center_instance;
in float outer_diameter_instance;
in float inner_diameter_instance;
in float gap_thickness_instance;
in float rotation_instance;
uniform mat3 transform;
out lowp vec2 vPosition;
out lowp float vInnerDiameter;
out lowp float vOuterDiameter;
out lowp float vGapThickness;
out lowp float vRotation;
void main() {
    float outer_radius = outer_diameter_instance / 2.0;
    vec2 scaledPos = position * outer_radius + center_instance;
    vec3 transformed = transform * vec3(scaledPos, 1.0);
    gl_Position = vec4(transformed.xy, 0.0, 1.0);
    vPosition = position;
    vInnerDiameter = inner_diameter_instance;
    vOuterDiameter = outer_diameter_instance;
    vGapThickness = gap_thickness_instance;
    vRotation = rotation_instance;
}
"#;

const THERMAL_FRAGMENT_SHADER: &str = r#"#version 300 es
precision lowp float;
in lowp vec2 vPosition;
in lowp float vInnerDiameter;
in lowp float vOuterDiameter;
in lowp float vGapThickness;
in lowp float vRotation;
uniform vec4 color;
out vec4 fragColor;

void main() {
    // Apply rotation to vPosition
    float cosR = cos(vRotation);
    float sinR = sin(vRotation);
    vec2 rotated = vec2(
        vPosition.x * cosR - vPosition.y * sinR,
        vPosition.x * sinR + vPosition.y * cosR
    );

    float dist = length(rotated);
    float inner_radius = vInnerDiameter / (2.0 * vOuterDiameter);
    float outer_radius = 0.5;

    // Discard if outside outer radius or inside inner radius
    if (dist > outer_radius || dist < inner_radius) {
        discard;
    }

    // Compute half gap thickness in normalized space
    float half_gap = vGapThickness / (2.0 * vOuterDiameter);

    // Discard if in cross-shaped gap region
    if (abs(rotated.x) < half_gap || abs(rotated.y) < half_gap) {
        discard;
    }

    fragColor = color;
}
"#;

const TEXTURE_VERTEX_SHADER: &str = r#"#version 300 es
in vec2 position;
out vec2 v_uv;
void main() {
    v_uv = position * 0.5 + 0.5;
    gl_Position = vec4(position, 0.0, 1.0);
}
"#;

const TEXTURE_FRAGMENT_SHADER: &str = r#"#version 300 es
precision lowp float;
in vec2 v_uv;
uniform sampler2D u_texture;
uniform vec4 u_color;
out vec4 fragColor;
void main() {
    vec4 texColor = texture(u_texture, v_uv);
    // Pre-multiply alpha: color * alpha for additive blending
    float finalAlpha = u_color.a * texColor.a;
    fragColor = vec4(u_color.rgb * finalAlpha, finalAlpha);
}
"#;

/// Camera transformation
struct Camera {
    zoom: f32,
    offset_x: f32,
    offset_y: f32,
}

impl Camera {
    fn new() -> Camera {
        Camera {
            zoom: 2.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    fn get_transform_matrix(&self, canvas_width: u32, canvas_height: u32) -> [f32; 9] {
        let aspect = canvas_width as f32 / canvas_height as f32;

        let (scale_x, scale_y) = if aspect > 1.0 {
            (self.zoom / aspect, self.zoom)
        } else {
            (self.zoom, self.zoom * aspect)
        };

        let (offset_x, offset_y) = if aspect > 1.0 {
            (self.offset_x / aspect, self.offset_y)
        } else {
            (self.offset_x, self.offset_y * aspect)
        };

        [
            scale_x, 0.0, 0.0, 0.0, scale_y, 0.0, offset_x, offset_y, 1.0,
        ]
    }
}

/// Shader program with uniform locations
struct ShaderProgram {
    program: WebGlProgram,
    uniforms: HashMap<String, WebGlUniformLocation>,
    attributes: HashMap<String, u32>,
}

/// All shader programs
struct ShaderPrograms {
    triangle: ShaderProgram,
    circle: ShaderProgram,
    arc: ShaderProgram,
    thermal: ShaderProgram,
    texture: ShaderProgram,
}

struct Fbo {
    framebuffer: WebGlFramebuffer,
    texture: WebGlTexture,
}

/// Buffer cache for geometry rendering (per polarity sublayer)
#[derive(Default)]
struct BufferCache {
    // Triangles cache
    triangle_vao: Option<WebGlVertexArrayObject>,
    triangle_vertex_buffer: Option<WebGlBuffer>,
    triangle_index_buffer: Option<WebGlBuffer>,

    // Circles cache
    circle_vao: Option<WebGlVertexArrayObject>,
    circle_center_buffer: Option<WebGlBuffer>,
    circle_radius_buffer: Option<WebGlBuffer>,

    // Arcs cache
    arc_vao: Option<WebGlVertexArrayObject>,
    arc_center_buffer: Option<WebGlBuffer>,
    arc_radius_buffer: Option<WebGlBuffer>,
    arc_start_angle_buffer: Option<WebGlBuffer>,
    arc_sweep_angle_buffer: Option<WebGlBuffer>,
    arc_thickness_buffer: Option<WebGlBuffer>,

    // Thermals cache
    thermal_vao: Option<WebGlVertexArrayObject>,
    thermal_center_buffer: Option<WebGlBuffer>,
    thermal_outer_diameter_buffer: Option<WebGlBuffer>,
    thermal_inner_diameter_buffer: Option<WebGlBuffer>,
    thermal_gap_thickness_buffer: Option<WebGlBuffer>,
    thermal_rotation_buffer: Option<WebGlBuffer>,
}

/// Metadata for a single user layer (may contain multiple polarity sublayers)
pub struct LayerMetadata {
    gerber_data: Vec<GerberData>,    // Polarity sublayers for this layer
    fbo: Fbo,                        // FBO for rendering this layer
    buffer_caches: Vec<BufferCache>, // Buffer cache per polarity sublayer
    boundary: Boundary,              // Combined boundary
}

/// WebGL renderer for Gerber graphics with multi-layer support
pub struct Renderer {
    gl: WebGl2RenderingContext,
    layers: Vec<Option<LayerMetadata>>, // Sparse vec (None = deallocated slot)
    layer_count: usize,                 // Active layer count
    programs: ShaderPrograms,
    camera: Camera,
    quad_buffer: WebGlBuffer, // Shared quad buffer for all layers
}

impl Renderer {
    /// Create a new renderer with WebGL context (no layers initially)
    pub fn new(gl: WebGl2RenderingContext) -> Result<Renderer, JsValue> {
        // Compile shader programs
        let programs = Self::create_shader_programs(&gl)?;

        // Create quad buffer for instanced rendering (shared across all layers)
        let quad_buffer = Self::create_quad_buffer(&gl)?;

        Ok(Renderer {
            gl,
            layers: Vec::new(),
            layer_count: 0,
            programs,
            camera: Camera::new(),
            quad_buffer,
        })
    }

    /// Add a new layer with parsed Gerber data
    /// Returns the layer index (layer_id)
    pub fn add_layer(&mut self, gerber_data: Vec<GerberData>) -> Result<usize, JsValue> {
        let (width, height) = self.get_canvas_size()?;

        // Calculate combined boundary from all polarity sublayers
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for data in &gerber_data {
            let b = &data.boundary;
            min_x = min_x.min(b.min_x);
            max_x = max_x.max(b.max_x);
            min_y = min_y.min(b.min_y);
            max_y = max_y.max(b.max_y);
        }

        let boundary = Boundary::new(min_x, max_x, min_y, max_y);

        // Create FBO for this layer
        let fbo = Self::create_fbo(&self.gl, width, height)?;

        // Create buffer caches for each polarity sublayer
        let mut buffer_caches = Vec::new();
        for _ in 0..gerber_data.len() {
            buffer_caches.push(BufferCache {
                triangle_vao: None,
                triangle_vertex_buffer: None,
                triangle_index_buffer: None,
                circle_vao: None,
                circle_center_buffer: None,
                circle_radius_buffer: None,
                arc_vao: None,
                arc_center_buffer: None,
                arc_radius_buffer: None,
                arc_start_angle_buffer: None,
                arc_sweep_angle_buffer: None,
                arc_thickness_buffer: None,
                thermal_vao: None,
                thermal_center_buffer: None,
                thermal_outer_diameter_buffer: None,
                thermal_inner_diameter_buffer: None,
                thermal_gap_thickness_buffer: None,
                thermal_rotation_buffer: None,
            });
        }

        let layer_metadata = LayerMetadata {
            gerber_data,
            fbo,
            buffer_caches,
            boundary,
        };

        // Find next free slot or extend vec
        if let Some(free_slot) = self.layers.iter().position(|layer| layer.is_none()) {
            self.layers[free_slot] = Some(layer_metadata);
            self.layer_count += 1;
            Ok(free_slot)
        } else {
            self.layers.push(Some(layer_metadata));
            self.layer_count += 1;
            Ok(self.layers.len() - 1)
        }
    }

    /// Remove a layer by index
    pub fn remove_layer(&mut self, layer_id: usize) -> Result<(), JsValue> {
        if layer_id >= self.layers.len() || self.layers[layer_id].is_none() {
            return Err(JsValue::from_str(&format!(
                "Invalid layer_id: {}",
                layer_id
            )));
        }

        // Remove layer metadata (which will drop cached WebGL resources)
        if let Some(layer) = self.layers[layer_id].take() {
            // Delete framebuffer and texture
            self.gl.delete_framebuffer(Some(&layer.fbo.framebuffer));
            self.gl.delete_texture(Some(&layer.fbo.texture));

            // Delete all cached buffers and VAOs
            for cache in layer.buffer_caches {
                // Delete triangle cache
                if let Some(vao) = cache.triangle_vao {
                    self.gl.delete_vertex_array(Some(&vao));
                }
                if let Some(buf) = cache.triangle_vertex_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.triangle_index_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }

                // Delete circle cache
                if let Some(vao) = cache.circle_vao {
                    self.gl.delete_vertex_array(Some(&vao));
                }
                if let Some(buf) = cache.circle_center_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.circle_radius_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }

                // Delete arc cache
                if let Some(vao) = cache.arc_vao {
                    self.gl.delete_vertex_array(Some(&vao));
                }
                if let Some(buf) = cache.arc_center_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.arc_radius_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.arc_start_angle_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.arc_sweep_angle_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.arc_thickness_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }

                // Delete thermal cache
                if let Some(vao) = cache.thermal_vao {
                    self.gl.delete_vertex_array(Some(&vao));
                }
                if let Some(buf) = cache.thermal_center_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.thermal_outer_diameter_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.thermal_inner_diameter_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.thermal_gap_thickness_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
                if let Some(buf) = cache.thermal_rotation_buffer {
                    self.gl.delete_buffer(Some(&buf));
                }
            }
        }

        self.layer_count -= 1;
        Ok(())
    }

    /// Clear all layers and clean up WebGL resources
    pub fn clear_all(&mut self) {
        // Delete all cached resources for each layer
        for layer_opt in self.layers.drain(..) {
            if let Some(layer) = layer_opt {
                // Delete framebuffer and texture
                self.gl.delete_framebuffer(Some(&layer.fbo.framebuffer));
                self.gl.delete_texture(Some(&layer.fbo.texture));

                // Delete all cached buffers and VAOs
                for cache in layer.buffer_caches {
                    // Delete triangle cache
                    if let Some(vao) = cache.triangle_vao {
                        self.gl.delete_vertex_array(Some(&vao));
                    }
                    if let Some(buf) = cache.triangle_vertex_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.triangle_index_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }

                    // Delete circle cache
                    if let Some(vao) = cache.circle_vao {
                        self.gl.delete_vertex_array(Some(&vao));
                    }
                    if let Some(buf) = cache.circle_center_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.circle_radius_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }

                    // Delete arc cache
                    if let Some(vao) = cache.arc_vao {
                        self.gl.delete_vertex_array(Some(&vao));
                    }
                    if let Some(buf) = cache.arc_center_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.arc_radius_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.arc_start_angle_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.arc_sweep_angle_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.arc_thickness_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }

                    // Delete thermal cache
                    if let Some(vao) = cache.thermal_vao {
                        self.gl.delete_vertex_array(Some(&vao));
                    }
                    if let Some(buf) = cache.thermal_center_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.thermal_outer_diameter_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.thermal_inner_diameter_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.thermal_gap_thickness_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                    if let Some(buf) = cache.thermal_rotation_buffer {
                        self.gl.delete_buffer(Some(&buf));
                    }
                }
            }
        }
        self.layer_count = 0;
    }

    /// Compile a shader
    fn compile_shader(
        gl: &WebGl2RenderingContext,
        shader_type: u32,
        source: &str,
    ) -> Result<WebGlShader, JsValue> {
        let shader = gl
            .create_shader(shader_type)
            .ok_or_else(|| JsValue::from_str("Failed to create shader"))?;

        gl.shader_source(&shader, source);
        gl.compile_shader(&shader);

        if !gl
            .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            let log = gl
                .get_shader_info_log(&shader)
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(JsValue::from_str(&format!(
                "Shader compilation failed: {}",
                log
            )));
        }

        Ok(shader)
    }

    /// Create a shader program
    fn create_program(
        gl: &WebGl2RenderingContext,
        vertex_src: &str,
        fragment_src: &str,
        uniform_names: &[&str],
        attribute_names: &[&str],
    ) -> Result<ShaderProgram, JsValue> {
        let vertex_shader = Self::compile_shader(gl, VERTEX_SHADER, vertex_src)?;
        let fragment_shader = Self::compile_shader(gl, FRAGMENT_SHADER, fragment_src)?;

        let program = gl
            .create_program()
            .ok_or_else(|| JsValue::from_str("Failed to create program"))?;

        gl.attach_shader(&program, &vertex_shader);
        gl.attach_shader(&program, &fragment_shader);
        gl.link_program(&program);

        if !gl
            .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            let log = gl
                .get_program_info_log(&program)
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(JsValue::from_str(&format!(
                "Program linking failed: {}",
                log
            )));
        }

        // Get uniform locations
        let mut uniforms = HashMap::new();
        for name in uniform_names {
            if let Some(location) = gl.get_uniform_location(&program, name) {
                uniforms.insert(name.to_string(), location);
            }
        }

        // Get attribute locations
        let mut attributes = HashMap::new();
        for name in attribute_names {
            let location = gl.get_attrib_location(&program, name) as u32;
            attributes.insert(name.to_string(), location);
        }

        Ok(ShaderProgram {
            program,
            uniforms,
            attributes,
        })
    }

    /// Create all shader programs
    fn create_shader_programs(gl: &WebGl2RenderingContext) -> Result<ShaderPrograms, JsValue> {
        let triangle = Self::create_program(
            gl,
            TRIANGLE_VERTEX_SHADER,
            TRIANGLE_FRAGMENT_SHADER,
            &["transform", "color"],
            &["position"],
        )?;

        let circle = Self::create_program(
            gl,
            CIRCLE_VERTEX_SHADER,
            CIRCLE_FRAGMENT_SHADER,
            &["transform", "color"],
            &["position", "center_instance", "radius_instance"],
        )?;

        let arc = Self::create_program(
            gl,
            ARC_VERTEX_SHADER,
            ARC_FRAGMENT_SHADER,
            &["transform", "color"],
            &[
                "position",
                "center_instance",
                "radius_instance",
                "startAngle_instance",
                "sweepAngle_instance",
                "thickness_instance",
            ],
        )?;

        let thermal = Self::create_program(
            gl,
            THERMAL_VERTEX_SHADER,
            THERMAL_FRAGMENT_SHADER,
            &["transform", "color"],
            &[
                "position",
                "center_instance",
                "outer_diameter_instance",
                "inner_diameter_instance",
                "gap_thickness_instance",
                "rotation_instance",
            ],
        )?;

        let texture = Self::create_program(
            gl,
            TEXTURE_VERTEX_SHADER,
            TEXTURE_FRAGMENT_SHADER,
            &["u_texture", "u_color"],
            &["position"],
        )?;

        Ok(ShaderPrograms {
            triangle,
            circle,
            arc,
            thermal,
            texture,
        })
    }

    fn create_fbo(gl: &WebGl2RenderingContext, width: u32, height: u32) -> Result<Fbo, JsValue> {
        let texture = gl.create_texture().ok_or("Failed to create texture")?;
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA as i32,
            width as i32,
            height as i32,
            0,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            None,
        )?;
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::LINEAR as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_S,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_T,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );

        let framebuffer = gl.create_framebuffer().ok_or("Failed to create FBO")?;
        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&framebuffer));
        gl.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::COLOR_ATTACHMENT0,
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&texture),
            0,
        );

        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);
        gl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);

        Ok(Fbo {
            framebuffer,
            texture,
        })
    }

    /// Create and bind a single-channel instance buffer
    fn create_instance_buffer(
        gl: &WebGl2RenderingContext,
        data: &[f32],
        program: &ShaderProgram,
        attr_name: &str,
        divisor: u32,
    ) -> Result<WebGlBuffer, JsValue> {
        let buffer = gl
            .create_buffer()
            .ok_or_else(|| JsValue::from_str("Failed to create buffer"))?;
        gl.bind_buffer(ARRAY_BUFFER, Some(&buffer));
        unsafe {
            let array = Float32Array::view(data);
            gl.buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
        }
        let loc = *program.attributes.get(attr_name).unwrap();
        gl.enable_vertex_attrib_array(loc);
        gl.vertex_attrib_pointer_with_i32(loc, 1, FLOAT, false, 0, 0);
        gl.vertex_attrib_divisor(loc, divisor);
        Ok(buffer)
    }

    /// Create and bind a dual-channel (2D) instance buffer
    fn create_instance_buffer_2d(
        gl: &WebGl2RenderingContext,
        data: &[f32],
        program: &ShaderProgram,
        attr_name: &str,
        divisor: u32,
    ) -> Result<WebGlBuffer, JsValue> {
        let buffer = gl
            .create_buffer()
            .ok_or_else(|| JsValue::from_str("Failed to create buffer"))?;
        gl.bind_buffer(ARRAY_BUFFER, Some(&buffer));
        unsafe {
            let array = Float32Array::view(data);
            gl.buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
        }
        let loc = *program.attributes.get(attr_name).unwrap();
        gl.enable_vertex_attrib_array(loc);
        gl.vertex_attrib_pointer_with_i32(loc, 2, FLOAT, false, 0, 0);
        gl.vertex_attrib_divisor(loc, divisor);
        Ok(buffer)
    }

    /// Interleave x,y arrays into a single flat array
    fn interleave_xy(x: &[f32], y: &[f32]) -> Vec<f32> {
        let mut result = Vec::with_capacity(x.len() * 2);
        for i in 0..x.len() {
            result.push(x[i]);
            result.push(y[i]);
        }
        result
    }

    /// Create quad buffer for instanced rendering
    fn create_quad_buffer(gl: &WebGl2RenderingContext) -> Result<WebGlBuffer, JsValue> {
        let vertices: [f32; 12] = [
            -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
        ];

        let buffer = gl
            .create_buffer()
            .ok_or_else(|| JsValue::from_str("Failed to create quad buffer"))?;

        gl.bind_buffer(ARRAY_BUFFER, Some(&buffer));

        unsafe {
            let array = Float32Array::view(&vertices);
            gl.buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
        }

        Ok(buffer)
    }

    fn get_canvas_size_from_gl(gl: &WebGl2RenderingContext) -> Result<(u32, u32), JsValue> {
        let canvas = gl
            .canvas()
            .ok_or_else(|| JsValue::from_str("No canvas"))?
            .dyn_into::<web_sys::HtmlCanvasElement>()?;
        Ok((canvas.width(), canvas.height()))
    }

    /// Get canvas dimensions
    fn get_canvas_size(&self) -> Result<(u32, u32), JsValue> {
        Self::get_canvas_size_from_gl(&self.gl)
    }

    /// Update camera state
    fn update_camera(&mut self, zoom: f32, offset_x: f32, offset_y: f32) {
        self.camera.zoom = zoom;
        self.camera.offset_x = offset_x;
        self.camera.offset_y = offset_y;
    }

    /// Draw a specific FBO texture to the current framebuffer
    fn draw_fbo_texture(&self, texture: &WebGlTexture, color: &[f32; 4]) -> Result<(), JsValue> {
        let program = &self.programs.texture;
        self.gl.use_program(Some(&program.program));

        // Use the shared quad buffer
        self.gl.bind_buffer(ARRAY_BUFFER, Some(&self.quad_buffer));
        let pos_loc = *program.attributes.get("position").unwrap();
        self.gl.enable_vertex_attrib_array(pos_loc);
        self.gl
            .vertex_attrib_pointer_with_i32(pos_loc, 2, FLOAT, false, 0, 0);

        self.gl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.gl
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(texture));
        self.gl.uniform1i(program.uniforms.get("u_texture"), 0);
        self.gl
            .uniform4fv_with_f32_array(program.uniforms.get("u_color"), color);

        self.gl.draw_arrays(TRIANGLES, 0, 6);

        Ok(())
    }

    /// Draw instanced triangles
    fn draw_instanced_triangles(
        &mut self,
        transform: &[f32; 9],
        color: &[f32; 4],
        layer_id: usize,
        sublayer_idx: usize,
    ) -> Result<(), JsValue> {
        // Check if data is empty (short-lived borrow)
        {
            let layer = self.layers[layer_id].as_ref().unwrap();
            if layer.gerber_data[sublayer_idx].triangles.indices.is_empty() {
                return Ok(());
            }
        }

        let program = &self.programs.triangle;
        self.gl.use_program(Some(&program.program));

        // Buffer creation/update phase (scoped to end borrow early)
        let index_count = {
            let layer = self.layers[layer_id]
                .as_mut()
                .ok_or_else(|| JsValue::from_str("Layer not found"))?;
            let triangles = &layer.gerber_data[sublayer_idx].triangles;
            let buffer_cache = &mut layer.buffer_caches[sublayer_idx];

            // Check if VAO is cached for this sublayer
            if buffer_cache.triangle_vao.is_none() {
                // Create VAO
                let vao = self
                    .gl
                    .create_vertex_array()
                    .ok_or_else(|| JsValue::from_str("Failed to create VAO"))?;
                self.gl.bind_vertex_array(Some(&vao));

                // Create and bind vertex buffer
                let vertex_buffer = self
                    .gl
                    .create_buffer()
                    .ok_or_else(|| JsValue::from_str("Failed to create vertex buffer"))?;
                self.gl.bind_buffer(ARRAY_BUFFER, Some(&vertex_buffer));
                unsafe {
                    let array = Float32Array::view(&triangles.vertices);
                    self.gl
                        .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
                }

                // Create and bind index buffer
                let index_buffer = self
                    .gl
                    .create_buffer()
                    .ok_or_else(|| JsValue::from_str("Failed to create index buffer"))?;
                self.gl
                    .bind_buffer(ELEMENT_ARRAY_BUFFER, Some(&index_buffer));
                unsafe {
                    let array = js_sys::Uint32Array::view(&triangles.indices);
                    self.gl.buffer_data_with_array_buffer_view(
                        ELEMENT_ARRAY_BUFFER,
                        &array,
                        STATIC_DRAW,
                    );
                }

                // Set up attributes
                let position_loc = *program.attributes.get("position").unwrap();
                self.gl.enable_vertex_attrib_array(position_loc);
                self.gl
                    .vertex_attrib_pointer_with_i32(position_loc, 2, FLOAT, false, 0, 0);

                // Unbind VAO
                self.gl.bind_vertex_array(None);

                // Cache VAO and buffers for this sublayer
                buffer_cache.triangle_vao = Some(vao);
                buffer_cache.triangle_vertex_buffer = Some(vertex_buffer);
                buffer_cache.triangle_index_buffer = Some(index_buffer);
            }

            triangles.indices.len()
        }; // Borrow ends here

        // Rendering phase (new borrow)
        let layer = self.layers[layer_id].as_ref().unwrap();
        let buffer_cache = &layer.buffer_caches[sublayer_idx];

        // Bind cached VAO for this sublayer
        self.gl
            .bind_vertex_array(buffer_cache.triangle_vao.as_ref());

        // Set uniforms (only these change per frame)
        if let Some(loc) = program.uniforms.get("transform") {
            self.gl
                .uniform_matrix3fv_with_f32_array(Some(loc), false, transform);
        }
        if let Some(loc) = program.uniforms.get("color") {
            self.gl.uniform4fv_with_f32_array(Some(loc), color);
        }

        // Draw
        self.gl
            .draw_elements_with_i32(TRIANGLES, index_count as i32, UNSIGNED_INT, 0);

        // Unbind VAO to prevent state leakage
        self.gl.bind_vertex_array(None);

        Ok(())
    }

    /// Draw instanced circles
    fn draw_instanced_circles(
        &mut self,
        transform: &[f32; 9],
        color: &[f32; 4],
        layer_id: usize,
        sublayer_idx: usize,
    ) -> Result<(), JsValue> {
        // Check if data is empty (short-lived borrow)
        let instance_count = {
            let layer = self.layers[layer_id].as_ref().unwrap();
            layer.gerber_data[sublayer_idx].circles.x.len()
        };
        if instance_count == 0 {
            return Ok(());
        }

        let program = &self.programs.circle;
        self.gl.use_program(Some(&program.program));

        // Get mutable reference to buffer cache and immutable reference to data
        // Split borrowing: gerber_data and buffer_caches are different fields
        let layer = self.layers[layer_id]
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Layer not found"))?;
        let circles = &layer.gerber_data[sublayer_idx].circles;
        let buffer_cache = &mut layer.buffer_caches[sublayer_idx];

        // Check if VAO is cached for this sublayer
        if buffer_cache.circle_vao.is_none() {
            // Create VAO
            let vao = self
                .gl
                .create_vertex_array()
                .ok_or_else(|| JsValue::from_str("Failed to create VAO"))?;
            self.gl.bind_vertex_array(Some(&vao));

            // Bind shared quad buffer for position attribute
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&self.quad_buffer));
            let position_loc = *program.attributes.get("position").unwrap();
            self.gl.enable_vertex_attrib_array(position_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(position_loc, 2, FLOAT, false, 0, 0);

            // Create instance buffers
            let centers = Self::interleave_xy(&circles.x, &circles.y);
            let center_buffer = Self::create_instance_buffer_2d(&self.gl, &centers, program, "center_instance", 1)?;
            let radius_buffer = Self::create_instance_buffer(&self.gl, &circles.radius, program, "radius_instance", 1)?;

            // Unbind VAO
            self.gl.bind_vertex_array(None);

            // Cache VAO and buffers for this sublayer
            buffer_cache.circle_vao = Some(vao);
            buffer_cache.circle_center_buffer = Some(center_buffer);
            buffer_cache.circle_radius_buffer = Some(radius_buffer);
        }

        // Re-get immutable reference for rendering
        let layer = self.layers[layer_id].as_ref().unwrap();
        let buffer_cache = &layer.buffer_caches[sublayer_idx];

        // Bind cached VAO for this sublayer
        self.gl.bind_vertex_array(buffer_cache.circle_vao.as_ref());

        // Set uniforms (only these change per frame)
        if let Some(loc) = program.uniforms.get("transform") {
            self.gl
                .uniform_matrix3fv_with_f32_array(Some(loc), false, transform);
        }
        if let Some(loc) = program.uniforms.get("color") {
            self.gl.uniform4fv_with_f32_array(Some(loc), color);
        }

        // Draw
        self.gl
            .draw_arrays_instanced(TRIANGLES, 0, 6, instance_count as i32);

        // Unbind VAO to prevent state leakage
        self.gl.bind_vertex_array(None);

        Ok(())
    }

    /// Draw instanced arcs
    fn draw_instanced_arcs(
        &mut self,
        transform: &[f32; 9],
        color: &[f32; 4],
        layer_id: usize,
        sublayer_idx: usize,
    ) -> Result<(), JsValue> {
        // Check if data is empty (short-lived borrow)
        let instance_count = {
            let layer = self.layers[layer_id].as_ref().unwrap();
            layer.gerber_data[sublayer_idx].arcs.x.len()
        };
        if instance_count == 0 {
            return Ok(());
        }

        let program = &self.programs.arc;
        self.gl.use_program(Some(&program.program));

        // Get mutable reference to buffer cache and immutable reference to data
        // Split borrowing: gerber_data and buffer_caches are different fields
        let layer = self.layers[layer_id]
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Layer not found"))?;
        let arcs = &layer.gerber_data[sublayer_idx].arcs;
        let buffer_cache = &mut layer.buffer_caches[sublayer_idx];

        // Check if VAO is cached for this sublayer
        if buffer_cache.arc_vao.is_none() {
            // Create VAO
            let vao = self
                .gl
                .create_vertex_array()
                .ok_or_else(|| JsValue::from_str("Failed to create VAO"))?;
            self.gl.bind_vertex_array(Some(&vao));

            // Bind shared quad buffer for position attribute
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&self.quad_buffer));
            let position_loc = *program.attributes.get("position").unwrap();
            self.gl.enable_vertex_attrib_array(position_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(position_loc, 2, FLOAT, false, 0, 0);

            // Create instance buffers
            let centers = Self::interleave_xy(&arcs.x, &arcs.y);
            let center_buffer = Self::create_instance_buffer_2d(&self.gl, &centers, program, "center_instance", 1)?;
            let radius_buffer = Self::create_instance_buffer(&self.gl, &arcs.radius, program, "radius_instance", 1)?;
            let start_angle_buffer = Self::create_instance_buffer(&self.gl, &arcs.start_angle, program, "startAngle_instance", 1)?;
            let sweep_angle_buffer = Self::create_instance_buffer(&self.gl, &arcs.sweep_angle, program, "sweepAngle_instance", 1)?;
            let thickness_buffer = Self::create_instance_buffer(&self.gl, &arcs.thickness, program, "thickness_instance", 1)?;

            // Unbind VAO
            self.gl.bind_vertex_array(None);

            // Cache VAO and buffers for this sublayer
            buffer_cache.arc_vao = Some(vao);
            buffer_cache.arc_center_buffer = Some(center_buffer);
            buffer_cache.arc_radius_buffer = Some(radius_buffer);
            buffer_cache.arc_start_angle_buffer = Some(start_angle_buffer);
            buffer_cache.arc_sweep_angle_buffer = Some(sweep_angle_buffer);
            buffer_cache.arc_thickness_buffer = Some(thickness_buffer);
        }

        // Re-get immutable reference for rendering
        let layer = self.layers[layer_id].as_ref().unwrap();
        let buffer_cache = &layer.buffer_caches[sublayer_idx];

        // Bind cached VAO for this sublayer
        self.gl.bind_vertex_array(buffer_cache.arc_vao.as_ref());

        // Set uniforms (only these change per frame)
        if let Some(loc) = program.uniforms.get("transform") {
            self.gl
                .uniform_matrix3fv_with_f32_array(Some(loc), false, transform);
        }
        if let Some(loc) = program.uniforms.get("color") {
            self.gl.uniform4fv_with_f32_array(Some(loc), color);
        }

        // Draw
        self.gl
            .draw_arrays_instanced(TRIANGLES, 0, 6, instance_count as i32);

        // Unbind VAO to prevent state leakage
        self.gl.bind_vertex_array(None);

        Ok(())
    }

    /// Draw instanced thermals
    fn draw_instanced_thermals(
        &mut self,
        transform: &[f32; 9],
        color: &[f32; 4],
        layer_id: usize,
        sublayer_idx: usize,
    ) -> Result<(), JsValue> {
        // Check if data is empty (short-lived borrow)
        let instance_count = {
            let layer = self.layers[layer_id].as_ref().unwrap();
            layer.gerber_data[sublayer_idx].thermals.x.len()
        };
        if instance_count == 0 {
            return Ok(());
        }

        let program = &self.programs.thermal;
        self.gl.use_program(Some(&program.program));

        // Get mutable reference to buffer cache and immutable reference to data
        // Split borrowing: gerber_data and buffer_caches are different fields
        let layer = self.layers[layer_id]
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Layer not found"))?;
        let thermals = &layer.gerber_data[sublayer_idx].thermals;
        let buffer_cache = &mut layer.buffer_caches[sublayer_idx];

        // Check if VAO is cached for this sublayer
        if buffer_cache.thermal_vao.is_none() {
            // Create VAO
            let vao = self
                .gl
                .create_vertex_array()
                .ok_or_else(|| JsValue::from_str("Failed to create VAO"))?;
            self.gl.bind_vertex_array(Some(&vao));

            // Bind shared quad buffer for position attribute
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&self.quad_buffer));
            let position_loc = *program.attributes.get("position").unwrap();
            self.gl.enable_vertex_attrib_array(position_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(position_loc, 2, FLOAT, false, 0, 0);

            // Create instance buffers
            let centers = Self::interleave_xy(&thermals.x, &thermals.y);
            let center_buffer = Self::create_instance_buffer_2d(&self.gl, &centers, program, "center_instance", 1)?;
            let outer_diameter_buffer = Self::create_instance_buffer(&self.gl, &thermals.outer_diameter, program, "outer_diameter_instance", 1)?;
            let inner_diameter_buffer = Self::create_instance_buffer(&self.gl, &thermals.inner_diameter, program, "inner_diameter_instance", 1)?;
            let gap_thickness_buffer = Self::create_instance_buffer(&self.gl, &thermals.gap_thickness, program, "gap_thickness_instance", 1)?;
            let rotation_buffer = Self::create_instance_buffer(&self.gl, &thermals.rotation, program, "rotation_instance", 1)?;

            // Unbind VAO
            self.gl.bind_vertex_array(None);

            // Cache VAO and buffers for this sublayer
            buffer_cache.thermal_vao = Some(vao);
            buffer_cache.thermal_center_buffer = Some(center_buffer);
            buffer_cache.thermal_outer_diameter_buffer = Some(outer_diameter_buffer);
            buffer_cache.thermal_inner_diameter_buffer = Some(inner_diameter_buffer);
            buffer_cache.thermal_gap_thickness_buffer = Some(gap_thickness_buffer);
            buffer_cache.thermal_rotation_buffer = Some(rotation_buffer);
        }

        // Re-get immutable reference for rendering
        let layer = self.layers[layer_id].as_ref().unwrap();
        let buffer_cache = &layer.buffer_caches[sublayer_idx];

        // Bind cached VAO for this sublayer
        self.gl.bind_vertex_array(buffer_cache.thermal_vao.as_ref());

        // Set uniforms (only transform and color)
        if let Some(loc) = program.uniforms.get("transform") {
            self.gl
                .uniform_matrix3fv_with_f32_array(Some(loc), false, transform);
        }
        if let Some(loc) = program.uniforms.get("color") {
            self.gl.uniform4fv_with_f32_array(Some(loc), color);
        }

        // Draw
        self.gl
            .draw_arrays_instanced(TRIANGLES, 0, 6, instance_count as i32);

        // Unbind VAO to prevent state leakage
        self.gl.bind_vertex_array(None);

        Ok(())
    }

    /// Render all geometry from a specific user layer (with polarity sublayers)
    fn render_layer_geometry(
        &mut self,
        layer_id: usize,
        transform: &[f32; 9],
    ) -> Result<(), JsValue> {
        if layer_id >= self.layers.len() || self.layers[layer_id].is_none() {
            return Ok(());
        }

        let white_color = [1.0, 1.0, 1.0, 1.0];

        // Get sublayer count
        let sublayer_count = self.layers[layer_id].as_ref().unwrap().gerber_data.len();

        // Render each polarity sublayer with appropriate blending
        for sublayer_idx in 0..sublayer_count {
            // Check polarity: even index = positive, odd index = negative
            let is_negative = (sublayer_idx % 2) == 1;

            // Set polarity blending mode
            self.gl.enable(BLEND);
            if is_negative {
                // Negative polarity: erase alpha
                self.gl
                    .blend_func_separate(ZERO, ONE, ZERO, ONE_MINUS_SRC_ALPHA);
            } else {
                // Positive polarity: add alpha
                self.gl.blend_func_separate(ZERO, ONE, ONE, ONE);
            }
            self.gl.blend_equation(FUNC_ADD);

            // Render all shapes (empty checks done inside draw methods)
            self.draw_instanced_triangles(transform, &white_color, layer_id, sublayer_idx)?;
            self.draw_instanced_circles(transform, &white_color, layer_id, sublayer_idx)?;
            self.draw_instanced_arcs(transform, &white_color, layer_id, sublayer_idx)?;
            self.draw_instanced_thermals(transform, &white_color, layer_id, sublayer_idx)?;
        }

        self.gl.disable(BLEND);
        Ok(())
    }

    /// Set active layers and colors (stores state for FBO reuse)
    /// Render geometry to FBOs and composite to canvas
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        active_layer_ids: &[u32],
        color_data: &[f32],
        zoom_x: f32,
        _zoom_y: f32,
        offset_x: f32,
        offset_y: f32,
        alpha: f32,
    ) -> Result<(), JsValue> {
        // Update camera state
        self.update_camera(zoom_x, offset_x, offset_y);

        // Get canvas dimensions
        let (width, height) = self.get_canvas_size()?;

        // Get transform matrix
        let transform = self.camera.get_transform_matrix(width, height);

        // STEP 1: Render each active layer's geometry to its FBO (white)
        for &layer_id in active_layer_ids {
            let layer_idx = layer_id as usize;

            // Validate layer exists
            if layer_idx >= self.layers.len() || self.layers[layer_idx].is_none() {
                return Err(JsValue::from_str(&format!(
                    "Invalid layer_id: {}",
                    layer_id
                )));
            }

            // Get FBO for this layer
            let fbo = &self.layers[layer_idx].as_ref().unwrap().fbo;

            // Bind layer FBO
            self.gl
                .bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&fbo.framebuffer));
            self.gl.viewport(0, 0, width as i32, height as i32);

            // Clear layer FBO
            self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
            self.gl.clear(COLOR_BUFFER_BIT);

            // Render layer geometry (with polarity blending handled internally)
            self.render_layer_geometry(layer_idx, &transform)?;
        }

        // STEP 2: Composite FBOs to canvas
        self.composite_layers(active_layer_ids, color_data, alpha)?;

        Ok(())
    }

    fn composite_layers(
        &mut self,
        active_layer_ids: &[u32],
        color_data: &[f32],
        alpha: f32,
    ) -> Result<(), JsValue> {
        // Get canvas dimensions
        let (width, height) = self.get_canvas_size()?;

        // Bind canvas framebuffer
        self.gl
            .bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        self.gl.viewport(0, 0, width as i32, height as i32);

        // Clear canvas
        self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
        self.gl.clear(COLOR_BUFFER_BIT);

        // Setup additive blending for layer compositing (lighter blend mode)
        self.gl.enable(BLEND);
        self.gl.blend_func(ONE, ONE);
        self.gl.blend_equation(FUNC_ADD);

        // Render each active layer's FBO to canvas with its color/alpha
        for (color_index, &layer_id) in active_layer_ids.iter().enumerate() {
            let layer_idx = layer_id as usize;

            if let Some(layer) = &self.layers[layer_idx] {
                // Get RGB color from array (3 floats per layer)
                let color_offset = color_index * 3;
                if color_offset + 2 < color_data.len() {
                    let color = [
                        color_data[color_offset],
                        color_data[color_offset + 1],
                        color_data[color_offset + 2],
                        alpha, // Use provided alpha
                    ];
                    self.draw_fbo_texture(&layer.fbo.texture, &color)?;
                }
            }
        }

        self.gl.disable(BLEND);

        Ok(())
    }

    /// Get the combined boundary from all layers
    pub fn get_boundary(&self) -> Boundary {
        if self.layer_count == 0 {
            return Boundary::new(0.0, 0.0, 0.0, 0.0);
        }

        // Combine boundaries from all active layers
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for layer in self.layers.iter().flatten() {
            let b = &layer.boundary;
            min_x = min_x.min(b.min_x);
            max_x = max_x.max(b.max_x);
            min_y = min_y.min(b.min_y);
            max_y = max_y.max(b.max_y);
        }

        Boundary::new(min_x, max_x, min_y, max_y)
    }

    /// Resize framebuffers when canvas size changes
    pub fn resize(&mut self) -> Result<(), JsValue> {
        let (width, height) = self.get_canvas_size()?;

        // Recreate FBO for each active layer
        for layer in self.layers.iter_mut().flatten() {
            layer.fbo = Self::create_fbo(&self.gl, width, height)?;
        }

        Ok(())
    }
}
