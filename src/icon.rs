//! Tray icon generation
//!
//! Creates a shield icon for the ad blocker

/// Generate a shield icon with block symbol
/// Returns RGBA pixel data
fn generate_shield_icon(size: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let center_x = size as f32 / 2.0;
    let center_y = size as f32 / 2.0;

    // Shield colors - KakaoTalk style
    let shield_color = [0xFA, 0xE1, 0x00, 0xFF]; // KakaoTalk Yellow #FAE100
    let block_color = [0x3C, 0x1E, 0x1E, 0xFF]; // Dark brown (KakaoTalk text color)
    let outline_color = [0xD4, 0xBE, 0x00, 0xFF]; // Darker yellow

    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let fx = x as f32;
            let fy = y as f32;

            // Shield shape: rounded top, pointed bottom
            let in_shield = is_in_shield(fx, fy, center_x, center_y, size as f32);
            let on_outline = is_on_shield_outline(fx, fy, center_x, center_y, size as f32);

            if on_outline {
                rgba[idx..idx + 4].copy_from_slice(&outline_color);
            } else if in_shield {
                // Check if in the "block" symbol (diagonal line)
                let in_block = is_in_block_symbol(fx, fy, center_x, center_y, size as f32);

                if in_block {
                    rgba[idx..idx + 4].copy_from_slice(&block_color);
                } else {
                    rgba[idx..idx + 4].copy_from_slice(&shield_color);
                }
            }
            // else: transparent (already 0)
        }
    }

    rgba
}

/// Check if point is inside shield shape
fn is_in_shield(x: f32, y: f32, cx: f32, cy: f32, size: f32) -> bool {
    let margin = size * 0.1;
    let top = margin;
    let bottom = size - margin;
    let left = margin + size * 0.05;
    let right = size - margin - size * 0.05;

    // Upper part: rounded rectangle
    let upper_bottom = cy + size * 0.1;

    if y >= top && y <= upper_bottom {
        // Check horizontal bounds with rounded corners
        let corner_radius = size * 0.15;
        let in_x = x >= left && x <= right;

        if in_x {
            // Round the top corners
            if y < top + corner_radius {
                if x < left + corner_radius {
                    let dx = x - (left + corner_radius);
                    let dy = y - (top + corner_radius);
                    return dx * dx + dy * dy <= corner_radius * corner_radius;
                } else if x > right - corner_radius {
                    let dx = x - (right - corner_radius);
                    let dy = y - (top + corner_radius);
                    return dx * dx + dy * dy <= corner_radius * corner_radius;
                }
            }
            return true;
        }
        return false;
    }

    // Lower part: triangle pointing down
    if y > upper_bottom && y <= bottom {
        let progress = (y - upper_bottom) / (bottom - upper_bottom);
        let half_width = (right - left) / 2.0 * (1.0 - progress);
        return x >= cx - half_width && x <= cx + half_width;
    }

    false
}

/// Check if point is on shield outline
fn is_on_shield_outline(x: f32, y: f32, cx: f32, cy: f32, size: f32) -> bool {
    let thickness = size * 0.06;

    is_in_shield(x, y, cx, cy, size) && !is_in_shield_inner(x, y, cx, cy, size, thickness)
}

/// Check if point is inside inner shield (for outline calculation)
fn is_in_shield_inner(x: f32, y: f32, cx: f32, cy: f32, size: f32, shrink: f32) -> bool {
    let margin = size * 0.1 + shrink;
    let top = margin;
    let bottom = size - margin;
    let left = margin + size * 0.05;
    let right = size - margin - size * 0.05;

    let upper_bottom = cy + size * 0.1;

    if y >= top && y <= upper_bottom {
        let corner_radius = (size * 0.15 - shrink).max(0.0);
        let in_x = x >= left && x <= right;

        if in_x {
            if y < top + corner_radius && corner_radius > 0.0 {
                if x < left + corner_radius {
                    let dx = x - (left + corner_radius);
                    let dy = y - (top + corner_radius);
                    return dx * dx + dy * dy <= corner_radius * corner_radius;
                } else if x > right - corner_radius {
                    let dx = x - (right - corner_radius);
                    let dy = y - (top + corner_radius);
                    return dx * dx + dy * dy <= corner_radius * corner_radius;
                }
            }
            return true;
        }
        return false;
    }

    if y > upper_bottom && y <= bottom {
        let progress = (y - upper_bottom) / (bottom - upper_bottom);
        let half_width = ((right - left) / 2.0 - shrink).max(0.0) * (1.0 - progress);
        return x >= cx - half_width && x <= cx + half_width;
    }

    false
}

/// Check if point is in the block symbol (X or diagonal line)
fn is_in_block_symbol(x: f32, y: f32, cx: f32, cy: f32, size: f32) -> bool {
    let symbol_size = size * 0.25;
    let thickness = size * 0.08;

    // Offset the symbol center slightly up
    let sy = cy - size * 0.05;

    // Distance from center
    let dx = (x - cx).abs();
    let dy = (y - sy).abs();

    // Check if within symbol bounds
    if dx > symbol_size || dy > symbol_size {
        return false;
    }

    // X shape: two diagonal lines
    // Line 1: top-left to bottom-right
    let dist1 = ((x - cx) - (y - sy)).abs() / 1.414;
    // Line 2: top-right to bottom-left
    let dist2 = ((x - cx) + (y - sy)).abs() / 1.414;

    dist1 < thickness || dist2 < thickness
}

/// Load and create the tray icon
pub fn load_icon() -> tray_icon::Icon {
    let size = 32u32;
    let rgba = generate_shield_icon(size);

    tray_icon::Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}
