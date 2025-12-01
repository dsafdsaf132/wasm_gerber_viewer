use crate::shape::{Arcs, Boundary, Circles, GerberData, Thermals, Triangles};
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
#[allow(dead_code)]
struct BufferCache {
    // Shared quad buffer for instanced rendering
    quad_buffer: WebGlBuffer,

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
    // Cached state for FBO reuse
    active_layer_ids: Vec<u32>, // Currently active layer IDs
    layer_colors: Vec<f32>,     // RGB colors (3 floats per layer, NO alpha)
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
            active_layer_ids: Vec::new(),
            layer_colors: Vec::new(),
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
                quad_buffer: self.quad_buffer.clone(),
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

        self.layers[layer_id] = None;
        self.layer_count -= 1;
        Ok(())
    }

    /// Clear all layers
    pub fn clear_all(&mut self) {
        self.layers.clear();
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
        triangles: &Triangles,
        transform: &[f32; 9],
        color: &[f32; 4],
        layer_id: usize,
        sublayer_idx: usize,
    ) -> Result<(), JsValue> {
        if triangles.indices.is_empty() {
            return Ok(());
        }

        let program = &self.programs.triangle;
        self.gl.use_program(Some(&program.program));

        // Get mutable reference to buffer cache
        let layer = self.layers[layer_id]
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Layer not found"))?;
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

        // Re-get immutable reference for rendering
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
            .draw_elements_with_i32(TRIANGLES, triangles.indices.len() as i32, UNSIGNED_INT, 0);

        // Unbind VAO to prevent state leakage
        self.gl.bind_vertex_array(None);

        Ok(())
    }

    /// Draw instanced circles
    fn draw_instanced_circles(
        &mut self,
        circles: &Circles,
        transform: &[f32; 9],
        color: &[f32; 4],
        layer_id: usize,
        sublayer_idx: usize,
    ) -> Result<(), JsValue> {
        let instance_count = circles.x.len();
        if instance_count == 0 {
            return Ok(());
        }

        let program = &self.programs.circle;
        self.gl.use_program(Some(&program.program));

        // Get mutable reference to buffer cache
        let layer = self.layers[layer_id]
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Layer not found"))?;
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

            // Create interleaved centers array [x1, y1, x2, y2, ...]
            let mut centers = Vec::with_capacity(instance_count * 2);
            for i in 0..instance_count {
                centers.push(circles.x[i]);
                centers.push(circles.y[i]);
            }

            // Create and bind center buffer
            let center_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create center buffer"))?;
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&center_buffer));
            unsafe {
                let array = Float32Array::view(&centers);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let center_loc = *program.attributes.get("center_instance").unwrap();
            self.gl.enable_vertex_attrib_array(center_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(center_loc, 2, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(center_loc, 1);

            // Create and bind radius buffer
            let radius_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create radius buffer"))?;
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&radius_buffer));
            unsafe {
                let array = Float32Array::view(&circles.radius);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let radius_loc = *program.attributes.get("radius_instance").unwrap();
            self.gl.enable_vertex_attrib_array(radius_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(radius_loc, 1, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(radius_loc, 1);

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
        arcs: &Arcs,
        transform: &[f32; 9],
        color: &[f32; 4],
        layer_id: usize,
        sublayer_idx: usize,
    ) -> Result<(), JsValue> {
        let instance_count = arcs.x.len();
        if instance_count == 0 {
            return Ok(());
        }

        let program = &self.programs.arc;
        self.gl.use_program(Some(&program.program));

        // Get mutable reference to buffer cache
        let layer = self.layers[layer_id]
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Layer not found"))?;
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

            // Create interleaved centers array [x1, y1, x2, y2, ...]
            let mut centers = Vec::with_capacity(instance_count * 2);
            for i in 0..instance_count {
                centers.push(arcs.x[i]);
                centers.push(arcs.y[i]);
            }

            // Create and bind center buffer
            let center_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create center buffer"))?;
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&center_buffer));
            unsafe {
                let array = Float32Array::view(&centers);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let center_loc = *program.attributes.get("center_instance").unwrap();
            self.gl.enable_vertex_attrib_array(center_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(center_loc, 2, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(center_loc, 1);

            // Create and bind radius buffer
            let radius_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create radius buffer"))?;
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&radius_buffer));
            unsafe {
                let array = Float32Array::view(&arcs.radius);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let radius_loc = *program.attributes.get("radius_instance").unwrap();
            self.gl.enable_vertex_attrib_array(radius_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(radius_loc, 1, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(radius_loc, 1);

            // Create and bind start angle buffer
            let start_angle_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create start angle buffer"))?;
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&start_angle_buffer));
            unsafe {
                let array = Float32Array::view(&arcs.start_angle);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let start_angle_loc = *program.attributes.get("startAngle_instance").unwrap();
            self.gl.enable_vertex_attrib_array(start_angle_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(start_angle_loc, 1, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(start_angle_loc, 1);

            // Create and bind sweep angle buffer
            let sweep_angle_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create sweep angle buffer"))?;
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&sweep_angle_buffer));
            unsafe {
                let array = Float32Array::view(&arcs.sweep_angle);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let sweep_angle_loc = *program.attributes.get("sweepAngle_instance").unwrap();
            self.gl.enable_vertex_attrib_array(sweep_angle_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(sweep_angle_loc, 1, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(sweep_angle_loc, 1);

            // Create and bind thickness buffer
            let thickness_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create thickness buffer"))?;
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&thickness_buffer));
            unsafe {
                let array = Float32Array::view(&arcs.thickness);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let thickness_loc = *program.attributes.get("thickness_instance").unwrap();
            self.gl.enable_vertex_attrib_array(thickness_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(thickness_loc, 1, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(thickness_loc, 1);

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
        thermals: &Thermals,
        transform: &[f32; 9],
        color: &[f32; 4],
        layer_id: usize,
        sublayer_idx: usize,
    ) -> Result<(), JsValue> {
        let instance_count = thermals.x.len();
        if instance_count == 0 {
            return Ok(());
        }

        let program = &self.programs.thermal;
        self.gl.use_program(Some(&program.program));

        // Get mutable reference to buffer cache
        let layer = self.layers[layer_id]
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Layer not found"))?;
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

            // Create interleaved centers array [x1, y1, x2, y2, ...]
            let mut centers = Vec::with_capacity(instance_count * 2);
            for i in 0..instance_count {
                centers.push(thermals.x[i]);
                centers.push(thermals.y[i]);
            }

            // Create and bind center buffer
            let center_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create center buffer"))?;
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&center_buffer));
            unsafe {
                let array = Float32Array::view(&centers);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let center_loc = *program.attributes.get("center_instance").unwrap();
            self.gl.enable_vertex_attrib_array(center_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(center_loc, 2, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(center_loc, 1);

            // Create and bind outer_diameter buffer
            let outer_diameter_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create outer_diameter buffer"))?;
            self.gl
                .bind_buffer(ARRAY_BUFFER, Some(&outer_diameter_buffer));
            unsafe {
                let array = Float32Array::view(&thermals.outer_diameter);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let outer_diameter_loc = *program.attributes.get("outer_diameter_instance").unwrap();
            self.gl.enable_vertex_attrib_array(outer_diameter_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(outer_diameter_loc, 1, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(outer_diameter_loc, 1);

            // Create and bind inner_diameter buffer
            let inner_diameter_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create inner_diameter buffer"))?;
            self.gl
                .bind_buffer(ARRAY_BUFFER, Some(&inner_diameter_buffer));
            unsafe {
                let array = Float32Array::view(&thermals.inner_diameter);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let inner_diameter_loc = *program.attributes.get("inner_diameter_instance").unwrap();
            self.gl.enable_vertex_attrib_array(inner_diameter_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(inner_diameter_loc, 1, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(inner_diameter_loc, 1);

            // Create and bind gap_thickness buffer
            let gap_thickness_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create gap_thickness buffer"))?;
            self.gl
                .bind_buffer(ARRAY_BUFFER, Some(&gap_thickness_buffer));
            unsafe {
                let array = Float32Array::view(&thermals.gap_thickness);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let gap_thickness_loc = *program.attributes.get("gap_thickness_instance").unwrap();
            self.gl.enable_vertex_attrib_array(gap_thickness_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(gap_thickness_loc, 1, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(gap_thickness_loc, 1);

            // Create and bind rotation buffer
            let rotation_buffer = self
                .gl
                .create_buffer()
                .ok_or_else(|| JsValue::from_str("Failed to create rotation buffer"))?;
            self.gl.bind_buffer(ARRAY_BUFFER, Some(&rotation_buffer));
            unsafe {
                let array = Float32Array::view(&thermals.rotation);
                self.gl
                    .buffer_data_with_array_buffer_view(ARRAY_BUFFER, &array, STATIC_DRAW);
            }
            let rotation_loc = *program.attributes.get("rotation_instance").unwrap();
            self.gl.enable_vertex_attrib_array(rotation_loc);
            self.gl
                .vertex_attrib_pointer_with_i32(rotation_loc, 1, FLOAT, false, 0, 0);
            self.gl.vertex_attrib_divisor(rotation_loc, 1);

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

        // Get layer metadata (need to clone to avoid borrow checker issues)
        let gerber_data_list = self.layers[layer_id].as_ref().unwrap().gerber_data.clone();

        // Render each polarity sublayer with appropriate blending
        for (sublayer_idx, gerber_data) in gerber_data_list.iter().enumerate() {
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

            // Clone sublayer data
            let triangles = gerber_data.triangles().clone();
            let circles = gerber_data.circles().clone();
            let arcs = gerber_data.arcs().clone();
            let thermals = gerber_data.thermals().clone();

            // Render triangles
            if !triangles.indices.is_empty() {
                self.draw_instanced_triangles(
                    &triangles,
                    transform,
                    &white_color,
                    layer_id,
                    sublayer_idx,
                )?;
            }

            // Render circles
            if !circles.x.is_empty() {
                self.draw_instanced_circles(
                    &circles,
                    transform,
                    &white_color,
                    layer_id,
                    sublayer_idx,
                )?;
            }

            // Render arcs
            if !arcs.x.is_empty() {
                self.draw_instanced_arcs(&arcs, transform, &white_color, layer_id, sublayer_idx)?;
            }

            // Render thermals
            if !thermals.x.is_empty() {
                self.draw_instanced_thermals(
                    &thermals,
                    transform,
                    &white_color,
                    layer_id,
                    sublayer_idx,
                )?;
            }
        }

        self.gl.disable(BLEND);
        Ok(())
    }

    /// Set active layers and colors (stores state for FBO reuse)
    pub fn set_active_layers(
        &mut self,
        active_layer_ids: &[u32],
        color_data: &[f32],
    ) -> Result<(), JsValue> {
        // Store active layer IDs and colors (RGB only, no alpha)
        self.active_layer_ids = active_layer_ids.to_vec();
        self.layer_colors = color_data.to_vec();
        Ok(())
    }

    /// Render geometry to FBOs and composite to canvas
    pub fn render(
        &mut self,
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

        // Clone active layer IDs to avoid borrow checker issues
        let active_ids = self.active_layer_ids.clone();

        // STEP 1: Render each active layer's geometry to its FBO (white)
        for &layer_id in &active_ids {
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
        self.composite(alpha)?;

        Ok(())
    }

    /// Composite FBOs to canvas with alpha (reuses existing FBO geometry)
    pub fn composite(&mut self, alpha: f32) -> Result<(), JsValue> {
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
        for (color_index, &layer_id) in self.active_layer_ids.iter().enumerate() {
            let layer_idx = layer_id as usize;

            if let Some(layer) = &self.layers[layer_idx] {
                // Get RGB color from array (3 floats per layer)
                let color_offset = color_index * 3;
                if color_offset + 2 < self.layer_colors.len() {
                    let color = [
                        self.layer_colors[color_offset],
                        self.layer_colors[color_offset + 1],
                        self.layer_colors[color_offset + 2],
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
