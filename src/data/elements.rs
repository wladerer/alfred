use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct AtomEntry {
    color: String,
    radius: f64,
    avg_ionic_radius: f64,
}

/// Lookup table for element visual properties, loaded from atoms.json.
#[derive(Resource)]
pub struct ElementData {
    by_symbol: HashMap<String, ElementProps>,
    by_z: HashMap<u8, ElementProps>,
}

#[derive(Debug, Clone)]
pub struct ElementProps {
    pub color: Color,
    pub radius: f32,
    pub avg_ionic_radius: f32,
}

impl ElementData {
    /// Load from the embedded atoms.json (baked in at compile time).
    pub fn load() -> Self {
        let json = include_str!("../../resources/atoms.json");
        let raw: HashMap<String, AtomEntry> =
            serde_json::from_str(json).expect("Failed to parse atoms.json");

        let mut by_symbol = HashMap::new();
        let mut by_z = HashMap::new();

        for (symbol, entry) in &raw {
            let props = ElementProps {
                color: hex_to_color(&entry.color),
                radius: entry.radius as f32,
                avg_ionic_radius: entry.avg_ionic_radius as f32,
            };
            let z = super::structure::symbol_to_z(symbol);
            by_symbol.insert(symbol.clone(), props.clone());
            if z > 0 {
                by_z.insert(z, props);
            }
        }

        Self { by_symbol, by_z }
    }

    pub fn by_symbol(&self, symbol: &str) -> &ElementProps {
        self.by_symbol.get(symbol).unwrap_or_else(|| {
            self.by_symbol.get("C").expect("fallback element missing")
        })
    }

    pub fn by_z(&self, z: u8) -> &ElementProps {
        self.by_z.get(&z).unwrap_or_else(|| {
            self.by_z.get(&6).expect("fallback element missing")
        })
    }
}

fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128) as f32 / 255.0;
    Color::srgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_element_data() {
        let data = ElementData::load();
        let na = data.by_symbol("Na");
        assert!(na.radius > 1.0);
        let o = data.by_z(8);
        assert!(o.radius > 0.0);
    }

    #[test]
    fn test_hex_to_color() {
        let c = hex_to_color("#FF0000");
        // Should be red
        let srgba = c.to_srgba();
        assert!((srgba.red - 1.0).abs() < 0.01);
        assert!(srgba.green.abs() < 0.01);
    }
}
