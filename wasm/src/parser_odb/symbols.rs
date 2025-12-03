use wasm_bindgen::prelude::*;

/// Symbol shape definition
#[derive(Clone, Debug)]
pub enum SymbolShape {
    Round(f32),              // r<diameter>
    Square(f32),             // s<size>
    Rectangle(f32, f32),     // r<width>x<height>
    Obround(f32, f32),       // o<width>x<height>
    Polygon(u32, f32),       // p<sides>x<diameter>
}

/// Symbol definition with shape and parameters
#[derive(Clone, Debug)]
pub struct Symbol {
    pub id: String,
    pub shape: SymbolShape,
}

/// Parse symbols definition from content
///
/// # Format
/// ```
/// #
/// #Feature symbol names
/// $0 r200
/// $1 s300
/// $2 r100x200
/// ```
pub fn parse_symbols(content: &str) -> Result<std::collections::HashMap<String, Symbol>, JsValue> {
    let mut symbols = std::collections::HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse symbol definition: $<id> <shape_def>
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let symbol_id = parts[0];
        let shape_def = parts[1];

        match parse_symbol_definition(symbol_id, shape_def) {
            Ok(symbol) => {
                symbols.insert(symbol_id.to_string(), symbol);
            }
            Err(_) => {
                // Skip malformed symbols
                continue;
            }
        }
    }

    Ok(symbols)
}

/// Parse individual symbol definition
/// Format: r<diameter>, s<size>, r<width>x<height>, o<width>x<height>, p<sides>x<diameter>
fn parse_symbol_definition(id: &str, def: &str) -> Result<Symbol, JsValue> {
    let shape = if def.starts_with('r') && def.contains('x') {
        // Rectangle: r<width>x<height>
        let dims = &def[1..];
        let parts: Vec<&str> = dims.split('x').collect();
        if parts.len() == 2 {
            let width = parts[0]
                .parse::<f32>()
                .map_err(|_| JsValue::from_str("Invalid rectangle width"))?;
            let height = parts[1]
                .parse::<f32>()
                .map_err(|_| JsValue::from_str("Invalid rectangle height"))?;
            SymbolShape::Rectangle(width, height)
        } else {
            return Err(JsValue::from_str("Invalid rectangle format"));
        }
    } else if def.starts_with('r') {
        // Round: r<diameter>
        let diameter = def[1..]
            .parse::<f32>()
            .map_err(|_| JsValue::from_str("Invalid round diameter"))?;
        SymbolShape::Round(diameter)
    } else if def.starts_with('s') {
        // Square: s<size>
        let size = def[1..]
            .parse::<f32>()
            .map_err(|_| JsValue::from_str("Invalid square size"))?;
        SymbolShape::Square(size)
    } else if def.starts_with('o') && def.contains('x') {
        // Obround: o<width>x<height>
        let dims = &def[1..];
        let parts: Vec<&str> = dims.split('x').collect();
        if parts.len() == 2 {
            let width = parts[0]
                .parse::<f32>()
                .map_err(|_| JsValue::from_str("Invalid obround width"))?;
            let height = parts[1]
                .parse::<f32>()
                .map_err(|_| JsValue::from_str("Invalid obround height"))?;
            SymbolShape::Obround(width, height)
        } else {
            return Err(JsValue::from_str("Invalid obround format"));
        }
    } else if def.starts_with('p') && def.contains('x') {
        // Polygon: p<sides>x<diameter>
        let dims = &def[1..];
        let parts: Vec<&str> = dims.split('x').collect();
        if parts.len() == 2 {
            let sides = parts[0]
                .parse::<u32>()
                .map_err(|_| JsValue::from_str("Invalid polygon sides"))?;
            let diameter = parts[1]
                .parse::<f32>()
                .map_err(|_| JsValue::from_str("Invalid polygon diameter"))?;
            SymbolShape::Polygon(sides, diameter)
        } else {
            return Err(JsValue::from_str("Invalid polygon format"));
        }
    } else {
        return Err(JsValue::from_str("Unknown symbol shape"));
    };

    Ok(Symbol {
        id: id.to_string(),
        shape,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_round_symbol() {
        let result = parse_symbol_definition("$0", "r200");
        assert!(result.is_ok());
        let symbol = result.unwrap();
        assert_eq!(symbol.id, "$0");
        match symbol.shape {
            SymbolShape::Round(d) => assert_eq!(d, 200.0),
            _ => panic!("Expected Round shape"),
        }
    }

    #[test]
    fn test_parse_square_symbol() {
        let result = parse_symbol_definition("$1", "s300");
        assert!(result.is_ok());
        let symbol = result.unwrap();
        match symbol.shape {
            SymbolShape::Square(s) => assert_eq!(s, 300.0),
            _ => panic!("Expected Square shape"),
        }
    }

    #[test]
    fn test_parse_rectangle_symbol() {
        let result = parse_symbol_definition("$2", "r100x200");
        assert!(result.is_ok());
        let symbol = result.unwrap();
        match symbol.shape {
            SymbolShape::Rectangle(w, h) => {
                assert_eq!(w, 100.0);
                assert_eq!(h, 200.0);
            }
            _ => panic!("Expected Rectangle shape"),
        }
    }
}
