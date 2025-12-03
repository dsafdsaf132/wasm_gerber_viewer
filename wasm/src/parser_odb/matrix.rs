use wasm_bindgen::prelude::*;

/// Layer information from matrix file
#[derive(Clone, Debug)]
pub struct LayerInfo {
    pub name: String,
    pub layer_type: String,
    pub polarity: String,
}

/// Parse matrix file to extract layer information
///
/// # Format example
/// ```
/// (matrix (created "2023-01-01T00:00:00")
///  (version 1.0)
///  (units MM)
///  (step
///   (name "default")
///   (layer
///    (name "GERBER")
///    (type "CONDUCTOR")
///    (polarity "POSITIVE")
///    (features "steps/default/layers/GERBER/features")
///   )
///  )
/// )
/// ```
pub fn parse_matrix(content: &str) -> Result<Vec<LayerInfo>, JsValue> {
    let mut layers = Vec::new();

    // Simple parser for matrix format - look for layer blocks
    let content = content.replace('\n', " ").replace('\t', " ");

    // Find all layer definitions
    let mut in_layer = false;
    let mut current_layer = LayerInfo {
        name: String::new(),
        layer_type: String::new(),
        polarity: String::new(),
    };

    let mut depth = 0;
    let mut i = 0;
    let chars: Vec<char> = content.chars().collect();

    while i < chars.len() {
        if chars[i] == '(' {
            depth += 1;

            // Check if this is a layer block
            let rest = &content[i..];
            if rest.starts_with("(layer") {
                in_layer = true;
                current_layer = LayerInfo {
                    name: String::new(),
                    layer_type: String::new(),
                    polarity: String::new(),
                };
            }
        } else if chars[i] == ')' {
            depth -= 1;

            // If we're closing a layer block
            if in_layer && depth == 2 {
                if !current_layer.name.is_empty() {
                    layers.push(current_layer.clone());
                }
                in_layer = false;
            }
        }

        // Parse fields within layer block
        if in_layer {
            let rest = &content[i..];
            if let Some(name_value) = extract_field(rest, "(name") {
                current_layer.name = name_value;
            } else if let Some(type_value) = extract_field(rest, "(type") {
                current_layer.layer_type = type_value;
            } else if let Some(polarity_value) = extract_field(rest, "(polarity") {
                current_layer.polarity = polarity_value;
            }
        }

        i += 1;
    }

    Ok(layers)
}

/// Extract field value from s-expression format
/// Example: (name "GERBER") -> "GERBER"
fn extract_field(content: &str, field_name: &str) -> Option<String> {
    if let Some(pos) = content.find(field_name) {
        let rest = &content[pos + field_name.len()..];
        let rest = rest.trim_start();

        // Find quoted string
        if let Some(quote_start) = rest.find('"') {
            if let Some(quote_end) = rest[quote_start + 1..].find('"') {
                let value = &rest[quote_start + 1..quote_start + 1 + quote_end];
                return Some(value.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_matrix() {
        let content = r#"
(matrix (created "2023-01-01")
 (version 1.0)
 (step
  (name "default")
  (layer
   (name "TOP")
   (type "CONDUCTOR")
   (polarity "POSITIVE")
  )
 )
)
        "#;

        let result = parse_matrix(content);
        assert!(result.is_ok());
        let layers = result.unwrap();
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].name, "TOP");
        assert_eq!(layers[0].layer_type, "CONDUCTOR");
        assert_eq!(layers[0].polarity, "POSITIVE");
    }
}
