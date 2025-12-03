use super::symbols::{Symbol, SymbolShape};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Primitive shape created from ODB++ features
/// Matches Gerber primitive structure with exposure field for polarity
#[derive(Clone, Debug)]
pub enum Primitive {
    Circle {
        x: f32,
        y: f32,
        radius: f32,
        exposure: f32,     // 1.0 = positive, 0.0 = negative
        hole_x: f32,       // Hole center X (absolute position)
        hole_y: f32,       // Hole center Y (absolute position)
        hole_radius: f32,  // Hole radius (0.0 = no hole)
    },
    Triangle {
        vertices: Vec<[f32; 2]>,
        exposure: f32,     // 1.0 = positive, 0.0 = negative
        hole_x: f32,       // Hole center X (relative to triangle)
        hole_y: f32,       // Hole center Y (relative to triangle)
        hole_radius: f32,  // Hole radius (0.0 = no hole)
    },
    Arc {
        x: f32,
        y: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        thickness: f32,
        exposure: f32,     // 1.0 = positive, 0.0 = negative
    },
}

/// Parse features file and convert to primitives
///
/// # Format
/// ```
/// #
/// #Feature symbol names
/// $0 r200
/// $1 s300
/// #
/// #Feature data
/// P 1000 2000 0 0 0 $0 0 0
/// P 3000 4000 0 0 0 $1 0 0
/// L 5000 6000 7000 8000 0 0 0 $0 0 0
/// A 5000 5000 1000 0 180 0 0 0 $0 0 0
/// ```
pub fn parse_features(
    content: &str,
    symbols: &HashMap<String, Symbol>,
) -> Result<Vec<Primitive>, JsValue> {
    let mut primitives = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let feature_type = parts[0];
        match feature_type {
            "P" => {
                if let Ok(primitive) = parse_pad(&parts, symbols) {
                    primitives.push(primitive);
                }
            }
            "L" => {
                if let Ok(primitive) = parse_line(&parts, symbols) {
                    primitives.push(primitive);
                }
            }
            "A" => {
                if let Ok(primitive) = parse_arc(&parts, symbols) {
                    primitives.push(primitive);
                }
            }
            "S" => {
                if let Ok(prim_list) = parse_surface(&parts) {
                    primitives.extend(prim_list);
                }
            }
            _ => {
                // Unknown feature type, skip
            }
        }
    }

    Ok(primitives)
}

/// Parse Pad (P) feature: P <x> <y> <rotation> <mirror_x> <mirror_y> <symbol> <polarity> <attributes>
fn parse_pad(
    parts: &[&str],
    symbols: &HashMap<String, Symbol>,
) -> Result<Primitive, JsValue> {
    if parts.len() < 8 {
        return Err(JsValue::from_str("Invalid Pad format"));
    }

    let x = parts[1]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Pad X coordinate"))?;
    let y = parts[2]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Pad Y coordinate"))?;
    let _rotation = parts[3]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Pad rotation"))?;
    let _mirror_x = parts[4]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Pad mirror_x"))?;
    let _mirror_y = parts[5]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Pad mirror_y"))?;
    let symbol_id = parts[6];
    let polarity_str = parts[7];

    // Parse polarity: 0 = positive, 1 = negative (or similar)
    let exposure = if polarity_str == "0" || polarity_str == "1" {
        1.0
    } else {
        0.0
    };

    let symbol = symbols
        .get(symbol_id)
        .ok_or(JsValue::from_str("Symbol not found"))?;

    match &symbol.shape {
        SymbolShape::Round(diameter) => {
            Ok(Primitive::Circle {
                x,
                y,
                radius: diameter / 2.0,
                exposure,
                hole_x: 0.0,
                hole_y: 0.0,
                hole_radius: 0.0,
            })
        }
        SymbolShape::Square(size) => {
            // Convert square to triangles
            let half = size / 2.0;
            let v1 = [x - half, y - half];
            let v2 = [x + half, y - half];
            let v3 = [x + half, y + half];
            let _v4 = [x - half, y + half];

            // Return first triangle: (v1, v2, v3)
            Ok(Primitive::Triangle {
                vertices: vec![v1, v2, v3],
                exposure,
                hole_x: 0.0,
                hole_y: 0.0,
                hole_radius: 0.0,
            })
        }
        SymbolShape::Rectangle(width, height) => {
            // Convert rectangle to triangles
            let half_w = width / 2.0;
            let half_h = height / 2.0;
            let v1 = [x - half_w, y - half_h];
            let v2 = [x + half_w, y - half_h];
            let v3 = [x + half_w, y + half_h];
            let _v4 = [x - half_w, y + half_h];

            // Return first triangle: (v1, v2, v3)
            Ok(Primitive::Triangle {
                vertices: vec![v1, v2, v3],
                exposure,
                hole_x: 0.0,
                hole_y: 0.0,
                hole_radius: 0.0,
            })
        }
        SymbolShape::Obround(width, height) => {
            // For simplicity, treat as a circle with average radius
            let avg_radius = (width + height) / 4.0;
            Ok(Primitive::Circle {
                x,
                y,
                radius: avg_radius,
                exposure,
                hole_x: 0.0,
                hole_y: 0.0,
                hole_radius: 0.0,
            })
        }
        SymbolShape::Polygon(sides, diameter) => {
            let radius = diameter / 2.0;
            let vertices = generate_polygon_vertices(x, y, *sides, radius);
            Ok(Primitive::Triangle {
                vertices,
                exposure,
                hole_x: 0.0,
                hole_y: 0.0,
                hole_radius: 0.0,
            })
        }
    }
}

/// Parse Arc (A) feature: A <cx> <cy> <radius> <start_angle> <sweep_angle> <width> <polarity> <attributes>
fn parse_arc(parts: &[&str], _symbols: &HashMap<String, Symbol>) -> Result<Primitive, JsValue> {
    if parts.len() < 8 {
        return Err(JsValue::from_str("Invalid Arc format"));
    }

    let x = parts[1]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Arc center X"))?;
    let y = parts[2]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Arc center Y"))?;
    let radius = parts[3]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Arc radius"))?;
    let start_angle = parts[4]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Arc start angle"))?;
    let sweep_angle = parts[5]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Arc sweep angle"))?;
    let thickness = parts[6]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Arc thickness"))?;
    let polarity_str = parts[7];

    // Parse polarity
    let exposure = if polarity_str == "0" || polarity_str == "1" {
        1.0
    } else {
        0.0
    };

    // Calculate end angle from sweep angle
    let end_angle = start_angle + sweep_angle;

    Ok(Primitive::Arc {
        x,
        y,
        radius,
        start_angle,
        end_angle,
        thickness,
        exposure,
    })
}

/// Parse Line (L) feature: L <x1> <y1> <x2> <y2> <width> <symbol> <polarity> <attributes>
fn parse_line(
    parts: &[&str],
    symbols: &HashMap<String, Symbol>,
) -> Result<Primitive, JsValue> {
    if parts.len() < 8 {
        return Err(JsValue::from_str("Invalid Line format"));
    }

    let x1 = parts[1]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Line X1 coordinate"))?;
    let y1 = parts[2]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Line Y1 coordinate"))?;
    let x2 = parts[3]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Line X2 coordinate"))?;
    let y2 = parts[4]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Line Y2 coordinate"))?;
    let width = parts[5]
        .parse::<f32>()
        .map_err(|_| JsValue::from_str("Invalid Line width"))?;
    let symbol_id = parts[6];
    let polarity_str = parts[7];

    let _symbol = symbols
        .get(symbol_id)
        .ok_or(JsValue::from_str("Symbol not found"))?;

    // Parse polarity
    let exposure = if polarity_str == "0" || polarity_str == "1" {
        1.0
    } else {
        0.0
    };

    // Create line as 2 triangles (like Gerber line_to_triangles)
    let dx = x2 - x1;
    let dy = y2 - y1;
    let length = (dx * dx + dy * dy).sqrt();

    if length < 0.001 {
        // Degenerate line, skip it
        return Err(JsValue::from_str("Degenerate line"));
    }

    // Perpendicular vector (width direction)
    let half_width = width / 2.0;
    let perp_x = -dy / length * half_width;
    let perp_y = dx / length * half_width;

    // 4 vertices on both sides of the line
    let v1 = [x1 + perp_x, y1 + perp_y];
    let v2 = [x1 - perp_x, y1 - perp_y];
    let v3 = [x2 + perp_x, y2 + perp_y];
    let _v4 = [x2 - perp_x, y2 - perp_y];

    // Return first triangle: (v1, v2, v3)
    Ok(Primitive::Triangle {
        vertices: vec![v1, v2, v3],
        exposure,
        hole_x: 0.0,
        hole_y: 0.0,
        hole_radius: 0.0,
    })
}

/// Parse Surface (S) feature: S <vertices>... <polarity> <attributes>
/// Surface format: S x1 y1 x2 y2 x3 y3 ... xN yN [polarity] [attributes]
fn parse_surface(parts: &[&str]) -> Result<Vec<Primitive>, JsValue> {
    if parts.len() < 7 {
        return Err(JsValue::from_str("Invalid Surface format"));
    }

    // Parse vertex coordinates (parts[1] to some point)
    // Last two elements might be polarity and attributes
    let mut vertices = Vec::new();
    let mut i = 1;

    // Parse pairs of coordinates until we hit a non-numeric value or run out
    while i + 1 < parts.len() {
        if let (Ok(x), Ok(y)) = (parts[i].parse::<f32>(), parts[i + 1].parse::<f32>()) {
            vertices.push([x, y]);
            i += 2;
        } else {
            break;
        }
    }

    if vertices.len() < 3 {
        return Err(JsValue::from_str("Surface needs at least 3 vertices"));
    }

    // Get polarity from remaining parts
    let polarity_str = if i < parts.len() { parts[i] } else { "0" };
    let exposure = if polarity_str == "0" || polarity_str == "1" {
        1.0
    } else {
        0.0
    };

    // Simple triangulation: create triangle from first three vertices and fan out
    let mut primitives = Vec::new();
    for j in 1..vertices.len() - 1 {
        let triangle_vertices = vec![vertices[0], vertices[j], vertices[j + 1]];
        primitives.push(Primitive::Triangle {
            vertices: triangle_vertices,
            exposure,
            hole_x: 0.0,
            hole_y: 0.0,
            hole_radius: 0.0,
        });
    }

    Ok(primitives)
}

/// Generate vertices for a regular polygon
fn generate_polygon_vertices(cx: f32, cy: f32, sides: u32, radius: f32) -> Vec<[f32; 2]> {
    let mut vertices = Vec::new();
    let angle_step = 2.0 * std::f32::consts::PI / sides as f32;

    for i in 0..sides {
        let angle = i as f32 * angle_step;
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();
        vertices.push([x, y]);
    }

    vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pad() {
        let symbols = {
            let mut m = HashMap::new();
            m.insert(
                "$0".to_string(),
                Symbol {
                    id: "$0".to_string(),
                    shape: SymbolShape::Round(200.0),
                },
            );
            m
        };

        let parts = vec!["P", "1000", "2000", "0", "0", "0", "$0", "0", "0"];
        let result = parse_pad(&parts, &symbols);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_line() {
        let symbols = {
            let mut m = HashMap::new();
            m.insert(
                "$0".to_string(),
                Symbol {
                    id: "$0".to_string(),
                    shape: SymbolShape::Round(200.0),
                },
            );
            m
        };

        let parts = vec!["L", "1000", "2000", "3000", "4000", "100", "$0", "0", "0"];
        let result = parse_line(&parts, &symbols);
        assert!(result.is_ok());
    }
}
