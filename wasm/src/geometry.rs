use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use i_triangle::float::triangulatable::Triangulatable;

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

/// Rotate point around given center
#[inline]
pub fn rotate_point(point: &mut [f32; 2], angle: f32, center_x: f32, center_y: f32) {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let x = point[0] - center_x;
    let y = point[1] - center_y;
    point[0] = center_x + x * cos_a - y * sin_a;
    point[1] = center_y + x * sin_a + y * cos_a;
}

/// Triangulate outline into triangles
pub fn triangulate_outline(vertices: &[[f32; 2]], exposure: f32) -> Result<Vec<Primitive>, String> {
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
pub fn line_to_triangles(
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

/// Convert a primitive to a polygon (outer boundary as Vec<[f32; 2]>)
pub fn primitive_to_polygon(primitive: &Primitive) -> Vec<[f32; 2]> {
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

/// Apply sequential boolean operations to shapes (new version using Shape format)
/// Input: Vec<(Shape, exposure)> where Shape is Vec<Contour> and Contour is Vec<Point>
/// Returns: Vec<Primitive::Triangle> with all triangulated results
pub fn apply_boolean_operations(shapes: &[(Vec<Vec<[f32; 2]>>, f32)]) -> Vec<Primitive> {
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
    for (i, (shape, exposure)) in shapes.iter().enumerate() {
        if i == first_idx {
            continue; // Skip the first shape we already added
        }

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
pub fn triangulate_shape_with_holes(
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
pub fn offset_primitive_by(primitive: &Primitive, dx: f32, dy: f32) -> Primitive {
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
