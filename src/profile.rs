//! What the material of the stencil asks for. The numbers are starting points taken from common practice, to be tried on a scrap of the material.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Mylar,
    Kapton,
    Steel,
}

impl Material {
    pub fn parse(text: &str) -> Result<Material, String> {
        match text.trim().to_ascii_lowercase().as_str() {
            "mylar" | "polyester" => Ok(Material::Mylar),
            "kapton" | "polyimide" => Ok(Material::Kapton),
            "steel" | "stainless" => Ok(Material::Steel),
            other => Err(format!(
                "'{other}' is not a material (mylar, kapton or steel)"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Profile {
    /// The thickness of the sheet.
    pub thickness_mm: f64,
    /// How much the openings are made smaller than the pads, in percent of each side.
    pub shrink_percent: f64,
    /// The radius of the corners of the openings.
    pub corner_radius_mm: f64,
    /// The width of what the laser removes: the cut line is moved inwards by half of it so that the opening comes out the size that was asked.
    pub kerf_mm: f64,
}

impl Profile {
    pub fn of(m: Material) -> Profile {
        match m {
            Material::Mylar => Profile {
                thickness_mm: 0.12,
                shrink_percent: 10.0,
                corner_radius_mm: 0.05,
                kerf_mm: 0.10,
            },
            Material::Kapton => Profile {
                thickness_mm: 0.075,
                shrink_percent: 8.0,
                corner_radius_mm: 0.04,
                kerf_mm: 0.08,
            },
            Material::Steel => Profile {
                thickness_mm: 0.12,
                shrink_percent: 5.0,
                corner_radius_mm: 0.05,
                kerf_mm: 0.04,
            },
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("thickness_mm", self.thickness_mm, 0.02, 1.0)?;
        range("shrink_percent", self.shrink_percent, 0.0, 50.0)?;
        range("corner_radius_mm", self.corner_radius_mm, 0.0, 2.0)?;
        range("kerf_mm", self.kerf_mm, 0.0, 1.0)
    }
}
