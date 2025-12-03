mod features;
mod matrix;
mod symbols;

pub use features::parse_features;
pub use symbols::{parse_symbols, Symbol};

use crate::shape::{Arcs, Boundary, Circles, GerberData, Thermals, Triangles};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use self::features::Primitive;

/// ODB++ Parser with symbol and feature storage
pub struct OdbParser {
    pub symbols: HashMap<String, Symbol>,
    pub current_primitives: Vec<Primitive>,
}

impl OdbParser {
    /// Create new ODB++ parser instance
    pub fn new() -> Self {
        OdbParser {
            symbols: HashMap::new(),
            current_primitives: Vec::new(),
        }
    }

    /// Parse ODB++ features and convert to GerberData
    ///
    /// # Arguments
    /// * `features_content` - Features file content
    /// * `symbols_content` - Symbols definition content
    ///
    /// # Returns
    /// * `GerberData` containing parsed geometry
    pub fn parse(&mut self, features_content: &str, symbols_content: &str) -> Result<GerberData, JsValue> {
        // Parse symbols first
        self.symbols = parse_symbols(symbols_content)?;

        // Parse features and generate primitives
        self.current_primitives = parse_features(features_content, &self.symbols)?;

        // Convert primitives to GerberData structures
        self.convert_to_gerber_data()
    }

    /// Convert Primitive list to GerberData with Circles, Triangles, Arcs, Thermals
    fn convert_to_gerber_data(&self) -> Result<GerberData, JsValue> {
        let mut triangles_vertices = Vec::new();
        let mut triangles_indices = Vec::new();
        let mut triangles_holes_x = Vec::new();
        let mut triangles_holes_y = Vec::new();
        let mut triangles_holes_radius = Vec::new();

        let mut circles_x = Vec::new();
        let mut circles_y = Vec::new();
        let mut circles_radius = Vec::new();
        let mut circles_holes_x = Vec::new();
        let mut circles_holes_y = Vec::new();
        let mut circles_holes_radius = Vec::new();

        let mut arcs_x = Vec::new();
        let mut arcs_y = Vec::new();
        let mut arcs_radius = Vec::new();
        let mut arcs_start_angle = Vec::new();
        let mut arcs_sweep_angle = Vec::new();
        let mut arcs_thickness = Vec::new();

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        // Convert each primitive
        for primitive in &self.current_primitives {
            match primitive {
                Primitive::Circle {
                    x,
                    y,
                    radius,
                } => {
                    circles_x.push(*x);
                    circles_y.push(*y);
                    circles_radius.push(*radius);
                    circles_holes_x.push(0.0);
                    circles_holes_y.push(0.0);
                    circles_holes_radius.push(0.0);

                    min_x = min_x.min(x - radius);
                    max_x = max_x.max(x + radius);
                    min_y = min_y.min(y - radius);
                    max_y = max_y.max(y + radius);
                }
                Primitive::Triangle { vertices } => {
                    let index_offset = (triangles_vertices.len() / 2) as u32;
                    for vertex in vertices {
                        triangles_vertices.push(vertex[0]);
                        triangles_vertices.push(vertex[1]);

                        min_x = min_x.min(vertex[0]);
                        max_x = max_x.max(vertex[0]);
                        min_y = min_y.min(vertex[1]);
                        max_y = max_y.max(vertex[1]);
                    }

                    // Add triangle indices
                    if vertices.len() >= 3 {
                        triangles_indices.push(index_offset);
                        triangles_indices.push(index_offset + 1);
                        triangles_indices.push(index_offset + 2);
                    }

                    triangles_holes_x.push(0.0);
                    triangles_holes_y.push(0.0);
                    triangles_holes_radius.push(0.0);
                }
                Primitive::Arc {
                    x,
                    y,
                    radius,
                    start_angle,
                    sweep_angle,
                    thickness,
                } => {
                    arcs_x.push(*x);
                    arcs_y.push(*y);
                    arcs_radius.push(*radius);
                    arcs_start_angle.push(*start_angle);
                    arcs_sweep_angle.push(*sweep_angle);
                    arcs_thickness.push(*thickness);

                    min_x = min_x.min(x - radius - thickness / 2.0);
                    max_x = max_x.max(x + radius + thickness / 2.0);
                    min_y = min_y.min(y - radius - thickness / 2.0);
                    max_y = max_y.max(y + radius + thickness / 2.0);
                }
            }
        }

        // Handle empty geometry
        if triangles_vertices.is_empty()
            && circles_x.is_empty()
            && arcs_x.is_empty()
        {
            min_x = 0.0;
            max_x = 0.0;
            min_y = 0.0;
            max_y = 0.0;
        }

        let triangles = Triangles::new(
            triangles_vertices,
            triangles_indices,
            triangles_holes_x,
            triangles_holes_y,
            triangles_holes_radius,
        );

        let circles = Circles::new(
            circles_x,
            circles_y,
            circles_radius,
            circles_holes_x,
            circles_holes_y,
            circles_holes_radius,
        );

        let arcs = Arcs::new(
            arcs_x,
            arcs_y,
            arcs_radius,
            arcs_start_angle,
            arcs_sweep_angle,
            arcs_thickness,
        );

        let thermals = Thermals::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );

        let boundary = Boundary::new(min_x, max_x, min_y, max_y);

        Ok(GerberData::new(triangles, circles, arcs, thermals, boundary))
    }
}

impl Default for OdbParser {
    fn default() -> Self {
        Self::new()
    }
}
