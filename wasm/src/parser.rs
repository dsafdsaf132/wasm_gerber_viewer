use crate::aperture::{parse_aperture, Aperture};
use crate::aperture_macro::{parse_macro, ApertureMacro};
use crate::geometry::{parse_graphic_command, Primitive};
use crate::parser_state::{
    parse_format_spec, parse_if, parse_lp, parse_mo, parse_sr, ParserState, Polarity,
};
use crate::shape::{Arcs, Boundary, Circles, GerberData, Thermals, Triangles};
use std::collections::HashMap;
use std::mem::take;
use wasm_bindgen::prelude::*;

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
                self.positive_layers
                    .push(take(&mut self.current_primitives));
            } else {
                self.negative_layers
                    .push(take(&mut self.current_primitives));
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

pub fn parse_gerber(data: &str) -> Result<Vec<GerberData>, JsValue> {
    let mut parser = GerberParser::new();
    parser.parse(data)
}
