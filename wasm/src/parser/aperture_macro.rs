use super::geometry::{line_to_triangles, rotate_point, triangulate_outline, Primitive};
use std::collections::HashMap;
use std::mem::take;

/// Aperture macro definition - kept as statements until parameters arrive
#[derive(Clone, Debug)]
pub struct ApertureMacro {
    pub statements: Vec<String>, // Keep as String for dynamic calculation
    pub has_negative: bool,      // true if any primitive has exposure 0
}

impl ApertureMacro {
    pub fn new() -> Self {
        ApertureMacro {
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
}

/// Parse Aperture macro - %AMname*statements%
pub fn parse_macro(data: &str, macros: &mut HashMap<String, ApertureMacro>) {
    // Format: %AMname*statement1*statement2*...*%
    // Remove %AM and %
    let content = data.trim_start_matches("%AM").trim_end_matches('%');

    // Split name and statements
    let parts: Vec<&str> = content.split('*').collect();
    if parts.is_empty() {
        return;
    }

    let name = parts[0].to_string();
    let mut macro_def = ApertureMacro::new();

    // Parse statements (parts[1] to parts[n-1], last part might be empty)
    for part in &parts[1..] {
        let stmt = part.trim();
        if stmt.is_empty() {
            continue;
        }

        // Store statement as-is (will be evaluated when instantiated with parameters)
        macro_def.statements.push(stmt.to_string());
    }

    // Calculate has_negative by checking if any statement has exposure=0
    macro_def.has_negative = check_macro_has_negative(&macro_def.statements);

    macros.insert(name, macro_def);
}

/// Check if macro statements contain any primitive with exposure=0
pub fn check_macro_has_negative(statements: &[String]) -> bool {
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

/// Evaluate expression - $5-$3, $1/2, 2X$3, etc.
/// X is interpreted as multiply, $variables are evaluated in real-time
pub fn evaluate_expression(expr: &str, variables: &HashMap<String, f32>) -> Result<f32, String> {
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
                tokens.push(take(&mut current_token));
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
                    tokens.push(take(&mut current_token)); // Save "-"

                    // Process $variable
                    current_token.push(chars.next().unwrap()); // $
                    while let Some(&digit_ch) = chars.peek() {
                        if digit_ch.is_ascii_digit() {
                            current_token.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(take(&mut current_token));
                } else if next_ch.is_ascii_digit() || next_ch == '.' {
                    // Read number after sign
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_ascii_digit() || next_ch == '.' {
                            current_token.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(take(&mut current_token));
                } else {
                    // If only sign without following value, treat as operator
                    if !current_token.is_empty() {
                        tokens.push(take(&mut current_token));
                    }
                    tokens.push(ch.to_string());
                }
            }
        }
        // Process $variable: $1, $2, $5, etc.
        else if ch == '$' {
            if !current_token.is_empty() {
                tokens.push(take(&mut current_token));
            }

            current_token.push(ch);
            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_ascii_digit() {
                    current_token.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            tokens.push(take(&mut current_token));
        }
        // Regular number
        else if ch.is_ascii_digit() || ch == '.' {
            current_token.push(ch);
        }
        // Operator
        else if "+-*/()".contains(ch) {
            if !current_token.is_empty() {
                tokens.push(take(&mut current_token));
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
        if i + 2 < tokens.len() && ("*" == tokens[i + 1] || "/" == tokens[i + 1]) {
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

/// Parse primitive statement: 1,1,$7,$5-$3,$6-$3,$4*
pub fn parse_primitive_statement(
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
                hole_x: center_x,
                hole_y: center_y,
                hole_radius: 0.0, // Macros don't support holes in circle primitives
            });

            Some(1)
        }
        4 => {
            // Outline: 4,exposure,vertices,x1,y1,x2,y2,...,xn,yn[,rotation]
            if parts.len() < 4 {
                return None;
            }
            let exposure: f32 = evaluate_expression(parts[1], variables).ok()?;
            let num_vertices: u32 =
                evaluate_expression(parts[2], variables).ok()? as u32;
            let rotation: f32 = if parts.len() > 3 + (num_vertices as usize) * 2 {
                evaluate_expression(
                    parts[3 + (num_vertices as usize) * 2],
                    variables,
                )
                .ok()?
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
            let num_vertices: u32 =
                evaluate_expression(parts[2], variables).ok()? as u32;
            let center_x: f32 = evaluate_expression(parts[3], variables).ok()?;
            let center_y: f32 = evaluate_expression(parts[4], variables).ok()?;
            let diameter: f32 = evaluate_expression(parts[5], variables).ok()?;
            let rotation: f32 = if parts.len() > 6 {
                evaluate_expression(parts[6], variables).ok()?
                    * (std::f32::consts::PI / 180.0)
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
                    vertices: [[center_x, center_y], vertices[i], vertices[next_i]],
                    exposure,
                    hole_x: 0.0,
                    hole_y: 0.0,
                    hole_radius: 0.0, // Macros don't support holes in outline primitives
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
            let outer_diameter: f32 =
                evaluate_expression(parts[3], variables).ok()?;
            let inner_diameter: f32 =
                evaluate_expression(parts[4], variables).ok()?;
            let gap_thickness: f32 =
                evaluate_expression(parts[5], variables).ok()?;
            let rotation: f32 = if parts.len() > 6 {
                evaluate_expression(parts[6], variables).ok()?
                    * (std::f32::consts::PI / 180.0)
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
                evaluate_expression(parts[7], variables).ok()?
                    * (std::f32::consts::PI / 180.0)
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
                vertices: [v1, v2, v3],
                exposure,
                hole_x: 0.0,
                hole_y: 0.0,
                hole_radius: 0.0,
            };
            let mut tri2 = Primitive::Triangle {
                vertices: [v1, v3, v4],
                exposure,
                hole_x: 0.0,
                hole_y: 0.0,
                hole_radius: 0.0,
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
