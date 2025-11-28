use crate::shape::{Arcs, Boundary, Circles, GerberData, Thermals, Triangles};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use i_triangle::float::triangulatable::Triangulatable;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Polarity - Dark (positive) or Clear (negative)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Polarity {
    Positive, // Dark - add geometry
    Negative, // Clear - remove geometry
}

/// Basic primitive shape - created directly by parser
#[derive(Clone, Debug)]
pub enum Primitive {
    Triangle {
        vertices: Vec<[f32; 2]>,
        exposure: f32, // 1.0 = positive, 0.0 = negative
    },
    Circle {
        x: f32,
        y: f32,
        radius: f32,
        exposure: f32, // 1.0 = positive, 0.0 = negative
    },
    Arc {
        x: f32,
        y: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        thickness: f32,
        exposure: f32, // 1.0 = positive, 0.0 = negative
    },
    Thermal {
        x: f32,
        y: f32,
        outer_diameter: f32,
        inner_diameter: f32,
        gap_thickness: f32,
        rotation: f32,
        exposure: f32, // 1.0 = positive, 0.0 = negative
    },
}

impl Primitive {
    /// Rotate primitive by given angle around center
    pub fn rotate(&mut self, angle: f32, center_x: f32, center_y: f32) {
        match self {
            Primitive::Triangle { vertices, .. } => {
                for vertex in vertices.iter_mut() {
                    rotate_point(vertex, angle, center_x, center_y);
                }
            }
            Primitive::Circle { x, y, .. } => {
                let mut point = [*x, *y];
                rotate_point(&mut point, angle, center_x, center_y);
                *x = point[0];
                *y = point[1];
            }
            Primitive::Arc {
                x,
                y,
                start_angle,
                end_angle,
                ..
            } => {
                let mut point = [*x, *y];
                rotate_point(&mut point, angle, center_x, center_y);
                *x = point[0];
                *y = point[1];
                *start_angle += angle;
                *end_angle += angle;
            }
            Primitive::Thermal {
                x,
                y,
                rotation: thermal_rotation,
                ..
            } => {
                let mut point = [*x, *y];
                rotate_point(&mut point, angle, center_x, center_y);
                *x = point[0];
                *y = point[1];
                *thermal_rotation += angle;
            }
        }
    }
}

/// Rotate point around given center
#[inline]
fn rotate_point(point: &mut [f32; 2], angle: f32, center_x: f32, center_y: f32) {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let x = point[0] - center_x;
    let y = point[1] - center_y;
    point[0] = center_x + x * cos_a - y * sin_a;
    point[1] = center_y + x * sin_a + y * cos_a;
}

/// Evaluate expression - $5-$3, $1/2, 2X$3, etc.
/// X is interpreted as multiply, $variables are evaluated in real-time
fn evaluate_expression(expr: &str, variables: &HashMap<String, f32>) -> Result<f32, String> {
    let expr = expr.trim();

    // Replace X with * (X means multiply in Gerber format)
    let expr = expr.replace('X', "*");

    // Use simple expression calculator (pass variable map)
    calculate_simple_expression(&expr, variables)
}

/// Simple arithmetic expression calculator: supports +, -, *, /
/// Priority: * / > + -
/// Tokens are numbers, $variables, or operators
fn calculate_simple_expression(
    expr: &str,
    variables: &HashMap<String, f32>,
) -> Result<f32, String> {
    let expr = expr.trim();

    if expr.is_empty() {
        return Err("Empty expression".to_string());
    }

    // Tokenize: separate numbers, $variables, operators
    let tokens = tokenize(expr)?;

    if tokens.is_empty() {
        return Err("No tokens".to_string());
    }

    // Process multiplication and division first (pass variable map)
    let tokens = apply_multiplication_division(tokens, variables)?;

    // Process addition and subtraction (pass variable map)
    apply_addition_subtraction(tokens, variables)
}

/// Split expression into tokens - recognize $variables as tokens, handle negative numbers
fn tokenize(expr: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut chars = expr.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_ascii_whitespace() {
            if !current_token.is_empty() {
                tokens.push(current_token.clone());
                current_token.clear();
            }
            continue;
        }

        // Handle negative numbers: - or + followed by number or $variable
        // If no previous token or previous token is operator/bracket → interpret as sign
        if (ch == '-' || ch == '+') && (tokens.is_empty() || is_operator_or_bracket(tokens.last()))
        {
            current_token.push(ch);

            // After sign, $variable or number can follow
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '$' {
                    // Case like -$1
                    tokens.push(current_token.clone()); // Save "-"
                    current_token.clear();

                    // Process $variable
                    current_token.push(chars.next().unwrap()); // $
                    while let Some(&digit_ch) = chars.peek() {
                        if digit_ch.is_ascii_digit() {
                            current_token.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(current_token.clone());
                    current_token.clear();
                } else if next_ch.is_ascii_digit() || next_ch == '.' {
                    // Read number after sign
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_ascii_digit() || next_ch == '.' {
                            current_token.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(current_token.clone());
                    current_token.clear();
                } else {
                    // If only sign without following value, treat as operator
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    tokens.push(ch.to_string());
                }
            }
        }
        // Process $variable: $1, $2, $5, etc.
        else if ch == '$' {
            if !current_token.is_empty() {
                tokens.push(current_token.clone());
                current_token.clear();
            }

            current_token.push(ch);
            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_ascii_digit() {
                    current_token.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            tokens.push(current_token.clone());
            current_token.clear();
        }
        // Regular number
        else if ch.is_ascii_digit() || ch == '.' {
            current_token.push(ch);
        }
        // Operator
        else if "+-*/()".contains(ch) {
            if !current_token.is_empty() {
                tokens.push(current_token.clone());
                current_token.clear();
            }
            tokens.push(ch.to_string());
        } else {
            return Err(format!("Invalid character: {}", ch));
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    Ok(tokens)
}

/// Check if token is operator or bracket
fn is_operator_or_bracket(token: Option<&String>) -> bool {
    match token {
        Some(t) => matches!(t.as_str(), "+" | "-" | "*" | "/" | "("),
        None => true, // True if before first token
    }
}

/// Convert token to value (number or $variable)
fn token_to_value(token: &str, variables: &HashMap<String, f32>) -> Result<f32, String> {
    if token.starts_with('$') {
        // Variable reference
        variables
            .get(token)
            .copied()
            .ok_or_else(|| format!("Undefined variable: {}", token))
    } else {
        // Number
        token
            .parse::<f32>()
            .map_err(|_| format!("Invalid number: {}", token))
    }
}

/// Process * and / operations
fn apply_multiplication_division(
    tokens: Vec<String>,
    variables: &HashMap<String, f32>,
) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        if i + 2 < tokens.len() && ("*".to_string() == tokens[i + 1] || "/" == tokens[i + 1]) {
            let left = token_to_value(&tokens[i], variables)?;
            let op = &tokens[i + 1];
            let right = token_to_value(&tokens[i + 2], variables)?;

            let value = if op == "*" {
                left * right
            } else {
                if right == 0.0 {
                    return Err("Division by zero".to_string());
                }
                left / right
            };

            result.push(value.to_string());
            i += 3;
        } else {
            result.push(tokens[i].clone());
            i += 1;
        }
    }

    Ok(result)
}

/// Process + and - operations
fn apply_addition_subtraction(
    tokens: Vec<String>,
    variables: &HashMap<String, f32>,
) -> Result<f32, String> {
    if tokens.is_empty() {
        return Err("No tokens to process".to_string());
    }

    let mut i;
    let mut result;

    // Handle case where first token is operator (e.g., -$1, +$2)
    if tokens[0] == "-" || tokens[0] == "+" {
        let sign = if tokens[0] == "-" { -1.0 } else { 1.0 };
        if tokens.len() < 2 {
            return Err("Operator without operand".to_string());
        }
        result = sign * token_to_value(&tokens[1], variables)?;
        i = 2;
    } else {
        result = token_to_value(&tokens[0], variables)?;
        i = 1;
    }

    // Process remaining operations
    while i < tokens.len() {
        if i + 1 < tokens.len() {
            let op = &tokens[i];
            let right = token_to_value(&tokens[i + 1], variables)?;

            if op == "+" {
                result += right;
            } else if op == "-" {
                result -= right;
            } else {
                return Err(format!("Unexpected operator: {}", op));
            }

            i += 2;
        } else {
            break;
        }
    }

    Ok(result)
}

/// Triangulate outline into triangles
fn triangulate_outline(vertices: &[[f32; 2]], exposure: f32) -> Result<Vec<Primitive>, String> {
    if vertices.len() < 3 {
        return Err("Not enough vertices".to_string());
    }

    // Use i_triangle library
    let shape = [vertices.to_vec()];
    let triangulation = shape.triangulate();
    {
        let tri_result = triangulation.to_triangulation::<u32>();
        let mut triangles = Vec::new();

        // Group triangles in sets of 3 to create Primitive::Triangle
        for i in (0..tri_result.indices.len()).step_by(3) {
            if i + 2 < tri_result.indices.len() {
                let i0 = tri_result.indices[i] as usize;
                let i1 = tri_result.indices[i + 1] as usize;
                let i2 = tri_result.indices[i + 2] as usize;

                if i0 < tri_result.points.len()
                    && i1 < tri_result.points.len()
                    && i2 < tri_result.points.len()
                {
                    triangles.push(Primitive::Triangle {
                        vertices: vec![
                            tri_result.points[i0],
                            tri_result.points[i1],
                            tri_result.points[i2],
                        ],
                        exposure,
                    });
                }
            }
        }

        Ok(triangles)
    }
}

/// Split line into two triangles (including width)
fn line_to_triangles(
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    width: f32,
    exposure: f32,
) -> Vec<Primitive> {
    // Line direction vector
    let dx = end_x - start_x;
    let dy = end_y - start_y;
    let len = (dx * dx + dy * dy).sqrt();

    if len == 0.0 {
        return Vec::new();
    }

    // Perpendicular vector (width direction)
    let half_width = width / 2.0;
    let perp_x = -dy / len * half_width;
    let perp_y = dx / len * half_width;

    // 4 vertices on both sides of the line
    let v1 = [start_x + perp_x, start_y + perp_y];
    let v2 = [start_x - perp_x, start_y - perp_y];
    let v3 = [end_x + perp_x, end_y + perp_y];
    let v4 = [end_x - perp_x, end_y - perp_y];

    // Two triangles: (v1, v2, v3), (v2, v4, v3)
    vec![
        Primitive::Triangle {
            vertices: vec![v1, v2, v3],
            exposure,
        },
        Primitive::Triangle {
            vertices: vec![v2, v4, v3],
            exposure,
        },
    ]
}

/// Convert Gerber macro primitive directly to iOverlay Shape format
/// Returns Shape: Vec<Contour> where first contour is outer boundary (CCW),
/// subsequent contours are holes (CW)
///
/// Shape format: Vec<Vec<[f32; 2]>>
///   - Contour 0: Outer boundary (counterclockwise)
///   - Contour 1..N: Holes (clockwise)
#[allow(dead_code)]
fn macro_primitive_to_shape(
    code: u32,
    params: &[f32],
    _exposure: f32,
) -> Option<Vec<Vec<[f32; 2]>>> {
    match code {
        1 => {
            // Circle: exposure, diameter, centerX, centerY, [rotation]
            if params.len() < 3 {
                return None;
            }
            let diameter = params[0];
            let center_x = params[1];
            let center_y = params[2];
            let radius = diameter / 2.0;

            // 36-sided polygon (10 degree increments)
            let segments = 36;
            let mut vertices = Vec::with_capacity(segments);
            for i in 0..segments {
                let angle = (i as f32) * (2.0 * std::f32::consts::PI / segments as f32);
                vertices.push([
                    center_x + radius * angle.cos(),
                    center_y + radius * angle.sin(),
                ]);
            }

            // Single contour (outer boundary only)
            Some(vec![vertices])
        }

        4 => {
            // Outline: exposure, vertices, x1, y1, x2, y2, ..., xn, yn, [rotation]
            if params.len() < 3 {
                return None;
            }
            let num_vertices = params[0] as usize;
            if params.len() < 1 + num_vertices * 2 {
                return None;
            }

            let mut vertices = Vec::with_capacity(num_vertices);
            for i in 0..num_vertices {
                let x = params[1 + i * 2];
                let y = params[1 + i * 2 + 1];
                vertices.push([x, y]);
            }

            // Single contour (outline is already a closed polygon)
            Some(vec![vertices])
        }

        5 => {
            // Polygon: exposure, vertices, centerX, centerY, diameter, [rotation]
            if params.len() < 4 {
                return None;
            }
            let num_vertices = params[0] as usize;
            let center_x = params[1];
            let center_y = params[2];
            let diameter = params[3];
            let radius = diameter / 2.0;

            let mut vertices = Vec::with_capacity(num_vertices);
            let angle_step = 2.0 * std::f32::consts::PI / num_vertices as f32;

            for i in 0..num_vertices {
                let angle = angle_step * i as f32;
                vertices.push([
                    center_x + radius * angle.cos(),
                    center_y + radius * angle.sin(),
                ]);
            }

            // Single contour (regular polygon)
            Some(vec![vertices])
        }

        7 => {
            // Thermal: centerX, centerY, outerDiameter, innerDiameter, gapThickness, [rotation]
            // Note: Thermals don't have exposure parameter (always positive)
            if params.len() < 5 {
                return None;
            }
            let center_x = params[0];
            let center_y = params[1];
            let outer_diameter = params[2];
            let inner_diameter = params[3];
            let gap_thickness = params[4];
            let rotation = if params.len() > 5 {
                params[5] * (std::f32::consts::PI / 180.0) // degrees to radians
            } else {
                0.0
            };

            let outer_radius = outer_diameter / 2.0;
            let inner_radius = inner_diameter / 2.0;
            let half_gap = gap_thickness / 2.0;

            // Outer circle (36-sided polygon, counterclockwise)
            let segments = 36;
            let mut outer_contour = Vec::with_capacity(segments);
            for i in 0..segments {
                let angle = (i as f32) * (2.0 * std::f32::consts::PI / segments as f32);
                let x = center_x + outer_radius * angle.cos();
                let y = center_y + outer_radius * angle.sin();

                // Apply rotation
                if rotation != 0.0 {
                    let dx = x - center_x;
                    let dy = y - center_y;
                    outer_contour.push([
                        center_x + dx * rotation.cos() - dy * rotation.sin(),
                        center_y + dx * rotation.sin() + dy * rotation.cos(),
                    ]);
                } else {
                    outer_contour.push([x, y]);
                }
            }

            // Inner circle (36-sided polygon, clockwise for hole)
            let mut inner_contour = Vec::with_capacity(segments);
            for i in (0..segments).rev() {
                let angle = (i as f32) * (2.0 * std::f32::consts::PI / segments as f32);
                let x = center_x + inner_radius * angle.cos();
                let y = center_y + inner_radius * angle.sin();

                // Apply rotation
                if rotation != 0.0 {
                    let dx = x - center_x;
                    let dy = y - center_y;
                    inner_contour.push([
                        center_x + dx * rotation.cos() - dy * rotation.sin(),
                        center_y + dx * rotation.sin() + dy * rotation.cos(),
                    ]);
                } else {
                    inner_contour.push([x, y]);
                }
            }

            // Four gap rectangles (clockwise for holes)
            // Gap 1: vertical gap (top-bottom)
            let gap1 = vec![
                [center_x - half_gap, center_y + outer_radius],
                [center_x - half_gap, center_y - outer_radius],
                [center_x + half_gap, center_y - outer_radius],
                [center_x + half_gap, center_y + outer_radius],
            ];

            // Gap 2: horizontal gap (left-right)
            let gap2 = vec![
                [center_x - outer_radius, center_y - half_gap],
                [center_x - outer_radius, center_y + half_gap],
                [center_x + outer_radius, center_y + half_gap],
                [center_x + outer_radius, center_y - half_gap],
            ];

            // Apply rotation to gaps if needed
            let apply_rotation_to_contour = |contour: Vec<[f32; 2]>| -> Vec<[f32; 2]> {
                if rotation != 0.0 {
                    contour
                        .iter()
                        .map(|&[x, y]| {
                            let dx = x - center_x;
                            let dy = y - center_y;
                            [
                                center_x + dx * rotation.cos() - dy * rotation.sin(),
                                center_y + dx * rotation.sin() + dy * rotation.cos(),
                            ]
                        })
                        .collect()
                } else {
                    contour
                }
            };

            let gap1_rotated = apply_rotation_to_contour(gap1);
            let gap2_rotated = apply_rotation_to_contour(gap2);

            // Return: outer + inner + 2 gaps
            Some(vec![
                outer_contour, // Contour 0: outer (CCW)
                inner_contour, // Contour 1: inner hole (CW)
                gap1_rotated,  // Contour 2: vertical gap (CW)
                gap2_rotated,  // Contour 3: horizontal gap (CW)
            ])
        }

        20 => {
            // Vector Line: exposure, width, startX, startY, endX, endY, [rotation]
            if params.len() < 5 {
                return None;
            }
            let width = params[0];
            let start_x = params[1];
            let start_y = params[2];
            let end_x = params[3];
            let end_y = params[4];

            // Calculate perpendicular offset for line width
            let dx = end_x - start_x;
            let dy = end_y - start_y;
            let length = (dx * dx + dy * dy).sqrt();

            if length < 1e-6 {
                return None;
            }

            let half_width = width / 2.0;
            let perp_x = -dy / length * half_width;
            let perp_y = dx / length * half_width;

            // Rectangle vertices (counterclockwise)
            let vertices = vec![
                [start_x + perp_x, start_y + perp_y],
                [end_x + perp_x, end_y + perp_y],
                [end_x - perp_x, end_y - perp_y],
                [start_x - perp_x, start_y - perp_y],
            ];

            Some(vec![vertices])
        }

        21 => {
            // Center Line: exposure, width, height, centerX, centerY, [rotation]
            if params.len() < 4 {
                return None;
            }
            let width = params[0];
            let height = params[1];
            let center_x = params[2];
            let center_y = params[3];

            let half_width = width / 2.0;
            let half_height = height / 2.0;

            // Rectangle vertices (counterclockwise)
            let vertices = vec![
                [center_x - half_width, center_y - half_height],
                [center_x + half_width, center_y - half_height],
                [center_x + half_width, center_y + half_height],
                [center_x - half_width, center_y + half_height],
            ];

            Some(vec![vertices])
        }

        _ => None,
    }
}

/// Convert a primitive to a polygon (outer boundary as Vec<[f32; 2]>)
fn primitive_to_polygon(primitive: &Primitive) -> Vec<[f32; 2]> {
    match primitive {
        Primitive::Circle { x, y, radius, .. } => {
            // 36-sided polygon (10 degree increments)
            let segments = 36;
            let mut vertices = Vec::with_capacity(segments);
            for i in 0..segments {
                let angle = (i as f32) * (2.0 * std::f32::consts::PI / segments as f32);
                vertices.push([x + radius * angle.cos(), y + radius * angle.sin()]);
            }
            vertices
        }

        Primitive::Triangle { vertices, .. } => {
            // Already a polygon
            vertices.clone()
        }

        Primitive::Arc {
            x,
            y,
            radius,
            start_angle,
            end_angle,
            ..
        } => {
            // Subdivide arc into 10-degree segments
            let start_rad = start_angle.to_radians();
            let end_rad = end_angle.to_radians();
            let mut sweep = end_rad - start_rad;
            if sweep < 0.0 {
                sweep += 2.0 * std::f32::consts::PI;
            }

            let segment_angle = 10.0_f32.to_radians();
            let num_segments = (sweep / segment_angle).ceil() as usize;

            let mut vertices = Vec::with_capacity(num_segments + 1);
            for i in 0..=num_segments {
                let t = (i as f32) / (num_segments as f32);
                let angle = start_rad + sweep * t;
                vertices.push([x + radius * angle.cos(), y + radius * angle.sin()]);
            }
            vertices
        }

        Primitive::Thermal {
            x,
            y,
            outer_diameter,
            ..
        } => {
            // Convert thermal to polygon
            // For now, simplified to outer circle (can be refined later)
            let outer_radius = outer_diameter / 2.0;
            let segments = 36;

            let mut vertices = Vec::with_capacity(segments);
            for i in 0..segments {
                let angle = (i as f32) * (2.0 * std::f32::consts::PI / segments as f32);
                vertices.push([
                    x + outer_radius * angle.cos(),
                    y + outer_radius * angle.sin(),
                ]);
            }
            vertices
        }
    }
}

/// Apply sequential boolean operations to primitives with iOverlay
/// Returns triangulated result as a single Primitive::Triangle
#[allow(dead_code)]
fn apply_boolean_operations(primitives: &[Primitive]) -> Option<Primitive> {
    if primitives.is_empty() {
        return None;
    }

    // Convert all primitives to polygons with their exposure values
    let polygons: Vec<(Vec<[f32; 2]>, f32)> = primitives
        .iter()
        .map(|p| {
            let poly = primitive_to_polygon(p);
            let exposure = match p {
                Primitive::Circle { exposure, .. } => *exposure,
                Primitive::Triangle { exposure, .. } => *exposure,
                Primitive::Arc { exposure, .. } => *exposure,
                Primitive::Thermal { exposure, .. } => *exposure,
            };
            (poly, exposure)
        })
        .collect();

    // Start with first polygon as base
    let (first_poly, first_exposure) = &polygons[0];

    // If first polygon is negative, we can't start with it
    if *first_exposure < 0.5 {
        return None;
    }

    // Start with first positive polygon
    let mut result_shapes: Vec<Vec<Vec<[f32; 2]>>> = vec![vec![first_poly.clone()]];

    // Sequentially apply boolean operations
    for (poly, exposure) in polygons.iter().skip(1) {
        if poly.is_empty() {
            continue;
        }

        let clip_shape = vec![poly.clone()];

        if *exposure > 0.5 {
            // Positive: UNION with existing result
            result_shapes =
                result_shapes.overlay(&clip_shape, OverlayRule::Union, FillRule::NonZero);
        } else {
            // Negative: DIFFERENCE from existing result
            result_shapes =
                result_shapes.overlay(&clip_shape, OverlayRule::Difference, FillRule::NonZero);
        }

        if result_shapes.is_empty() {
            return None;
        }
    }

    // Flatten result back to a single polygon
    // iOverlay returns Vec<Shape> where Shape is Vec<Contour> and Contour is Vec<Point>
    if result_shapes.is_empty() {
        return None;
    }

    // Take the first shape and concatenate all its contours
    let flattened_polygon: Vec<[f32; 2]> = result_shapes
        .iter()
        .flat_map(|shape| shape.iter().flat_map(|contour| contour.iter().copied()))
        .collect();

    if flattened_polygon.is_empty() {
        return None;
    }

    // Triangulate using i_triangle
    match triangulate_outline(&flattened_polygon, 1.0) {
        Ok(triangles) => {
            // Combine all triangles into a single Triangle primitive
            let all_vertices: Vec<[f32; 2]> = triangles
                .iter()
                .flat_map(|tri| match tri {
                    Primitive::Triangle { vertices, .. } => vertices.clone(),
                    _ => Vec::new(),
                })
                .collect();

            if all_vertices.is_empty() {
                None
            } else {
                Some(Primitive::Triangle {
                    vertices: all_vertices,
                    exposure: 1.0, // Final result is always positive
                })
            }
        }
        Err(_) => None,
    }
}

/// Apply sequential boolean operations to shapes (new version using Shape format)
/// Input: Vec<(Shape, exposure)> where Shape is Vec<Contour> and Contour is Vec<Point>
/// Returns: Vec<Primitive::Triangle> with all triangulated results
fn apply_boolean_operations_v2(shapes: &[(Vec<Vec<[f32; 2]>>, f32)]) -> Vec<Primitive> {
    if shapes.is_empty() {
        return Vec::new();
    }

    // Find first positive shape
    let first_positive_idx = shapes.iter().position(|(_, exposure)| *exposure > 0.5);

    let first_idx = match first_positive_idx {
        Some(idx) => idx,
        None => return Vec::new(), // No positive shapes to start with
    };

    // Start with first positive shape
    let mut result_shapes: Vec<Vec<Vec<[f32; 2]>>> = vec![shapes[first_idx].0.clone()];

    // Apply boolean operations sequentially
    for i in 0..shapes.len() {
        if i == first_idx {
            continue; // Skip the first shape we already added
        }

        let (shape, exposure) = &shapes[i];

        if *exposure > 0.5 {
            // Positive: UNION
            result_shapes =
                result_shapes.overlay(&vec![shape.clone()], OverlayRule::Union, FillRule::NonZero);
        } else {
            // Negative: DIFFERENCE
            result_shapes = result_shapes.overlay(
                &vec![shape.clone()],
                OverlayRule::Difference,
                FillRule::NonZero,
            );
        }

        if result_shapes.is_empty() {
            return Vec::new();
        }
    }

    // Triangulate all result shapes (preserving holes)
    let mut all_primitives = Vec::new();

    for shape in result_shapes {
        // shape is Vec<Contour> where first contour is outer, rest are holes
        if shape.is_empty() {
            continue;
        }

        // Use i_triangle to triangulate shape with holes
        // i_triangle expects: outer boundary + holes
        let triangulated = triangulate_shape_with_holes(&shape, 1.0);

        match triangulated {
            Ok(primitives) => {
                all_primitives.extend(primitives);
            }
            Err(_) => {
                // If triangulation fails, skip this shape
                continue;
            }
        }
    }

    all_primitives
}

/// Triangulate a shape with holes using i_triangle
/// Input: Vec<Contour> where first is outer boundary (CCW), rest are holes (CW)
/// Returns: Vec<Primitive::Triangle>
fn triangulate_shape_with_holes(
    contours: &[Vec<[f32; 2]>],
    exposure: f32,
) -> Result<Vec<Primitive>, String> {
    if contours.is_empty() {
        return Ok(Vec::new());
    }

    // Extract outer boundary (first contour)
    let outer = &contours[0];

    if outer.len() < 3 {
        return Err("Outer boundary has less than 3 vertices".to_string());
    }

    // Extract holes (remaining contours)
    let holes: Vec<Vec<[f32; 2]>> = contours[1..].to_vec();

    // Convert to i_triangle format
    // i_triangle expects Vec<Vec<[f32; 2]>> where first is outer, rest are holes
    let mut paths = vec![outer.clone()];
    paths.extend(holes);

    // Use i_triangle for triangulation with holes
    let raw_triangulation = paths.triangulate();
    let tri_result = raw_triangulation.to_triangulation::<u32>();

    // Create triangles from indices
    let mut triangles = Vec::new();
    for i in (0..tri_result.indices.len()).step_by(3) {
        if i + 2 < tri_result.indices.len() {
            let idx0 = tri_result.indices[i] as usize;
            let idx1 = tri_result.indices[i + 1] as usize;
            let idx2 = tri_result.indices[i + 2] as usize;

            if idx0 < tri_result.points.len()
                && idx1 < tri_result.points.len()
                && idx2 < tri_result.points.len()
            {
                triangles.push(Primitive::Triangle {
                    vertices: vec![
                        tri_result.points[idx0],
                        tri_result.points[idx1],
                        tri_result.points[idx2],
                    ],
                    exposure,
                });
            }
        }
    }

    Ok(triangles)
}

/// Offset a primitive by the given dx and dy
fn offset_primitive_by(primitive: &Primitive, dx: f32, dy: f32) -> Primitive {
    match primitive {
        Primitive::Circle {
            x,
            y,
            radius,
            exposure,
        } => Primitive::Circle {
            x: x + dx,
            y: y + dy,
            radius: *radius,
            exposure: *exposure,
        },
        Primitive::Triangle { vertices, exposure } => Primitive::Triangle {
            vertices: vertices.iter().map(|[vx, vy]| [vx + dx, vy + dy]).collect(),
            exposure: *exposure,
        },
        Primitive::Arc {
            x,
            y,
            radius,
            start_angle,
            end_angle,
            thickness,
            exposure,
        } => Primitive::Arc {
            x: x + dx,
            y: y + dy,
            radius: *radius,
            start_angle: *start_angle,
            end_angle: *end_angle,
            thickness: *thickness,
            exposure: *exposure,
        },
        Primitive::Thermal {
            x,
            y,
            outer_diameter,
            inner_diameter,
            gap_thickness,
            rotation,
            exposure,
        } => Primitive::Thermal {
            x: x + dx,
            y: y + dy,
            outer_diameter: *outer_diameter,
            inner_diameter: *inner_diameter,
            gap_thickness: *gap_thickness,
            rotation: *rotation,
            exposure: *exposure,
        },
    }
}

/// Parse primitive statement: 1,1,$7,$5-$3,$6-$3,$4*
fn parse_primitive_statement(
    stmt: &str,
    variables: &HashMap<String, f32>,
    primitives: &mut Vec<Primitive>,
) -> Option<u32> {
    let stmt = stmt.trim_end_matches('*');
    let parts: Vec<&str> = stmt.split(',').collect();

    if parts.is_empty() {
        return None;
    }

    // First part is primitive code
    let code: u32 = parts[0].parse().ok()?;

    match code {
        0 => {
            // Comment
            Some(0)
        }
        1 => {
            // Circle: 1,exposure,diameter,centerX,centerY[,rotation]
            if parts.len() < 5 {
                return None;
            }
            let exposure: f32 = evaluate_expression(parts[1], variables).ok()?;
            let diameter: f32 = evaluate_expression(parts[2], variables).ok()?;
            let center_x: f32 = evaluate_expression(parts[3], variables).ok()?;
            let center_y: f32 = evaluate_expression(parts[4], variables).ok()?;

            primitives.push(Primitive::Circle {
                x: center_x,
                y: center_y,
                radius: diameter / 2.0,
                exposure,
            });

            Some(1)
        }
        4 => {
            // Outline: 4,exposure,vertices,x1,y1,x2,y2,...,xn,yn[,rotation]
            if parts.len() < 4 {
                return None;
            }
            let exposure: f32 = evaluate_expression(parts[1], variables).ok()?;
            let num_vertices: u32 = evaluate_expression(parts[2], variables).ok()? as u32;
            let rotation: f32 = if parts.len() > 3 + (num_vertices as usize) * 2 {
                evaluate_expression(parts[3 + (num_vertices as usize) * 2], variables).ok()?
                    * (std::f32::consts::PI / 180.0) // degrees to radians
            } else {
                0.0
            };

            // Collect vertices
            let mut vertices = Vec::new();
            for i in 0..num_vertices as usize {
                let x_idx = 3 + i * 2;
                let y_idx = 3 + i * 2 + 1;
                if x_idx >= parts.len() || y_idx >= parts.len() {
                    return None;
                }
                let x = evaluate_expression(parts[x_idx], variables).ok()?;
                let y = evaluate_expression(parts[y_idx], variables).ok()?;
                vertices.push([x, y]);
            }

            // Execute triangulation
            if vertices.len() >= 3 {
                match triangulate_outline(&vertices, exposure) {
                    Ok(triangles) => {
                        for triangle in triangles {
                            let mut tri = triangle;
                            // Apply rotation
                            if rotation != 0.0 {
                                if let Primitive::Triangle { vertices, .. } = &mut tri {
                                    for vertex in vertices.iter_mut() {
                                        rotate_point(vertex, rotation, 0.0, 0.0);
                                    }
                                }
                            }
                            primitives.push(tri);
                        }
                        Some(4)
                    }
                    Err(_) => None,
                }
            } else {
                None
            }
        }
        5 => {
            // Polygon: 5,exposure,vertices,centerX,centerY,diameter[,rotation]
            if parts.len() < 6 {
                return None;
            }
            let exposure: f32 = evaluate_expression(parts[1], variables).ok()?;
            let num_vertices: u32 = evaluate_expression(parts[2], variables).ok()? as u32;
            let center_x: f32 = evaluate_expression(parts[3], variables).ok()?;
            let center_y: f32 = evaluate_expression(parts[4], variables).ok()?;
            let diameter: f32 = evaluate_expression(parts[5], variables).ok()?;
            let rotation: f32 = if parts.len() > 6 {
                evaluate_expression(parts[6], variables).ok()? * (std::f32::consts::PI / 180.0)
            // degrees to radians
            } else {
                0.0
            };

            // Calculate regular polygon vertices
            let radius = diameter / 2.0;
            let mut vertices = Vec::new();
            let angle_step = 2.0 * std::f32::consts::PI / num_vertices as f32;

            for i in 0..num_vertices as usize {
                let angle = angle_step * i as f32;
                let x = center_x + radius * angle.cos();
                let y = center_y + radius * angle.sin();
                vertices.push([x, y]);
            }

            // Fan triangulation: create triangles from center to all adjacent vertices
            for i in 0..(num_vertices as usize) {
                let next_i = (i + 1) % (num_vertices as usize);
                let mut triangle = Primitive::Triangle {
                    vertices: vec![[center_x, center_y], vertices[i], vertices[next_i]],
                    exposure,
                };

                // Apply rotation
                if rotation != 0.0 {
                    if let Primitive::Triangle { vertices, .. } = &mut triangle {
                        for vertex in vertices.iter_mut() {
                            rotate_point(vertex, rotation, 0.0, 0.0);
                        }
                    }
                }

                primitives.push(triangle);
            }

            Some(5)
        }
        7 => {
            // Thermal: 7,centerX,centerY,outerDiameter,innerDiameter,gapThickness[,rotation]
            // Note: Thermal primitives don't have exposure parameter in Gerber spec (always positive)
            if parts.len() < 6 {
                return None;
            }
            let center_x: f32 = evaluate_expression(parts[1], variables).ok()?;
            let center_y: f32 = evaluate_expression(parts[2], variables).ok()?;
            let outer_diameter: f32 = evaluate_expression(parts[3], variables).ok()?;
            let inner_diameter: f32 = evaluate_expression(parts[4], variables).ok()?;
            let gap_thickness: f32 = evaluate_expression(parts[5], variables).ok()?;
            let rotation: f32 = if parts.len() > 6 {
                evaluate_expression(parts[6], variables).ok()? * (std::f32::consts::PI / 180.0)
            } else {
                0.0
            };

            primitives.push(Primitive::Thermal {
                x: center_x,
                y: center_y,
                outer_diameter,
                inner_diameter,
                gap_thickness,
                rotation,
                exposure: 1.0, // Thermals are always positive
            });

            Some(7)
        }
        20 => {
            // Vector Line: 20,exposure,width,startX,startY,endX,endY[,rotation]
            if parts.len() < 7 {
                return None;
            }
            let exposure: f32 = evaluate_expression(parts[1], variables).ok()?;
            let width: f32 = evaluate_expression(parts[2], variables).ok()?;
            let start_x: f32 = evaluate_expression(parts[3], variables).ok()?;
            let start_y: f32 = evaluate_expression(parts[4], variables).ok()?;
            let end_x: f32 = evaluate_expression(parts[5], variables).ok()?;
            let end_y: f32 = evaluate_expression(parts[6], variables).ok()?;
            let rotation: f32 = if parts.len() > 7 {
                evaluate_expression(parts[7], variables).ok()? * (std::f32::consts::PI / 180.0)
            // degrees to radians
            } else {
                0.0
            };

            // Split line into two triangles
            let triangles = line_to_triangles(start_x, start_y, end_x, end_y, width, exposure);
            for mut triangle in triangles {
                // Apply rotation
                if rotation != 0.0 {
                    if let Primitive::Triangle { vertices, .. } = &mut triangle {
                        for vertex in vertices.iter_mut() {
                            rotate_point(vertex, rotation, 0.0, 0.0);
                        }
                    }
                }
                primitives.push(triangle);
            }

            Some(20)
        }
        21 => {
            // Center Line: 21,exposure,width,height,centerX,centerY[,rotation]
            if parts.len() < 6 {
                return None;
            }
            let exposure: f32 = evaluate_expression(parts[1], variables).ok()?;
            let width: f32 = evaluate_expression(parts[2], variables).ok()?;
            let height: f32 = evaluate_expression(parts[3], variables).ok()?;
            let center_x: f32 = evaluate_expression(parts[4], variables).ok()?;
            let center_y: f32 = evaluate_expression(parts[5], variables).ok()?;
            let rotation: f32 = if parts.len() > 6 {
                evaluate_expression(parts[6], variables).ok()?
            } else {
                0.0
            };

            // Split center-based rectangle into two triangles
            let half_width = width / 2.0;
            let half_height = height / 2.0;

            let v1 = [center_x - half_width, center_y - half_height];
            let v2 = [center_x + half_width, center_y - half_height];
            let v3 = [center_x + half_width, center_y + half_height];
            let v4 = [center_x - half_width, center_y + half_height];

            // Two triangles: (v1, v2, v3), (v1, v3, v4)
            let mut tri1 = Primitive::Triangle {
                vertices: vec![v1, v2, v3],
                exposure,
            };
            let mut tri2 = Primitive::Triangle {
                vertices: vec![v1, v3, v4],
                exposure,
            };

            // Apply rotation
            if rotation != 0.0 {
                if let Primitive::Triangle { vertices, .. } = &mut tri1 {
                    for vertex in vertices.iter_mut() {
                        rotate_point(vertex, rotation, center_x, center_y);
                    }
                }
                if let Primitive::Triangle { vertices, .. } = &mut tri2 {
                    for vertex in vertices.iter_mut() {
                        rotate_point(vertex, rotation, center_x, center_y);
                    }
                }
            }

            primitives.push(tri1);
            primitives.push(tri2);

            Some(21)
        }
        _ => {
            // Unknown code
            None
        }
    }
}

/// Aperture definition (Circle, Rectangle, Obround, Polygon, or Macro reference)
#[derive(Clone, Debug)]
pub struct Aperture {
    pub code: String,
    pub shape: String, // C, R, O, P, or macro name
    pub radius: f32,
    pub primitives: Vec<Primitive>, // Aperture contains multiple basic primitives
    pub has_negative: bool,         // true if primitives contain exposure=0
}

impl Aperture {
    pub fn new(code: String, shape: String, radius: f32) -> Self {
        Aperture {
            code,
            shape,
            radius,
            primitives: Vec::new(),
            has_negative: false,
        }
    }
}

/// Aperture macro definition - kept as statements until parameters arrive
#[derive(Clone, Debug)]
pub struct ApertureMacro {
    pub name: String,
    pub statements: Vec<String>, // Keep as String for dynamic calculation
    pub has_negative: bool,      // true if any primitive has exposure 0
}

impl ApertureMacro {
    pub fn new(name: String) -> Self {
        ApertureMacro {
            name,
            statements: Vec::new(),
            has_negative: false,
        }
    }

    /// Called from %ADD with parameters to generate Aperture's primitives
    pub fn instantiate(&self, params: &[f32]) -> Vec<Primitive> {
        let mut primitives = Vec::new();
        let mut variables: HashMap<String, f32> = HashMap::new();

        // Initialize parameters as $1, $2, ...
        for (i, &param) in params.iter().enumerate() {
            variables.insert(format!("${}", i + 1), param);
        }

        for statement in &self.statements {
            let stmt = statement.trim();
            if stmt.is_empty() {
                continue;
            }

            // Skip comment lines: starting with "0 " or just "0"
            if stmt.starts_with("0 ") || stmt == "0" {
                continue;
            }

            // Check for variable assignment command: $5=$1/2
            if stmt.starts_with('$') && stmt.contains('=') {
                if let Some(eq_idx) = stmt.find('=') {
                    let var_name = stmt[..eq_idx].trim().to_string();
                    let expr = stmt[eq_idx + 1..].trim();

                    if let Ok(value) = evaluate_expression(expr, &variables) {
                        variables.insert(var_name, value);
                    }
                }
            } else {
                // Primitive command: 1,1,$7,$5-$3,$6-$3,$4*
                parse_primitive_statement(stmt, &variables, &mut primitives);
            }
        }

        primitives
    }

    /// Convert macro primitives directly to iOverlay Shape format
    /// Returns: Vec<(Shape, exposure)> where Shape is Vec<Contour>
    #[allow(dead_code)]
    pub fn instantiate_as_shapes(&self, params: &[f32]) -> Vec<(Vec<Vec<[f32; 2]>>, f32)> {
        let mut shapes = Vec::new();
        let mut variables: HashMap<String, f32> = HashMap::new();

        // Initialize parameters as $1, $2, ...
        for (i, &param) in params.iter().enumerate() {
            variables.insert(format!("${}", i + 1), param);
        }

        for statement in &self.statements {
            let stmt = statement.trim();
            if stmt.is_empty() {
                continue;
            }

            // Skip comments: "0 " or just "0"
            if stmt.starts_with("0 ") || stmt == "0" {
                continue;
            }

            // Variable assignment: $5=$1/2
            if stmt.starts_with('$') && stmt.contains('=') {
                if let Some(eq_idx) = stmt.find('=') {
                    let var_name = stmt[..eq_idx].trim().to_string();
                    let expr = stmt[eq_idx + 1..].trim();

                    if let Ok(value) = evaluate_expression(expr, &variables) {
                        variables.insert(var_name, value);
                    }
                }
            } else {
                // Primitive statement: code,exposure,...
                let stmt = stmt.trim_end_matches('*');
                let parts: Vec<&str> = stmt.split(',').collect();

                if parts.is_empty() {
                    continue;
                }

                // Parse primitive code
                let code: u32 = match parts[0].parse() {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                // Skip comment code
                if code == 0 {
                    continue;
                }

                // Extract exposure (for primitives that have it)
                let exposure: f32 = if code == 7 {
                    // Thermal doesn't have exposure parameter
                    1.0
                } else if parts.len() > 1 {
                    evaluate_expression(parts[1], &variables).unwrap_or(1.0)
                } else {
                    1.0
                };

                // Extract parameters based on primitive type
                let mut params_list = Vec::new();

                match code {
                    1 => {
                        // Circle: exposure, diameter, centerX, centerY, [rotation]
                        if parts.len() >= 5 {
                            if let Ok(diameter) = evaluate_expression(parts[2], &variables) {
                                params_list.push(diameter);
                            }
                            if let Ok(center_x) = evaluate_expression(parts[3], &variables) {
                                params_list.push(center_x);
                            }
                            if let Ok(center_y) = evaluate_expression(parts[4], &variables) {
                                params_list.push(center_y);
                            }
                        }
                    }
                    4 => {
                        // Outline: exposure, vertices, x1, y1, ..., [rotation]
                        if parts.len() >= 3 {
                            if let Ok(num_vertices) = evaluate_expression(parts[2], &variables) {
                                params_list.push(num_vertices);
                                let num_verts = num_vertices as usize;
                                for i in 0..num_verts {
                                    let x_idx = 3 + i * 2;
                                    let y_idx = 3 + i * 2 + 1;
                                    if x_idx < parts.len() && y_idx < parts.len() {
                                        if let Ok(x) = evaluate_expression(parts[x_idx], &variables)
                                        {
                                            params_list.push(x);
                                        }
                                        if let Ok(y) = evaluate_expression(parts[y_idx], &variables)
                                        {
                                            params_list.push(y);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    5 => {
                        // Polygon: exposure, vertices, centerX, centerY, diameter, [rotation]
                        if parts.len() >= 6 {
                            if let Ok(num_vertices) = evaluate_expression(parts[2], &variables) {
                                params_list.push(num_vertices);
                            }
                            if let Ok(center_x) = evaluate_expression(parts[3], &variables) {
                                params_list.push(center_x);
                            }
                            if let Ok(center_y) = evaluate_expression(parts[4], &variables) {
                                params_list.push(center_y);
                            }
                            if let Ok(diameter) = evaluate_expression(parts[5], &variables) {
                                params_list.push(diameter);
                            }
                        }
                    }
                    7 => {
                        // Thermal: centerX, centerY, outerDiameter, innerDiameter, gapThickness, [rotation]
                        if parts.len() >= 6 {
                            for i in 1..=5 {
                                if let Ok(val) = evaluate_expression(parts[i], &variables) {
                                    params_list.push(val);
                                }
                            }
                            if parts.len() > 6 {
                                if let Ok(rotation) = evaluate_expression(parts[6], &variables) {
                                    params_list.push(rotation);
                                }
                            }
                        }
                    }
                    20 => {
                        // Vector Line: exposure, width, startX, startY, endX, endY, [rotation]
                        if parts.len() >= 7 {
                            for i in 2..=6 {
                                if let Ok(val) = evaluate_expression(parts[i], &variables) {
                                    params_list.push(val);
                                }
                            }
                        }
                    }
                    21 => {
                        // Center Line: exposure, width, height, centerX, centerY, [rotation]
                        if parts.len() >= 6 {
                            for i in 2..=5 {
                                if let Ok(val) = evaluate_expression(parts[i], &variables) {
                                    params_list.push(val);
                                }
                            }
                        }
                    }
                    _ => {}
                }

                // Convert primitive to shape
                if let Some(shape) = macro_primitive_to_shape(code, &params_list, exposure) {
                    shapes.push((shape, exposure));
                }
            }
        }

        shapes
    }
}

/// Parser state tracking current position and modes
#[derive(Clone, Debug)]
pub struct ParserState {
    pub x: f32,
    pub y: f32,
    pub current_aperture: String,
    pub interpolation_mode: String,
    pub quadrant_mode: String,
    pub region_mode: bool,
    pub coordinate_mode: String,
    pub scale: f32,
    pub unit_multiplier: f32, // 1.0 for mm, 25.4 for inch
    pub i: f32,
    pub j: f32,
    #[allow(dead_code)]
    pub previous_x: f32,
    #[allow(dead_code)]
    pub previous_y: f32,
    pub pen_state: String,
    pub polarity: Polarity,
    pub format_spec: FormatSpec,
    // Step and Repeat settings
    pub sr_x: u32,
    pub sr_y: u32,
    pub sr_i: f32,
    pub sr_j: f32,
}

impl Default for ParserState {
    fn default() -> Self {
        ParserState {
            x: 0.0,
            y: 0.0,
            current_aperture: String::new(),
            interpolation_mode: "linear".to_string(),
            quadrant_mode: "single".to_string(),
            region_mode: false,
            coordinate_mode: "absolute".to_string(),
            scale: 1.0,
            unit_multiplier: 1.0, // Default to mm
            i: 0.0,
            j: 0.0,
            previous_x: 0.0,
            previous_y: 0.0,
            pen_state: "up".to_string(),
            polarity: Polarity::Positive,
            format_spec: FormatSpec::default(),
            sr_x: 1,
            sr_y: 1,
            sr_i: 0.0,
            sr_j: 0.0,
        }
    }
}

/// Format specification for coordinate conversion
#[derive(Clone, Debug)]
pub struct FormatSpec {
    pub x_integer_digits: u32,
    pub x_decimal_digits: u32,
    pub y_integer_digits: u32,
    pub y_decimal_digits: u32,
    pub leading: u32,
    pub trailing: u32,
    // Cached calculation values - performance optimization
    pub x_divisor: f64,      // 10^(x_decimal_digits)
    pub y_divisor: f64,      // 10^(y_decimal_digits)
    pub x_total_digits: i32, // x_integer_digits + x_decimal_digits
    pub y_total_digits: i32, // y_integer_digits + y_decimal_digits
}

impl Default for FormatSpec {
    fn default() -> Self {
        FormatSpec {
            x_integer_digits: 2,
            x_decimal_digits: 4,
            y_integer_digits: 2,
            y_decimal_digits: 4,
            leading: 2,
            trailing: 4,
            x_divisor: 10000.0, // 10^4
            y_divisor: 10000.0, // 10^4
            x_total_digits: 6,  // 2 + 4
            y_total_digits: 6,  // 2 + 4
        }
    }
}

/// Gerber parser with stateful aperture and macro storage
pub struct GerberParser {
    pub apertures: HashMap<String, Aperture>,
    pub macros: HashMap<String, ApertureMacro>,
    pub current_state: ParserState,
    // Store layers by polarity - [pos, neg, pos, neg, ...]
    pub positive_layers: Vec<Vec<Primitive>>,
    pub negative_layers: Vec<Vec<Primitive>>,
    pub current_primitives: Vec<Primitive>, // Accumulating primitives for current polarity
    pub region_contours: Vec<Vec<[f32; 2]>>, // Contour points collected in Region mode
    pub units: String,
    pub format_spec: FormatSpec,
}

impl GerberParser {
    /// Create new parser instance
    pub fn new() -> Self {
        GerberParser {
            apertures: HashMap::new(),
            macros: HashMap::new(),
            current_state: ParserState::default(),
            positive_layers: Vec::new(),
            negative_layers: Vec::new(),
            current_primitives: Vec::new(),
            region_contours: Vec::new(),
            units: "mm".to_string(),
            format_spec: FormatSpec::default(),
        }
    }

    /// Parse Gerber file content and return Vec of GerberData (one per polarity layer)
    /// Order: [pos_layer1, neg_layer1, pos_layer2, neg_layer2, ...]
    pub fn parse(&mut self, data: &str) -> Result<Vec<GerberData>, JsValue> {
        let lines: Vec<&str> = data.split('\n').collect();
        let length = lines.len();
        let mut i = 0;

        while i < length {
            let line_ref = lines[i].trim();

            if line_ref.is_empty() {
                i += 1;
                continue;
            }

            if line_ref.starts_with('%') {
                parse_command(
                    line_ref,
                    &mut i,
                    length,
                    &lines,
                    &mut self.current_state,
                    &mut self.apertures,
                    &mut self.macros,
                    &mut self.current_primitives,
                    &mut self.positive_layers,
                    &mut self.negative_layers,
                );
            } else if line_ref.starts_with("G04") {
                // Comment line, skip
            } else if line_ref.starts_with('G')
                || line_ref.starts_with('D')
                || line_ref.starts_with('X')
                || line_ref.starts_with('Y')
                || line_ref.starts_with('I')
                || line_ref.starts_with('J')
            {
                parse_graphic_command(
                    line_ref,
                    &mut self.current_state,
                    &self.apertures,
                    &mut self.current_primitives,
                    &mut self.region_contours,
                );
            }

            i += 1;
        }

        // Save last accumulated primitives by polarity
        if !self.current_primitives.is_empty() {
            if self.current_state.polarity == Polarity::Positive {
                self.positive_layers.push(self.current_primitives.clone());
            } else {
                self.negative_layers.push(self.current_primitives.clone());
            }
        }

        // Convert each layer to individual GerberData
        // Order: [pos_layer1, neg_layer1, pos_layer2, neg_layer2, ...]
        let mut gerber_data_layers: Vec<GerberData> = Vec::new();

        let max_layers = self.positive_layers.len().max(self.negative_layers.len());
        for idx in 0..max_layers {
            // Add positive layer
            if idx < self.positive_layers.len() {
                let gerber_data = Self::primitives_to_gerber_data(&self.positive_layers[idx]);
                gerber_data_layers.push(gerber_data);
            }
            // Add negative layer
            if idx < self.negative_layers.len() {
                let gerber_data = Self::primitives_to_gerber_data(&self.negative_layers[idx]);
                gerber_data_layers.push(gerber_data);
            }
        }

        Ok(gerber_data_layers)
    }

    /// Reset parser state to defaults
    fn reset(&mut self) {
        self.positive_layers.clear();
        self.negative_layers.clear();
        self.current_primitives.clear();
        self.region_contours.clear();
        self.current_state = ParserState::default();
    }

    /// Convert a vector of primitives to GerberData
    fn primitives_to_gerber_data(primitives: &[Primitive]) -> GerberData {
        let mut triangle_vertices: Vec<f32> = Vec::new();
        let mut triangle_indices: Vec<u32> = Vec::new();
        let mut circles_x: Vec<f32> = Vec::new();
        let mut circles_y: Vec<f32> = Vec::new();
        let mut circles_radius: Vec<f32> = Vec::new();
        let mut arcs_x: Vec<f32> = Vec::new();
        let mut arcs_y: Vec<f32> = Vec::new();
        let mut arcs_radius: Vec<f32> = Vec::new();
        let mut arcs_start_angle: Vec<f32> = Vec::new();
        let mut arcs_sweep_angle: Vec<f32> = Vec::new();
        let mut arcs_thickness: Vec<f32> = Vec::new();
        let mut thermals_x: Vec<f32> = Vec::new();
        let mut thermals_y: Vec<f32> = Vec::new();
        let mut thermals_outer_diameter: Vec<f32> = Vec::new();
        let mut thermals_inner_diameter: Vec<f32> = Vec::new();
        let mut thermals_gap_thickness: Vec<f32> = Vec::new();
        let mut thermals_rotation: Vec<f32> = Vec::new();

        let mut vertex_offset: u32 = 0;

        // Unit conversion: divide all values by 1000, same as JavaScript geometry.js
        // This converts Gerber file internal units to millimeters
        const TO_MM: f32 = 1.0 / 1000.0;

        for primitive in primitives {
            match primitive {
                Primitive::Triangle { vertices, .. } => {
                    // Add triangle vertices to array (convert to mm units)
                    for vertex in vertices {
                        triangle_vertices.push(vertex[0] * TO_MM);
                        triangle_vertices.push(vertex[1] * TO_MM);
                    }
                    // Add index for every 3 vertices (one triangle)
                    triangle_indices.push(vertex_offset);
                    triangle_indices.push(vertex_offset + 1);
                    triangle_indices.push(vertex_offset + 2);
                    vertex_offset += 3;
                }
                Primitive::Circle { x, y, radius, .. } => {
                    circles_x.push(*x * TO_MM);
                    circles_y.push(*y * TO_MM);
                    circles_radius.push(*radius * TO_MM);
                }
                Primitive::Arc {
                    x,
                    y,
                    radius,
                    start_angle,
                    end_angle,
                    thickness,
                    ..
                } => {
                    arcs_x.push(*x * TO_MM);
                    arcs_y.push(*y * TO_MM);
                    arcs_radius.push(*radius * TO_MM);
                    arcs_start_angle.push(*start_angle);
                    // sweep_angle = end_angle - start_angle
                    arcs_sweep_angle.push(*end_angle - *start_angle);
                    arcs_thickness.push(*thickness * TO_MM);
                }
                Primitive::Thermal {
                    x,
                    y,
                    outer_diameter,
                    inner_diameter,
                    gap_thickness,
                    rotation,
                    ..
                } => {
                    thermals_x.push(*x * TO_MM);
                    thermals_y.push(*y * TO_MM);
                    thermals_outer_diameter.push(*outer_diameter * TO_MM);
                    thermals_inner_diameter.push(*inner_diameter * TO_MM);
                    thermals_gap_thickness.push(*gap_thickness * TO_MM);
                    thermals_rotation.push(*rotation);
                }
            }
        }

        // Calculate boundary from all geometry
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        // Include triangle vertices in boundary
        for i in (0..triangle_vertices.len()).step_by(2) {
            let x = triangle_vertices[i];
            let y = triangle_vertices[i + 1];
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        // Include circles in boundary (center +/- radius)
        for i in 0..circles_x.len() {
            let x = circles_x[i];
            let y = circles_y[i];
            let r = circles_radius[i];
            min_x = min_x.min(x - r);
            max_x = max_x.max(x + r);
            min_y = min_y.min(y - r);
            max_y = max_y.max(y + r);
        }

        // Include arcs in boundary (center +/- radius + thickness/2)
        for i in 0..arcs_x.len() {
            let x = arcs_x[i];
            let y = arcs_y[i];
            let r = arcs_radius[i];
            let t = arcs_thickness[i];
            let outer = r + t / 2.0;
            min_x = min_x.min(x - outer);
            max_x = max_x.max(x + outer);
            min_y = min_y.min(y - outer);
            max_y = max_y.max(y + outer);
        }

        // Include thermals in boundary (center +/- outer_diameter/2)
        for i in 0..thermals_x.len() {
            let x = thermals_x[i];
            let y = thermals_y[i];
            let r = thermals_outer_diameter[i] / 2.0;
            min_x = min_x.min(x - r);
            max_x = max_x.max(x + r);
            min_y = min_y.min(y - r);
            max_y = max_y.max(y + r);
        }

        // Handle empty geometry case
        if min_x == f32::INFINITY {
            min_x = 0.0;
            max_x = 0.0;
            min_y = 0.0;
            max_y = 0.0;
        }

        GerberData::new(
            Triangles::new(triangle_vertices, triangle_indices),
            Circles::new(circles_x, circles_y, circles_radius),
            Arcs::new(
                arcs_x,
                arcs_y,
                arcs_radius,
                arcs_start_angle,
                arcs_sweep_angle,
                arcs_thickness,
            ),
            Thermals::new(
                thermals_x,
                thermals_y,
                thermals_outer_diameter,
                thermals_inner_diameter,
                thermals_gap_thickness,
                thermals_rotation,
            ),
            Boundary::new(min_x, max_x, min_y, max_y),
        )
    }
}

fn parse_command(
    line_ref: &str,
    i: &mut usize,
    length: usize,
    lines: &[&str],
    state: &mut ParserState,
    apertures: &mut HashMap<String, Aperture>,
    macros: &mut HashMap<String, ApertureMacro>,
    current_primitives: &mut Vec<Primitive>,
    positive_layers: &mut Vec<Vec<Primitive>>,
    negative_layers: &mut Vec<Vec<Primitive>>,
) {
    let line = if !line_ref.ends_with('%') {
        let mut buffer = vec![line_ref.to_string()];
        *i += 1;

        while *i < length {
            let next_line = lines[*i].trim();
            buffer.push(next_line.to_string());

            if next_line.ends_with('%') {
                break;
            }
            *i += 1;
        }

        buffer.join("")
    } else {
        line_ref.to_string()
    };

    if line.starts_with("%AM") {
        parse_macro(&line, macros);
    } else if line.starts_with("%ADD") {
        parse_aperture(&line, apertures, macros, state.unit_multiplier);
    } else if line.starts_with("%MO") {
        // Unit mode: %MOMM* or %MOIN*
        parse_mo(&line, state);
    } else if line.starts_with("%FS") {
        // Format spec: %FSLAX24Y24*%
        parse_format_spec(&line, state);
    } else if line.starts_with("%LP") {
        // Polarity: %LPD* (dark/positive) or %LPC* (clear/negative)
        parse_lp(
            &line,
            state,
            current_primitives,
            positive_layers,
            negative_layers,
        );
    } else if line.starts_with("%SR") {
        // Step and repeat: %SRX3Y2I10J20*%
        parse_sr(&line, state);
    } else if line.starts_with("%IF") {
        // Image polarity: %IFPOS*% or %IFNEG*%
        parse_if(&line, state);
    } else if line.starts_with("%AB") {
        // Block Aperture: %ABD##*% ... %AB*%
        // TODO: Implement full block aperture support
    } else if line.starts_with("%LM") {
        // Layer mirroring: %LMN*, %LMX*, %LMY*, %LMXY*
        // TODO: Implement mirroring transformation
    } else if line.starts_with("%LR") {
        // Layer rotation: %LR45.0*
        // TODO: Implement rotation transformation
    } else if line.starts_with("%LS") {
        // Layer scaling: %LS0.8*
        // TODO: Implement scaling transformation
    } else {
        // Unknown or unsupported command
    }
}

/// Parse graphic commands - process G/D/XY codes
/// Example: G01X1000Y2000D01* (draw line), X1000Y2000D03* (flash), etc.
fn parse_graphic_command(
    line: &str,
    state: &mut ParserState,
    apertures: &HashMap<String, Aperture>,
    primitives: &mut Vec<Primitive>,
    region_contours: &mut Vec<Vec<[f32; 2]>>,
) {
    let clean_line = line.trim_end_matches('*');

    // Process G-code
    if let Some(g_match) = extract_value(clean_line, 'G') {
        if let Ok(g_code) = g_match.parse::<u32>() {
            match g_code {
                1 => {
                    // G01: Linear interpolation (1x scale)
                    state.interpolation_mode = "linear".to_string();
                    state.scale = 1.0;
                }
                2 => {
                    // G02: Clockwise arc interpolation
                    state.interpolation_mode = "clockwise".to_string();
                }
                3 => {
                    // G03: Counterclockwise arc interpolation
                    state.interpolation_mode = "counterclockwise".to_string();
                }
                10 => {
                    // G10: Linear interpolation (10x scale)
                    state.interpolation_mode = "linear_x10".to_string();
                    state.scale = 10.0;
                }
                11 => {
                    // G11: Linear interpolation (0.1x scale)
                    state.interpolation_mode = "linear_x01".to_string();
                    state.scale = 0.1;
                }
                12 => {
                    // G12: Linear interpolation (0.01x scale)
                    state.interpolation_mode = "linear_x001".to_string();
                    state.scale = 0.01;
                }
                36 => {
                    // G36: Start region fill mode
                    state.region_mode = true;
                    region_contours.clear();
                    region_contours.push(Vec::new()); // Start new contour
                }
                37 => {
                    // G37: End region fill mode
                    state.region_mode = false;

                    // Triangulate region and add to primitives
                    // Regions are always positive (add material)
                    for contour in region_contours.iter() {
                        if contour.len() >= 3 {
                            match triangulate_outline(contour, 1.0) {
                                Ok(triangles) => {
                                    primitives.extend(triangles);
                                }
                                Err(_e) => {
                                    // Triangulation failed, skip this contour
                                }
                            }
                        }
                    }

                    region_contours.clear();
                }
                70 => {
                    // G70: Unit mode - Inches
                    state.unit_multiplier = 25.4;
                }
                71 => {
                    // G71: Unit mode - Millimeters
                    state.unit_multiplier = 1.0;
                }
                74 => {
                    // G74: Single quadrant mode
                    state.quadrant_mode = "single".to_string();
                }
                75 => {
                    // G75: Multi-quadrant mode
                    state.quadrant_mode = "multi".to_string();
                }
                90 => {
                    // G90: Absolute coordinate mode
                    state.coordinate_mode = "absolute".to_string();
                }
                91 => {
                    // G91: Incremental coordinate mode
                    state.coordinate_mode = "incremental".to_string();
                }
                _ => {
                    // Unsupported G-code
                }
            }
        }
    }

    // Extract coordinates and D-code using regex
    let x_match = extract_value(clean_line, 'X');
    let y_match = extract_value(clean_line, 'Y');
    let i_match = extract_value(clean_line, 'I');
    let j_match = extract_value(clean_line, 'J');
    let d_match = extract_value(clean_line, 'D');

    let mut x = state.x;
    let mut y = state.y;
    let mut i = 0.0;
    let mut j = 0.0;

    // Process X coordinate
    if let Some(x_val) = x_match.as_ref() {
        let new_x =
            convert_coordinate(x_val, 'x', &state.format_spec, state.unit_multiplier) * state.scale;
        x = if state.coordinate_mode == "absolute" {
            new_x
        } else {
            state.x + new_x
        };
    }

    // Process Y coordinate
    if let Some(y_val) = y_match.as_ref() {
        let new_y =
            convert_coordinate(y_val, 'y', &state.format_spec, state.unit_multiplier) * state.scale;
        y = if state.coordinate_mode == "absolute" {
            new_y
        } else {
            state.y + new_y
        };
    }

    // Process I coordinate (arc center X offset)
    if let Some(i_val) = i_match.as_ref() {
        let raw_i =
            convert_coordinate(i_val, 'x', &state.format_spec, state.unit_multiplier) * state.scale;
        i = if state.quadrant_mode == "single" {
            raw_i.abs()
        } else {
            raw_i
        };
    }

    // Process J coordinate (arc center Y offset)
    if let Some(j_val) = j_match.as_ref() {
        let raw_j =
            convert_coordinate(j_val, 'y', &state.format_spec, state.unit_multiplier) * state.scale;
        j = if state.quadrant_mode == "single" {
            raw_j.abs()
        } else {
            raw_j
        };
    }

    // Process D-code
    if let Some(d_val) = d_match {
        if let Ok(d_code) = d_val.parse::<u32>() {
            match d_code {
                1 => {
                    // D01: Pen down (draw)
                    state.pen_state = "down".to_string();

                    // If in region mode, add coordinates to contour
                    if state.region_mode {
                        if !region_contours.is_empty() {
                            let last_contour = region_contours.last_mut().unwrap();
                            last_contour.push([x, y]);
                        }
                    } else {
                        execute_interpolation(state, apertures, primitives, x, y, i, j);
                    }
                }
                2 => {
                    // D02: Pen up (move)
                    state.pen_state = "up".to_string();

                    // Movement is also handled in Region mode
                    if state.region_mode && !region_contours.is_empty() {
                        // D02 starts a new contour (which can be a hole)
                        let last_contour = region_contours.last_mut().unwrap();
                        if !last_contour.is_empty() {
                            // If the current contour is not empty, add a new one
                            region_contours.push(Vec::new());
                        }
                    }
                }
                3 => {
                    // D03: Flash aperture at current position
                    if !state.region_mode {
                        flash_aperture(state, apertures, primitives, x, y);
                    }
                }
                10..=9999 => {
                    // D10+: Aperture selection
                    state.current_aperture = d_val;
                }
                _ => {}
            }
        }
    } else if (x_match.is_some() || y_match.is_some()) && state.pen_state == "down" {
        // If there is only X/Y without D-code and the pen is down, execute interpolation
        if state.region_mode {
            if !region_contours.is_empty() {
                let last_contour = region_contours.last_mut().unwrap();
                last_contour.push([x, y]);
            }
        } else {
            execute_interpolation(state, apertures, primitives, x, y, i, j);
        }
    } else {
        // No drawing operation
    }

    // Update state
    state.x = x;
    state.y = y;
    state.i = i;
    state.j = j;
}

/// Extracts the numeric value after a specific character in a string (e.g., "X1000" → "1000")
fn extract_value(line: &str, key: char) -> Option<String> {
    let key_str = key.to_string();
    if let Some(pos) = line.find(&key_str) {
        let rest = &line[pos + 1..];
        let mut value = String::new();
        let mut has_minus = false;

        for ch in rest.chars() {
            if ch == '-' || ch == '+' {
                has_minus = ch == '-';
            } else if ch.is_ascii_digit() {
                if has_minus && value.is_empty() {
                    value.push('-');
                }
                value.push(ch);
            } else {
                break;
            }
        }

        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    } else {
        None
    }
}

/// Coordinate value conversion - decimal point processing according to format spec
fn convert_coordinate(
    coord_str: &str,
    axis: char,
    format_spec: &FormatSpec,
    unit_multiplier: f32,
) -> f32 {
    if let Ok(val) = coord_str.parse::<i64>() {
        let divisor = match axis {
            'x' => format_spec.x_divisor,
            'y' => format_spec.y_divisor,
            _ => 10000.0,
        };

        // Divide by decimal point position (no padding) and then convert units (1.0 for mm, 25.4 for inch)
        (val as f64 / divisor) as f32 * unit_multiplier
    } else {
        0.0
    }
}

/// Flash aperture at given position - add all primitives of the aperture to the position
fn flash_aperture(
    state: &ParserState,
    apertures: &HashMap<String, Aperture>,
    primitives: &mut Vec<Primitive>,
    x: f32,
    y: f32,
) {
    if let Some(aperture) = apertures.get(&state.current_aperture) {
        // Step and Repeat iteration
        for sy in 0..state.sr_y {
            for sx in 0..state.sr_x {
                let flash_x = x + sx as f32 * state.sr_i;
                let flash_y = y + sy as f32 * state.sr_j;

                // Use pre-calculated has_negative field for performance
                if aperture.has_negative {
                    // Boolean operations with hole preservation
                    // Convert offset primitives to shapes
                    let shapes_with_exposure: Vec<(Vec<Vec<[f32; 2]>>, f32)> = aperture
                        .primitives
                        .iter()
                        .map(|p| {
                            let offset_p = offset_primitive_by(p, flash_x, flash_y);
                            let poly = primitive_to_polygon(&offset_p);
                            let exposure = match &offset_p {
                                Primitive::Circle { exposure, .. } => *exposure,
                                Primitive::Triangle { exposure, .. } => *exposure,
                                Primitive::Arc { exposure, .. } => *exposure,
                                Primitive::Thermal { exposure, .. } => *exposure,
                            };
                            // Wrap polygon in shape format (single contour)
                            (vec![poly], exposure)
                        })
                        .collect();

                    // Apply boolean operations with hole preservation
                    let result_primitives = apply_boolean_operations_v2(&shapes_with_exposure);
                    primitives.extend(result_primitives);
                } else {
                    // Direct primitive cloning
                    for primitive in &aperture.primitives {
                        let mut new_primitive = primitive.clone();
                        match &mut new_primitive {
                            Primitive::Circle { x: px, y: py, .. } => {
                                // Add Circle with offset to position
                                *px += flash_x;
                                *py += flash_y;
                            }
                            Primitive::Triangle { vertices, .. } => {
                                // Add all vertices of Triangle with offset to position
                                for vertex in vertices.iter_mut() {
                                    vertex[0] += flash_x;
                                    vertex[1] += flash_y;
                                }
                            }
                            Primitive::Arc { x: ax, y: ay, .. } => {
                                // Add Arc with offset to position
                                *ax += flash_x;
                                *ay += flash_y;
                            }
                            Primitive::Thermal { x: tx, y: ty, .. } => {
                                // Add Thermal with offset to position
                                *tx += flash_x;
                                *ty += flash_y;
                            }
                        }
                        primitives.push(new_primitive);
                    }
                }
            }
        }
    }
}

/// Execute interpolation (draw line or arc)
fn execute_interpolation(
    state: &mut ParserState,
    apertures: &HashMap<String, Aperture>,
    primitives: &mut Vec<Primitive>,
    end_x: f32,
    end_y: f32,
    i: f32,
    j: f32,
) {
    let start_x = state.x;
    let start_y = state.y;

    // Get current aperture
    if let Some(_aperture) = apertures.get(&state.current_aperture) {
        match state.interpolation_mode.as_str() {
            "linear" | "linear_x10" | "linear_x01" | "linear_x001" => {
                // Draw line
                // Flash aperture at start point
                flash_aperture(state, apertures, primitives, start_x, start_y);

                // Convert vector line with width of aperture diameter to triangle
                if let Some(aperture) = apertures.get(&state.current_aperture) {
                    let diameter = aperture.radius * 2.0;
                    let line_triangles =
                        line_to_triangles(start_x, start_y, end_x, end_y, diameter, 1.0);
                    for triangle in line_triangles {
                        primitives.push(triangle);
                    }
                }

                // Flash aperture at end point
                flash_aperture(state, apertures, primitives, end_x, end_y);
            }
            "clockwise" | "counterclockwise" => {
                // Draw arc
                // Flash aperture at start point
                flash_aperture(state, apertures, primitives, start_x, start_y);

                // Create Arc primitive
                if let Some(aperture) = apertures.get(&state.current_aperture) {
                    let center_x = start_x + i;
                    let center_y = start_y + j;
                    let radius =
                        ((start_x - center_x).powi(2) + (start_y - center_y).powi(2)).sqrt();
                    let start_angle = (start_y - center_y).atan2(start_x - center_x);
                    let end_angle = (end_y - center_y).atan2(end_x - center_x);
                    let thickness = aperture.radius * 2.0;

                    // Calculate sweep_angle considering direction
                    let mut sweep_angle = end_angle - start_angle;
                    let is_clockwise = state.interpolation_mode == "clockwise";

                    // Normalize sweep angle based on direction
                    if is_clockwise && sweep_angle > 0.0 {
                        sweep_angle -= 2.0 * std::f32::consts::PI;
                    } else if !is_clockwise && sweep_angle < 0.0 {
                        sweep_angle += 2.0 * std::f32::consts::PI;
                    }

                    // Add Arc primitive
                    primitives.push(Primitive::Arc {
                        x: center_x,
                        y: center_y,
                        radius,
                        start_angle,
                        end_angle: start_angle + sweep_angle,
                        thickness,
                        exposure: 1.0,
                    });
                }

                // Flash aperture at end point
                flash_aperture(state, apertures, primitives, end_x, end_y);
            }
            _ => {}
        }
    }
}

/// Convert Arc to triangles - approximate with 11.25 degree steps
/// Parse Aperture macro - %AMname*statements%
fn parse_macro(data: &str, macros: &mut HashMap<String, ApertureMacro>) {
    // Format: %AMname*statement1*statement2*...*%
    // Remove %AM and %
    let content = data.trim_start_matches("%AM").trim_end_matches('%');

    // Split name and statements
    let parts: Vec<&str> = content.split('*').collect();
    if parts.is_empty() {
        return;
    }

    let name = parts[0].to_string();
    let mut macro_def = ApertureMacro::new(name);

    // Parse statements (parts[1] to parts[n-1], last part might be empty)
    for i in 1..parts.len() {
        let stmt = parts[i].trim();
        if stmt.is_empty() {
            continue;
        }

        // Store statement as-is (will be evaluated when instantiated with parameters)
        macro_def.statements.push(stmt.to_string());
    }

    // Calculate has_negative by checking if any statement has exposure=0
    macro_def.has_negative = check_macro_has_negative(&macro_def.statements);

    macros.insert(macro_def.name.clone(), macro_def);
}

/// Check if macro statements contain any primitive with exposure=0
fn check_macro_has_negative(statements: &[String]) -> bool {
    for stmt in statements {
        let trimmed = stmt.trim();

        // Skip empty lines, comments (0 comment), and variable assignments ($x=...)
        if trimmed.is_empty() || trimmed.starts_with("0 ") || trimmed == "0" {
            continue;
        }
        if trimmed.starts_with('$') && trimmed.contains('=') {
            continue;
        }

        // Parse primitive statement: code,exposure,...
        let parts: Vec<&str> = trimmed.split(',').collect();
        if parts.len() >= 2 {
            let exposure_str = parts[1].trim();
            // Check if exposure is explicitly 0 or 0.0
            if exposure_str == "0" || exposure_str == "0.0" {
                return true;
            }
        }
    }
    false
}

/// Parse Aperture definition - %ADD{code}{shape}{params}*%
/// Example: %ADD10C,0.20*% (circle), %ADD20R,0.5X0.3*% (rectangle), %ADD30TESTMACRO*1.5*%
fn parse_aperture(
    data: &str,
    apertures: &mut HashMap<String, Aperture>,
    macros: &HashMap<String, ApertureMacro>,
    unit_multiplier: f32,
) {
    // Format: %ADD{code}{shape},{params}*%
    // Remove %ADD and %
    let content = data
        .trim_start_matches('%')
        .trim_start_matches("ADD")
        .trim_end_matches('%');

    // Split code and shape/params by first letter that's not a digit
    let mut code_end = 0;
    for (i, ch) in content.chars().enumerate() {
        if !ch.is_ascii_digit() {
            code_end = i;
            break;
        }
    }

    if code_end == 0 {
        return;
    }

    let code = content[..code_end].to_string();
    let rest = &content[code_end..];

    // Split shape and parameters by comma or *
    let shape_and_params: Vec<&str> = rest.split(|c: char| c == ',' || c == '*').collect();
    if shape_and_params.is_empty() {
        return;
    }

    let shape = shape_and_params[0].trim().to_string();
    let mut aperture = Aperture::new(code.clone(), shape.clone(), 0.0);

    // Process basic Aperture formats (C, R, O, P)
    match shape.as_str() {
        "C" => {
            // Circle: %ADD10C,0.20*%
            if shape_and_params.len() > 1 {
                if let Ok(diameter) = shape_and_params[1].trim().parse::<f32>() {
                    let diameter_mm = diameter * unit_multiplier;
                    aperture.radius = diameter_mm / 2.0;
                    aperture.primitives.push(Primitive::Circle {
                        x: 0.0,
                        y: 0.0,
                        radius: diameter_mm / 2.0,
                        exposure: 1.0,
                    });
                }
            }
        }
        "R" => {
            // Rectangle: %ADD20R,0.5X0.3*%
            if shape_and_params.len() > 1 {
                let params: Vec<&str> = shape_and_params[1].split('X').collect();
                if params.len() >= 2 {
                    if let (Ok(width), Ok(height)) = (
                        params[0].trim().parse::<f32>(),
                        params[1].trim().parse::<f32>(),
                    ) {
                        let width_mm = width * unit_multiplier;
                        let height_mm = height * unit_multiplier;
                        aperture.radius = width_mm.max(height_mm) / 2.0; // Half of the diagonal
                                                                         // Split Rectangle into two triangles
                        let half_width = width_mm / 2.0;
                        let half_height = height_mm / 2.0;

                        let v1 = [-half_width, -half_height];
                        let v2 = [half_width, -half_height];
                        let v3 = [half_width, half_height];
                        let v4 = [-half_width, half_height];

                        aperture.primitives.push(Primitive::Triangle {
                            vertices: vec![v1, v2, v3],
                            exposure: 1.0,
                        });
                        aperture.primitives.push(Primitive::Triangle {
                            vertices: vec![v1, v3, v4],
                            exposure: 1.0,
                        });
                    }
                }
            }
        }
        "O" => {
            // Obround (rounded rectangle): %ADD30O,0.5X0.3*%
            if shape_and_params.len() > 1 {
                let params: Vec<&str> = shape_and_params[1].split('X').collect();
                if params.len() >= 2 {
                    if let (Ok(width), Ok(height)) = (
                        params[0].trim().parse::<f32>(),
                        params[1].trim().parse::<f32>(),
                    ) {
                        let width_mm = width * unit_multiplier;
                        let height_mm = height * unit_multiplier;
                        let short_side = width_mm.min(height_mm);
                        let long_side = width_mm.max(height_mm);
                        let radius = short_side / 2.0;

                        aperture.radius = radius;

                        if width_mm > height_mm {
                            // If width is greater - circles on the left and right, rectangle in the middle
                            let rect_width = long_side - short_side;
                            let half_rect_width = rect_width / 2.0;
                            let half_height = height_mm / 2.0;

                            // Left circle (center: -half_rect_width, 0)
                            aperture.primitives.push(Primitive::Circle {
                                x: -half_rect_width,
                                y: 0.0,
                                radius,
                                exposure: 1.0,
                            });

                            // Right circle (center: half_rect_width, 0)
                            aperture.primitives.push(Primitive::Circle {
                                x: half_rect_width,
                                y: 0.0,
                                radius,
                                exposure: 1.0,
                            });

                            // Central rectangle (2 triangles)
                            let x1 = -half_rect_width;
                            let x2 = half_rect_width;
                            let y1 = -half_height;
                            let y2 = half_height;

                            aperture.primitives.push(Primitive::Triangle {
                                vertices: vec![[x1, y1], [x2, y1], [x1, y2]],
                                exposure: 1.0,
                            });
                            aperture.primitives.push(Primitive::Triangle {
                                vertices: vec![[x2, y1], [x2, y2], [x1, y2]],
                                exposure: 1.0,
                            });
                        } else {
                            // If height is greater - circles on the top and bottom, rectangle in the middle
                            let rect_height = long_side - short_side;
                            let half_rect_height = rect_height / 2.0;
                            let half_width = width_mm / 2.0;

                            // Bottom circle (center: 0, -half_rect_height)
                            aperture.primitives.push(Primitive::Circle {
                                x: 0.0,
                                y: -half_rect_height,
                                radius,
                                exposure: 1.0,
                            });

                            // Top circle (center: 0, half_rect_height)
                            aperture.primitives.push(Primitive::Circle {
                                x: 0.0,
                                y: half_rect_height,
                                radius,
                                exposure: 1.0,
                            });

                            // Central rectangle (2 triangles)
                            let x1 = -half_width;
                            let x2 = half_width;
                            let y1 = -half_rect_height;
                            let y2 = half_rect_height;

                            aperture.primitives.push(Primitive::Triangle {
                                vertices: vec![[x1, y1], [x2, y1], [x1, y2]],
                                exposure: 1.0,
                            });
                            aperture.primitives.push(Primitive::Triangle {
                                vertices: vec![[x2, y1], [x2, y2], [x1, y2]],
                                exposure: 1.0,
                            });
                        }
                    }
                }
            }
        }
        "P" => {
            // Polygon: %ADD40P,0.5X5*% (diameter 0.5, 5-sided)
            if shape_and_params.len() > 1 {
                let params: Vec<&str> = shape_and_params[1].split('X').collect();
                if params.len() >= 2 {
                    if let (Ok(diameter), Ok(num_vertices)) = (
                        params[0].trim().parse::<f32>(),
                        params[1].trim().parse::<f32>(),
                    ) {
                        let diameter_mm = diameter * unit_multiplier;
                        aperture.radius = diameter_mm / 2.0;
                        let radius = diameter_mm / 2.0;
                        let num_vertices = num_vertices as u32;
                        let angle_step = 2.0 * std::f32::consts::PI / num_vertices as f32;

                        // Fan triangulation
                        for i in 0..(num_vertices as usize) {
                            let next_i = (i + 1) % (num_vertices as usize);
                            let angle_i = angle_step * i as f32;
                            let angle_next = angle_step * next_i as f32;

                            let x1 = radius * angle_i.cos();
                            let y1 = radius * angle_i.sin();
                            let x2 = radius * angle_next.cos();
                            let y2 = radius * angle_next.sin();

                            aperture.primitives.push(Primitive::Triangle {
                                vertices: vec![[0.0, 0.0], [x1, y1], [x2, y2]],
                                exposure: 1.0,
                            });
                        }
                    }
                }
            }
        }
        _ => {
            // Macro reference: %ADD30TESTMACRO,1.5*% or %ADD11RoundRect,0.250000X0.600000X...
            // Check if shape is a macro name
            if let Some(macro_def) = macros.get(&shape) {
                // Collect parameters - also handle parameters separated by X
                let mut params = Vec::new();
                for i in 1..shape_and_params.len() {
                    let param_str = shape_and_params[i].trim();
                    if param_str.is_empty() {
                        continue;
                    }

                    // There can be multiple parameters separated by X
                    if param_str.contains('X') {
                        for sub_param in param_str.split('X') {
                            if let Ok(param) = sub_param.trim().parse::<f32>() {
                                // Convert dimension parameters (aperture macro params are dimensions)
                                params.push(param * unit_multiplier);
                            }
                        }
                    } else {
                        if let Ok(param) = param_str.parse::<f32>() {
                            // Convert dimension parameters (aperture macro params are dimensions)
                            params.push(param * unit_multiplier);
                        }
                    }
                }

                // Call Macro instantiate
                aperture.primitives = macro_def.instantiate(&params);
                aperture.radius = 0.0; // For macros, the radius depends on the parameters
            }
        }
    }

    // Calculate has_negative based on actual primitives
    aperture.has_negative = aperture.primitives.iter().any(|p| match p {
        Primitive::Circle { exposure, .. } => *exposure < 0.5,
        Primitive::Triangle { exposure, .. } => *exposure < 0.5,
        Primitive::Arc { exposure, .. } => *exposure < 0.5,
        Primitive::Thermal { exposure, .. } => *exposure < 0.5,
    });

    apertures.insert(code, aperture);
}
/// Parse Gerber data and return Vec of GerberData (one per polarity layer)
/// Order: [pos_layer1, neg_layer1, pos_layer2, neg_layer2, ...]
pub fn parse_gerber(data: &str) -> Result<Vec<GerberData>, JsValue> {
    let mut parser = GerberParser::new();
    parser.parse(data)
}

/// Test: Evaluate expression with negative numbers
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negative_numbers() {
        let mut vars = HashMap::new();
        vars.insert("$1".to_string(), 5.0);
        vars.insert("$2".to_string(), -2.0);
        vars.insert("$3".to_string(), 3.0);

        // $1 - $2 where $2 = -2 → 5 - (-2) = 7
        let result = evaluate_expression("$1-$2", &vars);
        println!("$1-$2 (where $1=5, $2=-2) = {:?}", result);
        assert_eq!(result, Ok(7.0));

        // $1 + $2 where $2 = -2 → 5 + (-2) = 3
        let result = evaluate_expression("$1+$2", &vars);
        println!("$1+$2 (where $1=5, $2=-2) = {:?}", result);
        assert_eq!(result, Ok(3.0));

        // $2 * $3 where $2 = -2, $3 = 3 → -2 * 3 = -6
        let result = evaluate_expression("$2X$3", &vars);
        println!("$2X$3 (where $2=-2, $3=3) = {:?}", result);
        assert_eq!(result, Ok(-6.0));

        // Direct negative number: -5 + 3 = -2
        let result = evaluate_expression("-5+3", &vars);
        println!("-5+3 = {:?}", result);
        assert_eq!(result, Ok(-2.0));

        // Multiple operations: $1 - $2 + $3 = 5 - (-2) + 3 = 10
        let result = evaluate_expression("$1-$2+$3", &vars);
        println!("$1-$2+$3 (where $1=5, $2=-2, $3=3) = {:?}", result);
        assert_eq!(result, Ok(10.0));

        // Negative variable at start: $2 - 1 = -2 - 1 = -3
        let result = evaluate_expression("$2-1", &vars);
        println!("$2-1 (where $2=-2) = {:?}", result);
        assert_eq!(result, Ok(-3.0));

        // Expression: -$1 + $2 = -5 + (-2) = -7
        let result = evaluate_expression("-$1+$2", &vars);
        println!("-$1+$2 (where $1=5, $2=-2) = {:?}", result);
        assert_eq!(result, Ok(-7.0));
    }
}

/// Triangulation result containing both vertices and triangle indices
#[wasm_bindgen]
pub struct TriangulationResult {
    points: Vec<f32>,
    indices: Vec<u32>,
}

#[wasm_bindgen]
impl TriangulationResult {
    #[wasm_bindgen(getter)]
    pub fn points(&self) -> Vec<f32> {
        self.points.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn indices(&self) -> Vec<u32> {
        self.indices.clone()
    }
}

/// Triangulate a polygon with optional holes
///
/// # Arguments
/// * `flat_vertices` - Flattened vertex coordinates [x1, y1, x2, y2, ...]
/// * `hole_indices` - Indices where holes start in the vertex array
///
/// # Returns
/// * `TriangulationResult` containing triangulated vertices and indices
#[wasm_bindgen]
pub fn triangulate_polygon(
    flat_vertices: Vec<f32>,
    hole_indices: Vec<u32>,
) -> Result<TriangulationResult, JsValue> {
    if flat_vertices.len() < 6 {
        return Ok(TriangulationResult {
            points: Vec::new(),
            indices: Vec::new(),
        });
    }

    if !flat_vertices.len().is_multiple_of(2) {
        return Err(JsValue::from_str("flat_vertices length must be even"));
    }

    // Convert flat vertices to Vec<Vec<[f32; 2]>> format
    let mut shape: Vec<Vec<[f32; 2]>> = Vec::new();

    if hole_indices.is_empty() {
        // No holes, just the main path
        let mut path: Vec<[f32; 2]> = Vec::new();
        for i in (0..flat_vertices.len()).step_by(2) {
            path.push([flat_vertices[i], flat_vertices[i + 1]]);
        }
        shape.push(path);
    } else {
        // First hole index marks the end of main path
        let first_hole_start = hole_indices[0] as usize;
        let mut main_path: Vec<[f32; 2]> = Vec::new();
        for i in (0..first_hole_start * 2).step_by(2) {
            main_path.push([flat_vertices[i], flat_vertices[i + 1]]);
        }
        shape.push(main_path);

        // Add holes
        for i in 0..hole_indices.len() {
            let hole_start = hole_indices[i] as usize;
            let hole_end = if i + 1 < hole_indices.len() {
                hole_indices[i + 1] as usize
            } else {
                flat_vertices.len() / 2
            };

            let mut hole_path: Vec<[f32; 2]> = Vec::new();
            for j in (hole_start * 2..hole_end * 2).step_by(2) {
                hole_path.push([flat_vertices[j], flat_vertices[j + 1]]);
            }
            shape.push(hole_path);
        }
    }

    // Perform triangulation
    let triangulation = shape.triangulate().to_triangulation::<u32>();

    // Flatten points from [[f32; 2]] to [f32]
    let mut flat_points: Vec<f32> = Vec::new();
    for point in triangulation.points {
        flat_points.push(point[0]);
        flat_points.push(point[1]);
    }

    Ok(TriangulationResult {
        points: flat_points,
        indices: triangulation.indices,
    })
}

/// Parse Format specification - %FSLAX24Y24*%
/// Format: %FS[L|T][A|I][X_int_digits][X_dec_digits][Y_int_digits][Y_dec_digits]*%
/// Example: %FSLAX24Y24*% = Leading, Absolute, 2 integer digits + 4 decimal digits
fn parse_format_spec(line: &str, state: &mut ParserState) {
    // Extract FSLAX24Y24 part from %FSLAX24Y24*% format
    let spec_str = line
        .trim_start_matches('%')
        .trim_end_matches('%')
        .trim_end_matches('*');

    if !spec_str.starts_with("FS") {
        return;
    }

    let spec_content = &spec_str[2..]; // "LAX24Y24" part

    // Position tracking
    let mut pos = 0;
    let chars: Vec<char> = spec_content.chars().collect();

    if pos >= chars.len() {
        return;
    }

    // L/T: Leading (L) or Trailing (T) zeros
    let _leading_type = chars[pos];
    pos += 1;

    if pos >= chars.len() {
        return;
    }

    // A/I: Absolute (A) or Incremental (I) mode
    let mode = chars[pos];
    pos += 1;

    // X coordinates format: X + digits
    if pos < chars.len() && chars[pos] == 'X' {
        pos += 1;
        if pos + 1 < chars.len() {
            if let (Ok(int_digits), Ok(dec_digits)) = (
                chars[pos].to_string().parse::<u32>(),
                chars[pos + 1].to_string().parse::<u32>(),
            ) {
                state.format_spec.x_integer_digits = int_digits;
                state.format_spec.x_decimal_digits = dec_digits;
            }
            pos += 2;
        }
    }

    // Y coordinates format: Y + digits
    if pos < chars.len() && chars[pos] == 'Y' {
        pos += 1;
        if pos + 1 < chars.len() {
            if let (Ok(int_digits), Ok(dec_digits)) = (
                chars[pos].to_string().parse::<u32>(),
                chars[pos + 1].to_string().parse::<u32>(),
            ) {
                state.format_spec.y_integer_digits = int_digits;
                state.format_spec.y_decimal_digits = dec_digits;
            }
        }
    }

    // Save mode
    if mode == 'I' {
        state.coordinate_mode = "incremental".to_string();
    } else {
        state.coordinate_mode = "absolute".to_string();
    }

    // Calculate cached values - performance optimization
    state.format_spec.x_divisor = 10_f64.powi(state.format_spec.x_decimal_digits as i32);
    state.format_spec.y_divisor = 10_f64.powi(state.format_spec.y_decimal_digits as i32);
    state.format_spec.x_total_digits =
        (state.format_spec.x_integer_digits + state.format_spec.x_decimal_digits) as i32;
    state.format_spec.y_total_digits =
        (state.format_spec.y_integer_digits + state.format_spec.y_decimal_digits) as i32;
}

/// Parse Step and Repeat - %SRX3Y2I10J20*%
/// Format: %SR[X count][Y count][I x_step][J y_step]*%
fn parse_sr(line: &str, state: &mut ParserState) {
    // Extract SRX3Y2I10J20 part from %SRX3Y2I10J20*% format
    let spec_str = line
        .trim_start_matches('%')
        .trim_end_matches('%')
        .trim_end_matches('*');

    if !spec_str.starts_with("SR") {
        return;
    }

    let spec_content = &spec_str[2..]; // "X3Y2I10J20" part

    // Extract X count
    if let Some(x_pos) = spec_content.find('X') {
        let x_part = &spec_content[x_pos + 1..];
        let x_end = x_part
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(x_part.len());
        if let Ok(x_count) = x_part[..x_end].parse::<u32>() {
            state.sr_x = x_count;
        }
    }

    // Extract Y count
    if let Some(y_pos) = spec_content.find('Y') {
        let y_part = &spec_content[y_pos + 1..];
        let y_end = y_part
            .find(|c: char| !c.is_ascii_digit() && c != '-' && c != '.')
            .unwrap_or(y_part.len());
        if let Ok(y_count) = y_part[..y_end].parse::<u32>() {
            state.sr_y = y_count;
        }
    }

    // Extract I step (X direction spacing)
    if let Some(i_pos) = spec_content.find('I') {
        let i_part = &spec_content[i_pos + 1..];
        let i_end = i_part
            .find(|c: char| !c.is_ascii_digit() && c != '-' && c != '.')
            .unwrap_or(i_part.len());
        if let Ok(i_step) = i_part[..i_end].parse::<f32>() {
            state.sr_i = i_step;
        }
    }

    // Extract J step (Y direction spacing)
    if let Some(j_pos) = spec_content.find('J') {
        let j_part = &spec_content[j_pos + 1..];
        let j_end = j_part
            .find(|c: char| !c.is_ascii_digit() && c != '-' && c != '.')
            .unwrap_or(j_part.len());
        if let Ok(j_step) = j_part[..j_end].parse::<f32>() {
            state.sr_j = j_step;
        }
    }
}

/// Parse Polarity - %LPD* (positive) or %LPC* (negative)
/// Save the primitives accumulated so far each time the polarity changes
fn parse_lp(
    line: &str,
    state: &mut ParserState,
    current_primitives: &mut Vec<Primitive>,
    positive_layers: &mut Vec<Vec<Primitive>>,
    negative_layers: &mut Vec<Vec<Primitive>>,
) {
    // Extract D or C from %LPD* or %LPC* format
    let spec_str = line
        .trim_start_matches('%')
        .trim_end_matches('%')
        .trim_end_matches('*');

    if !spec_str.starts_with("LP") {
        return;
    }

    let polarity_char = spec_str.chars().nth(2);

    let new_polarity = match polarity_char {
        Some('D') => Polarity::Positive, // Dark mode
        Some('C') => Polarity::Negative, // Clear mode
        _ => return,
    };

    // Check if polarity has changed
    if state.polarity != new_polarity && !current_primitives.is_empty() {
        // Save to layer according to current polarity
        if state.polarity == Polarity::Positive {
            positive_layers.push(current_primitives.clone());
        } else {
            negative_layers.push(current_primitives.clone());
        }
        // Initialize primitives for new polarity
        current_primitives.clear();
    }

    // Set new polarity
    state.polarity = new_polarity;
}

fn parse_if(line: &str, state: &mut ParserState) {
    // %IFPOS*% or %IFNEG*% format
    let spec_str = line
        .trim_start_matches('%')
        .trim_end_matches('%')
        .trim_end_matches('*');

    if !spec_str.starts_with("IF") {
        return;
    }

    let polarity_str = &spec_str[2..]; // "POS" or "NEG" part

    let new_polarity = if polarity_str == "POS" {
        Polarity::Positive
    } else if polarity_str == "NEG" {
        Polarity::Negative
    } else {
        return;
    };

    state.polarity = new_polarity;
}

/// Parse Unit mode - %MOMM* (millimeters) or %MOIN* (inches)
fn parse_mo(line: &str, state: &mut ParserState) {
    // Extract MM or IN from %MOMM*% or %MOIN*% format
    let spec_str = line
        .trim_start_matches('%')
        .trim_end_matches('%')
        .trim_end_matches('*');

    if !spec_str.starts_with("MO") {
        return;
    }

    let unit_str = &spec_str[2..]; // "MM" or "IN" part

    state.unit_multiplier = if unit_str == "MM" {
        1.0 // mm
    } else if unit_str == "IN" {
        25.4 // inch to mm
    } else {
        return;
    };
}
