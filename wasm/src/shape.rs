use wasm_bindgen::prelude::*;


/// Triangle mesh data structure
#[wasm_bindgen]
#[derive(Clone)]
pub struct Triangles {
    pub(crate) vertices: Vec<f32>,
    pub(crate) indices: Vec<u32>,
}

#[wasm_bindgen]
impl Triangles {
    #[wasm_bindgen(constructor)]
    pub fn new(vertices: Vec<f32>, indices: Vec<u32>) -> Triangles {
        Triangles { vertices, indices }
    }

    #[wasm_bindgen(getter)]
    pub fn vertices(&self) -> Vec<f32> {
        self.vertices.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn indices(&self) -> Vec<u32> {
        self.indices.clone()
    }
}


/// Circle primitive data structure
#[wasm_bindgen]
#[derive(Clone)]
pub struct Circles {
    pub(crate) x: Vec<f32>,
    pub(crate) y: Vec<f32>,
    pub(crate) radius: Vec<f32>,
}

#[wasm_bindgen]
impl Circles {
    #[wasm_bindgen(constructor)]
    pub fn new(x: Vec<f32>, y: Vec<f32>, radius: Vec<f32>) -> Circles {
        Circles { x, y, radius }
    }

    #[wasm_bindgen(getter)]
    pub fn x(&self) -> Vec<f32> {
        self.x.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> Vec<f32> {
        self.y.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn radius(&self) -> Vec<f32> {
        self.radius.clone()
    }
}


/// Arc primitive data structure
#[wasm_bindgen]
#[derive(Clone)]
pub struct Arcs {
    pub(crate) x: Vec<f32>,
    pub(crate) y: Vec<f32>,
    pub(crate) radius: Vec<f32>,
    pub(crate) start_angle: Vec<f32>,
    pub(crate) sweep_angle: Vec<f32>,
    pub(crate) thickness: Vec<f32>,
}

#[wasm_bindgen]
impl Arcs {
    #[wasm_bindgen(constructor)]
    pub fn new(
        x: Vec<f32>,
        y: Vec<f32>,
        radius: Vec<f32>,
        start_angle: Vec<f32>,
        sweep_angle: Vec<f32>,
        thickness: Vec<f32>,
    ) -> Arcs {
        Arcs {
            x,
            y,
            radius,
            start_angle,
            sweep_angle,
            thickness,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn x(&self) -> Vec<f32> {
        self.x.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> Vec<f32> {
        self.y.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn radius(&self) -> Vec<f32> {
        self.radius.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn start_angle(&self) -> Vec<f32> {
        self.start_angle.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn sweep_angle(&self) -> Vec<f32> {
        self.sweep_angle.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn thickness(&self) -> Vec<f32> {
        self.thickness.clone()
    }
}


/// Thermal primitive data structure
#[wasm_bindgen]
#[derive(Clone)]
pub struct Thermals {
    pub(crate) x: Vec<f32>,
    pub(crate) y: Vec<f32>,
    pub(crate) outer_diameter: Vec<f32>,
    pub(crate) inner_diameter: Vec<f32>,
    pub(crate) gap_thickness: Vec<f32>,
    pub(crate) rotation: Vec<f32>,
}

#[wasm_bindgen]
impl Thermals {
    #[wasm_bindgen(constructor)]
    pub fn new(
        x: Vec<f32>,
        y: Vec<f32>,
        outer_diameter: Vec<f32>,
        inner_diameter: Vec<f32>,
        gap_thickness: Vec<f32>,
        rotation: Vec<f32>,
    ) -> Thermals {
        Thermals {
            x,
            y,
            outer_diameter,
            inner_diameter,
            gap_thickness,
            rotation,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn x(&self) -> Vec<f32> {
        self.x.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> Vec<f32> {
        self.y.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn outer_diameter(&self) -> Vec<f32> {
        self.outer_diameter.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn inner_diameter(&self) -> Vec<f32> {
        self.inner_diameter.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn gap_thickness(&self) -> Vec<f32> {
        self.gap_thickness.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn rotation(&self) -> Vec<f32> {
        self.rotation.clone()
    }
}


/// Boundary information for the entire Gerber layer
#[wasm_bindgen]
#[derive(Clone)]
pub struct Boundary {
    pub(crate) min_x: f32,
    pub(crate) max_x: f32,
    pub(crate) min_y: f32,
    pub(crate) max_y: f32,
}

#[wasm_bindgen]
impl Boundary {
    #[wasm_bindgen(constructor)]
    pub fn new(min_x: f32, max_x: f32, min_y: f32, max_y: f32) -> Boundary {
        Boundary {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn min_x(&self) -> f32 {
        self.min_x
    }

    #[wasm_bindgen(getter)]
    pub fn max_x(&self) -> f32 {
        self.max_x
    }

    #[wasm_bindgen(getter)]
    pub fn min_y(&self) -> f32 {
        self.min_y
    }

    #[wasm_bindgen(getter)]
    pub fn max_y(&self) -> f32 {
        self.max_y
    }
}

/// Container for all parsed Gerber data
#[wasm_bindgen]
#[derive(Clone)]
pub struct GerberData {
    pub(crate) triangles: Triangles,
    pub(crate) circles: Circles,
    pub(crate) arcs: Arcs,
    pub(crate) thermals: Thermals,
    pub(crate) boundary: Boundary,
}

#[wasm_bindgen]
impl GerberData {
    #[wasm_bindgen(constructor)]
    pub fn new(
        triangles: Triangles,
        circles: Circles,
        arcs: Arcs,
        thermals: Thermals,
        boundary: Boundary,
    ) -> GerberData {
        GerberData {
            triangles,
            circles,
            arcs,
            thermals,
            boundary,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn triangles(&self) -> Triangles {
        self.triangles.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn circles(&self) -> Circles {
        self.circles.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn arcs(&self) -> Arcs {
        self.arcs.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn thermals(&self) -> Thermals {
        self.thermals.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn boundary(&self) -> Boundary {
        self.boundary.clone()
    }
}
