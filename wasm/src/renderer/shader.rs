use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader, WebGlUniformLocation};

// WebGL constants
pub const COLOR_BUFFER_BIT: u32 = WebGl2RenderingContext::COLOR_BUFFER_BIT;
pub const TRIANGLES: u32 = WebGl2RenderingContext::TRIANGLES;
pub const FLOAT: u32 = WebGl2RenderingContext::FLOAT;
pub const UNSIGNED_INT: u32 = WebGl2RenderingContext::UNSIGNED_INT;
pub const ARRAY_BUFFER: u32 = WebGl2RenderingContext::ARRAY_BUFFER;
pub const ELEMENT_ARRAY_BUFFER: u32 = WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER;
pub const STATIC_DRAW: u32 = WebGl2RenderingContext::STATIC_DRAW;
pub const VERTEX_SHADER: u32 = WebGl2RenderingContext::VERTEX_SHADER;
pub const FRAGMENT_SHADER: u32 = WebGl2RenderingContext::FRAGMENT_SHADER;
pub const BLEND: u32 = WebGl2RenderingContext::BLEND;
pub const ONE_MINUS_SRC_ALPHA: u32 = WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA;
pub const ONE: u32 = WebGl2RenderingContext::ONE;
pub const FUNC_ADD: u32 = WebGl2RenderingContext::FUNC_ADD;
pub const ZERO: u32 = WebGl2RenderingContext::ZERO;

// Shader sources
pub const TRIANGLE_VERTEX_SHADER: &str = r#"#version 300 es
in vec2 position;
in vec2 hole_center_instance;
in float hole_radius_instance;
uniform mat3 transform;
out lowp vec2 vPosition;
out lowp vec2 vHoleCenter;
out lowp float vHoleRadius;
void main() {
    vec3 transformed = transform * vec3(position, 1.0);
    gl_Position = vec4(transformed.xy, 0.0, 1.0);
    vPosition = position;
    vHoleCenter = hole_center_instance;
    vHoleRadius = hole_radius_instance;
}
"#;

pub const TRIANGLE_FRAGMENT_SHADER: &str = r#"#version 300 es
precision lowp float;
in lowp vec2 vPosition;
in lowp vec2 vHoleCenter;
in lowp float vHoleRadius;
uniform vec4 color;
out vec4 fragColor;
void main() {
    if (vHoleRadius > 0.0) {
        vec2 diff = vPosition - vHoleCenter;
        if (dot(diff, diff) < vHoleRadius * vHoleRadius) discard;
    }
    fragColor = color;
}
"#;

pub const CIRCLE_VERTEX_SHADER: &str = r#"#version 300 es
in vec2 position;
in vec2 center_instance;
in float radius_instance;
in vec2 hole_center_instance;
in float hole_radius_instance;
uniform mat3 transform;
out lowp vec2 vPosition;
out lowp vec2 vHoleCenter;
out lowp float vHoleRadius;
void main() {
    vec2 scaledPos = position * radius_instance + center_instance;
    vec3 transformed = transform * vec3(scaledPos, 1.0);
    gl_Position = vec4(transformed.xy, 0.0, 1.0);
    vPosition = position;
    vHoleCenter = (hole_center_instance - center_instance) / radius_instance;
    vHoleRadius = hole_radius_instance / radius_instance;
}
"#;

pub const CIRCLE_FRAGMENT_SHADER: &str = r#"#version 300 es
precision lowp float;
in lowp vec2 vPosition;
in lowp vec2 vHoleCenter;
in lowp float vHoleRadius;
uniform vec4 color;
out vec4 fragColor;
void main() {
    float dist = dot(vPosition, vPosition);
    if (dist > 1.0) discard;
    if (vHoleRadius > 0.0) {
        vec2 diff = vPosition - vHoleCenter;
        if (dot(diff, diff) < vHoleRadius * vHoleRadius) discard;
    }
    fragColor = color;
}
"#;

pub const ARC_VERTEX_SHADER: &str = r#"#version 300 es
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

pub const ARC_FRAGMENT_SHADER: &str = r#"#version 300 es
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

pub const THERMAL_VERTEX_SHADER: &str = r#"#version 300 es
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

pub const THERMAL_FRAGMENT_SHADER: &str = r#"#version 300 es
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

pub const TEXTURE_VERTEX_SHADER: &str = r#"#version 300 es
in vec2 position;
out vec2 v_uv;
void main() {
    v_uv = position * 0.5 + 0.5;
    gl_Position = vec4(position, 0.0, 1.0);
}
"#;

pub const TEXTURE_FRAGMENT_SHADER: &str = r#"#version 300 es
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

/// Shader program with uniform locations
pub struct ShaderProgram {
    pub program: WebGlProgram,
    pub uniforms: HashMap<String, WebGlUniformLocation>,
    pub attributes: HashMap<String, u32>,
}

/// All shader programs
pub struct ShaderPrograms {
    pub triangle: ShaderProgram,
    pub circle: ShaderProgram,
    pub arc: ShaderProgram,
    pub thermal: ShaderProgram,
    pub texture: ShaderProgram,
}

impl ShaderPrograms {
    /// Compile all shader programs
    pub fn new(gl: &WebGl2RenderingContext) -> Result<ShaderPrograms, JsValue> {
        let triangle = compile_program(
            gl,
            TRIANGLE_VERTEX_SHADER,
            TRIANGLE_FRAGMENT_SHADER,
            &["position", "hole_center_instance", "hole_radius_instance"],
            &["transform", "color"],
        )?;

        let circle = compile_program(
            gl,
            CIRCLE_VERTEX_SHADER,
            CIRCLE_FRAGMENT_SHADER,
            &["position", "center_instance", "radius_instance", "hole_center_instance", "hole_radius_instance"],
            &["transform", "color"],
        )?;

        let arc = compile_program(
            gl,
            ARC_VERTEX_SHADER,
            ARC_FRAGMENT_SHADER,
            &[
                "position",
                "center_instance",
                "radius_instance",
                "startAngle_instance",
                "sweepAngle_instance",
                "thickness_instance",
            ],
            &["transform", "color"],
        )?;

        let thermal = compile_program(
            gl,
            THERMAL_VERTEX_SHADER,
            THERMAL_FRAGMENT_SHADER,
            &[
                "position",
                "center_instance",
                "outer_diameter_instance",
                "inner_diameter_instance",
                "gap_thickness_instance",
                "rotation_instance",
            ],
            &["transform", "color"],
        )?;

        let texture = compile_program(
            gl,
            TEXTURE_VERTEX_SHADER,
            TEXTURE_FRAGMENT_SHADER,
            &["position"],
            &["u_texture", "u_color"],
        )?;

        Ok(ShaderPrograms {
            triangle,
            circle,
            arc,
            thermal,
            texture,
        })
    }
}

/// Compile a shader program
fn compile_program(
    gl: &WebGl2RenderingContext,
    vertex_src: &str,
    fragment_src: &str,
    attributes: &[&str],
    uniforms: &[&str],
) -> Result<ShaderProgram, JsValue> {
    let vert_shader = compile_shader(gl, VERTEX_SHADER, vertex_src)?;
    let frag_shader = compile_shader(gl, FRAGMENT_SHADER, fragment_src)?;

    let program = gl
        .create_program()
        .ok_or_else(|| JsValue::from_str("Unable to create shader program"))?;

    gl.attach_shader(&program, &vert_shader);
    gl.attach_shader(&program, &frag_shader);

    for (i, attr_name) in attributes.iter().enumerate() {
        gl.bind_attrib_location(&program, i as u32, attr_name);
    }

    gl.link_program(&program);

    if !gl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        let error = gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| "Unknown error".to_string());
        return Err(JsValue::from_str(&format!("Shader link error: {}", error)));
    }

    gl.delete_shader(Some(&vert_shader));
    gl.delete_shader(Some(&frag_shader));

    let mut attr_map = HashMap::new();
    for (i, attr_name) in attributes.iter().enumerate() {
        attr_map.insert(attr_name.to_string(), i as u32);
    }

    let mut uniform_map = HashMap::new();
    for uniform_name in uniforms.iter() {
        if let Some(loc) = gl.get_uniform_location(&program, uniform_name) {
            uniform_map.insert(uniform_name.to_string(), loc);
        }
    }

    Ok(ShaderProgram {
        program,
        uniforms: uniform_map,
        attributes: attr_map,
    })
}

/// Compile a single shader
fn compile_shader(
    gl: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, JsValue> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| JsValue::from_str("Unable to create shader object"))?;

    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if !gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        let error = gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown error".to_string());
        gl.delete_shader(Some(&shader));
        return Err(JsValue::from_str(&format!("Shader compile error: {}", error)));
    }

    Ok(shader)
}
