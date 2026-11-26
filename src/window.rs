//! The state of the window, without any screen: the paste layer that is open, the material and the parameters being edited, the openings and the paths worked out from them, what was
//! last said to the person and the zoom of the picture. The screen (`gui`, built with the `gui` feature) only draws this and passes on what the person does, so everything here is
//! tested as plain code.

use crate::crib::{sheet_from_file, sheets_from_file, SheetSpec};
use crate::eh3dfile;
use crate::lang::{fill, Lang, Msg};
use crate::layer::{read_layer, read_layer_file};
use crate::openings::{
    from_gerber_with, mirrored, ratios, shrunk, weak_with, Limits, Opening, Ratios,
};
use crate::output::{gcode, hpgl, paths, write_atomic, Path as CutPath};
use crate::profile::{Material, Profile};
use crate::viewport::Viewport;
use eh3d_gerberviewer::eh3dfile::{self as suite, JobFile};
use eh3d_parser_gcode::emit::{header, sha256_hex, Header};
use std::fs;
use std::path::{Path, PathBuf};

/// The extensions of the files the window offers to open as a Gerber layer.
pub const LAYER_EXTENSIONS: [&str; 15] = [
    "gtp", "gbp", "gbr", "ger", "pho", "art", "gtl", "gbl", "gts", "gbs", "gto", "gbo", "gko",
    "gm1", "eh3d",
];
/// The extensions of a table of sheets.
pub const TABLE_EXTENSIONS: [&str; 1] = ["csv"];

/// The materials of the stencil, in the order the window offers them.
pub const MATERIALS: [Material; 3] = [Material::Mylar, Material::Kapton, Material::Steel];

/// The sentence that names a material.
pub fn material_msg(m: Material) -> Msg {
    match m {
        Material::Mylar => Msg::MaterialMylar,
        Material::Kapton => Msg::MaterialKapton,
        Material::Steel => Msg::MaterialSteel,
    }
}

/// A parameter of the window: the sentence that names it and the text being typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: &'static str,
    pub label: Msg,
    pub text: String,
}

/// The parameters the window offers, in the order they are shown: the four of the profile of the material, then the feed and the power of the laser.
pub fn default_fields(material: Material) -> Vec<Field> {
    let p = Profile::of(material);
    let f = |key: &'static str, label: Msg, text: String| Field { key, label, text };
    vec![
        f("thickness_mm", Msg::Thickness, p.thickness_mm.to_string()),
        f("shrink_percent", Msg::Shrink, p.shrink_percent.to_string()),
        f(
            "corner_radius_mm",
            Msg::CornerRadius,
            p.corner_radius_mm.to_string(),
        ),
        f("kerf_mm", Msg::Kerf, p.kerf_mm.to_string()),
        f("feed", Msg::Feed, "600".to_string()),
        f("smax", Msg::LaserPower, "1000".to_string()),
    ]
}

/// What the window last said to the person.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ready,
    Computed {
        openings: usize,
        weak: usize,
    },
    Exported {
        file: String,
    },
    NoLayer,
    Failed(String),
    Incomplete(usize),
    Weak(usize),
    BadValue {
        label: Msg,
        why: String,
    },
    SameFile,
    /// A paste layer of the job of a project was opened.
    ProjectOpened {
        name: String,
        layers: usize,
    },
    /// The stencil was kept in the branch of the program.
    ProjectSaved {
        file: String,
    },
    /// The stencils of the project were made from an older job.
    ProjectStale {
        name: String,
    },
    NoProject,
    SheetsRead {
        count: usize,
        file: String,
    },
}

impl Status {
    /// The sentence, in the language of the person.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Status::Ready => Msg::Ready.tr(lang).to_string(),
            Status::Computed { openings, weak } => {
                let (openings, weak) = (openings.to_string(), weak.to_string());
                fill(Msg::Computed.tr(lang), &[openings.as_str(), weak.as_str()])
            }
            Status::Exported { file } => fill(Msg::Exported.tr(lang), &[file.as_str()]),
            Status::NoLayer => Msg::NoLayer.tr(lang).to_string(),
            Status::Failed(why) => fill(Msg::Failed.tr(lang), &[why.as_str()]),
            Status::Incomplete(n) => fill(Msg::Incomplete.tr(lang), &[n.to_string().as_str()]),
            Status::Weak(n) => fill(Msg::Weak.tr(lang), &[n.to_string().as_str()]),
            Status::BadValue { label, why } => {
                fill(Msg::BadValue.tr(lang), &[label.tr(lang), why.as_str()])
            }
            Status::SameFile => Msg::SameFile.tr(lang).to_string(),
            Status::ProjectOpened { name, layers } => fill(
                Msg::ProjectOpened.tr(lang),
                &[name.as_str(), layers.to_string().as_str()],
            ),
            Status::ProjectSaved { file } => fill(Msg::ProjectSaved.tr(lang), &[file.as_str()]),
            Status::ProjectStale { name } => fill(Msg::ProjectStale.tr(lang), &[name.as_str()]),
            Status::NoProject => Msg::NoProject.tr(lang).to_string(),
            Status::SheetsRead { count, file } => fill(
                Msg::SheetsRead.tr(lang),
                &[count.to_string().as_str(), file.as_str()],
            ),
        }
    }

    /// Whether the sentence is a problem or a warning (the window shows it in another colour).
    pub fn is_problem(&self) -> bool {
        !matches!(
            self,
            Status::Ready
                | Status::Computed { .. }
                | Status::Exported { .. }
                | Status::ProjectOpened { .. }
                | Status::ProjectSaved { .. }
                | Status::SheetsRead { .. }
        )
    }
}

/// The paste layer that is open.
#[derive(Debug, Clone)]
pub struct Layer {
    pub path: PathBuf,
    pub name: String,
    /// The bytes of the file, kept for the hash that the G-code names its source with.
    pub bytes: Vec<u8>,
    pub openings: Vec<Opening>,
    /// What could not become an opening.
    pub notes: Vec<String>,
    /// The layer of the job of a project this one is.
    pub job: Option<JobFile>,
}

/// The window.
pub struct Window {
    pub lang: Lang,
    pub material: Material,
    pub fields: Vec<Field>,
    /// The stencil of the bottom side: the layer is seen from the other face.
    pub mirror: bool,
    /// Write although an opening will not release the paste or part of the layer could not become an opening.
    pub force: bool,
    pub layer: Option<Layer>,
    /// The sheet of the crib that was chosen: its thickness, reduction, kerf and ratio limits replace those of the material.
    pub sheet: Option<SheetSpec>,
    /// The pads as drawn (mirrored when asked), before the shrink.
    pub openings: Vec<Opening>,
    /// The closed paths to cut, one for each opening, in the same order.
    pub cuts: Vec<CutPath>,
    /// The places in `openings` of those that will not release the paste.
    pub weak: Vec<usize>,
    /// The ratios of each opening as the paste sees it (shrunk), in the same order as `openings`.
    pub ratios: Vec<Ratios>,
    pub status: Status,
    pub viewport: Viewport,
    /// The project whose job is open.
    pub project: Option<PathBuf>,
    /// The paste layers of that job.
    pub job_layers: Vec<JobFile>,
    /// The sheets of the table of sheets that was read, to choose one.
    pub sheets: Vec<SheetSpec>,
}

impl Window {
    pub fn new(lang: Lang) -> Window {
        let material = Material::Mylar;
        Window {
            lang,
            material,
            fields: default_fields(material),
            mirror: false,
            force: false,
            layer: None,
            sheet: None,
            openings: Vec::new(),
            cuts: Vec::new(),
            weak: Vec::new(),
            ratios: Vec::new(),
            status: Status::Ready,
            viewport: Viewport::default(),
            project: None,
            job_layers: Vec::new(),
            sheets: Vec::new(),
        }
    }

    /// The profile, the feed and the power of the laser from the values typed in the parameters. A value that is not valid says which parameter it is.
    pub fn params(&self) -> Result<(Profile, f64, u32), Status> {
        let mut p = Profile::of(self.material);
        let (mut feed, mut smax) = (600.0, 1000.0);
        for f in &self.fields {
            let v = f
                .text
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| Status::BadValue {
                    label: f.label,
                    why: format!("'{}' is not a number", f.text.trim()),
                })?;
            match f.key {
                "thickness_mm" => p.thickness_mm = v,
                "shrink_percent" => p.shrink_percent = v,
                "corner_radius_mm" => p.corner_radius_mm = v,
                "kerf_mm" => p.kerf_mm = v,
                "feed" => feed = v,
                _ => smax = v,
            }
        }
        if let Some(s) = &self.sheet {
            p.reduction_mm = s.reduction_mm;
        }
        p.validate().map_err(Status::Failed)?;
        if !(1.0..=100_000.0).contains(&smax) {
            return Err(Status::BadValue {
                label: Msg::LaserPower,
                why: format!("{smax} must be from 1 to 100000"),
            });
        }
        Ok((p, feed, smax as u32))
    }

    /// Changes the text of a parameter and works the stencil out again.
    pub fn set_field(&mut self, index: usize, text: String) {
        if let Some(f) = self.fields.get_mut(index) {
            if f.text != text {
                f.text = text;
                self.update();
            }
        }
    }

    /// Chooses a material: its thickness, shrink, corner radius and kerf replace the four values typed (the feed and the power stay).
    pub fn set_material(&mut self, material: Material) {
        self.material = material;
        let fresh = default_fields(material);
        for (f, n) in self.fields.iter_mut().zip(fresh).take(4) {
            f.text = n.text;
        }
        // a sheet that was chosen keeps its figures over those of the material
        if let Some(s) = self.sheet.clone() {
            self.put_sheet_in_fields(&s);
        }
        self.update();
    }

    /// The ratio limits in force: those of the sheet when one was chosen (a limit of 0 leaves the default), the defaults otherwise.
    pub fn limits(&self) -> Limits {
        match &self.sheet {
            Some(s) => s.limits(Limits::default()).unwrap_or_default(),
            None => Limits::default(),
        }
    }

    fn put_sheet_in_fields(&mut self, s: &SheetSpec) {
        for f in &mut self.fields {
            match f.key {
                "thickness_mm" => f.text = s.thickness_mm.to_string(),
                "shrink_percent" => f.text = "0".to_string(),
                // a sheet that takes its kerf from a laser of the crib leaves the kerf of the material
                "kerf_mm" if s.kerf_mm > 0.0 => f.text = s.kerf_mm.to_string(),
                _ => {}
            }
        }
    }

    /// Chooses a sheet of the crib (or none): its thickness, kerf and reduction replace the values of the material, and the limits of its ratios replace the defaults. A sheet whose
    /// limits this program cannot take is refused and the old one stays.
    pub fn set_sheet(&mut self, sheet: Option<SheetSpec>) -> bool {
        if let Some(s) = &sheet {
            if let Err(why) = s.limits(Limits::default()) {
                self.status = Status::Failed(why);
                return false;
            }
        }
        match sheet {
            Some(s) => {
                self.put_sheet_in_fields(&s);
                self.sheet = Some(s);
            }
            None => {
                self.sheet = None;
                let fresh = default_fields(self.material);
                for (f, n) in self.fields.iter_mut().zip(fresh).take(4) {
                    f.text = n.text;
                }
            }
        }
        self.update();
        true
    }

    /// Reads the table of sheets in `file` (with the limits of the crib reader) and chooses the sheet called `name`; the status says what went wrong.
    pub fn load_sheet(&mut self, file: &str, name: &str) -> bool {
        match sheet_from_file(file, name) {
            Ok(s) => self.set_sheet(Some(s)),
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// The sentence that says which sheet is chosen and its figures, in the language of the person; `None` when no sheet is chosen.
    pub fn sheet_text(&self) -> Option<String> {
        let s = self.sheet.as_ref()?;
        let kerf = self
            .fields
            .iter()
            .find(|f| f.key == "kerf_mm")
            .map(|f| f.text.trim().to_string())
            .unwrap_or_default();
        Some(fill(
            Msg::SheetLine.tr(self.lang),
            &[
                s.name.as_str(),
                s.thickness_mm.to_string().as_str(),
                s.reduction_mm.to_string().as_str(),
                kerf.as_str(),
            ],
        ))
    }

    /// Mirrors the stencil (the bottom side) or not.
    pub fn set_mirror(&mut self, mirror: bool) {
        self.mirror = mirror;
        self.update();
    }

    /// Switches "write anyway" on or off.
    pub fn set_force(&mut self, force: bool) {
        self.force = force;
        self.update();
    }

    /// The box of the picture: every pad.
    fn extent(&self) -> Option<((f64, f64), (f64, f64))> {
        let mut lo = (f64::INFINITY, f64::INFINITY);
        let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for o in &self.openings {
            lo = (lo.0.min(o.x - o.w / 2.0), lo.1.min(o.y - o.h / 2.0));
            hi = (hi.0.max(o.x + o.w / 2.0), hi.1.max(o.y + o.h / 2.0));
        }
        (lo.0 <= hi.0 && lo.1 <= hi.1).then_some((lo, hi))
    }

    /// Shows the whole stencil in a picture of this size.
    pub fn fit(&mut self, width_px: f64, height_px: f64) {
        if let Some((lo, hi)) = self.extent() {
            self.viewport = Viewport::fit(lo, hi, width_px, height_px, 24.0);
        }
    }

    /// Reads a table of sheets so that one can be chosen; the sheet that was chosen stays until another one is.
    pub fn load_sheet_table(&mut self, path: &Path) -> bool {
        match sheets_from_file(&path.to_string_lossy()) {
            Ok(sheets) => {
                self.status = Status::SheetsRead {
                    count: sheets.len(),
                    file: name_of(path),
                };
                self.sheets = sheets;
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Chooses the sheet of the table at `index`, or the material alone with `None`.
    pub fn choose_sheet(&mut self, index: Option<usize>) -> bool {
        match index {
            None => self.set_sheet(None),
            Some(i) => match self.sheets.get(i).cloned() {
                Some(s) => self.set_sheet(Some(s)),
                None => false,
            },
        }
    }

    /// Opens the first paste layer of the job of a project, and says when the stencils kept in it are out of date. The old layer stays open when the job cannot be used.
    pub fn open_project(&mut self, path: &Path) -> bool {
        let read = eh3dfile::paste_layers(path)
            .and_then(|layers| Ok((layers, suite::state(&eh3dfile::KIT, path)?)));
        match read {
            Ok((layers, state)) => {
                let before = (self.project.take(), std::mem::take(&mut self.job_layers));
                self.project = Some(path.to_path_buf());
                self.job_layers = layers;
                if !self.choose_job_layer(0) {
                    (self.project, self.job_layers) = before;
                    return false;
                }
                let name = name_of(path);
                if state.out_of_date {
                    self.status = Status::ProjectStale { name };
                } else if matches!(self.status, Status::Computed { .. }) {
                    self.status = Status::ProjectOpened {
                        name,
                        layers: self.job_layers.len(),
                    };
                }
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Works with the paste layer of the job at `index` (the top and the bottom side are two layers).
    pub fn choose_job_layer(&mut self, index: usize) -> bool {
        let (Some(project), Some(file)) = (&self.project, self.job_layers.get(index)) else {
            return false;
        };
        // the stencil is suggested next to the project, with the name of the layer
        let beside = project.with_file_name(file.file_name());
        match paste_from_bytes(&beside, file.file_name(), file.bytes.clone()) {
            Ok(mut layer) => {
                layer.job = Some(file.clone());
                self.layer = Some(layer);
                self.update();
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Makes the G-code of the layer of the job and keeps it in the branch `stencilcam/` of the project. What stops an export stops this too.
    pub fn save_project(&mut self) -> bool {
        let (Some(project), Some(job)) = (
            self.project.clone(),
            self.layer.as_ref().and_then(|l| l.job.clone()),
        ) else {
            self.status = Status::NoProject;
            return false;
        };
        let name = eh3dfile::result_name(&job, "nc");
        let dir = match eh3dfile::scratch_dir() {
            Ok(dir) => dir,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        let out = dir.join(&name);
        let made = self.export(&out);
        let bytes = if made { fs::read(&out).ok() } else { None };
        let _ = fs::remove_dir_all(&dir);
        let Some(bytes) = bytes else {
            return false;
        };
        match eh3dfile::save_stencil(&project, &job, &name, bytes) {
            Ok(_) => {
                self.status = Status::ProjectSaved { file: name };
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Opens a paste layer and works the stencil out. The old layer stays open when the new one cannot be read. A `.eh3d` project opens the paste layer of its job and a `.csv` file
    /// is read as a table of sheets.
    pub fn open(&mut self, path: &Path) -> bool {
        if suite::is_project(path) {
            return self.open_project(path);
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
        {
            return self.load_sheet_table(path);
        }
        match read_paste_layer(path, self.lang) {
            Ok(layer) => {
                self.project = None;
                self.job_layers.clear();
                self.layer = Some(layer);
                self.update();
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// What stops the stencil being written: parts of the layer that are not openings, or openings that will not release the paste. The box "write anyway" lifts it.
    fn blocker(&self) -> Option<Status> {
        if self.force {
            return None;
        }
        let incomplete = self.layer.as_ref().map(|l| l.notes.len())?;
        if incomplete > 0 {
            return Some(Status::Incomplete(incomplete));
        }
        (!self.weak.is_empty()).then_some(Status::Weak(self.weak.len()))
    }

    /// Works the openings, the cutting paths and the checks out with the parameters. Returns whether the paths could be made.
    pub fn update(&mut self) -> bool {
        if self.layer.is_none() {
            self.status = Status::NoLayer;
            return false;
        }
        let (p, _, _) = match self.params() {
            Ok(v) => v,
            Err(st) => {
                self.status = st;
                return false;
            }
        };
        let raw = self
            .layer
            .as_ref()
            .map(|l| l.openings.clone())
            .unwrap_or_default();
        self.openings = if self.mirror { mirrored(&raw) } else { raw };
        self.cuts.clear();
        self.weak.clear();
        self.ratios.clear();
        let limits = self.limits();
        let shown: Result<Vec<Opening>, String> =
            self.openings.iter().map(|o| shrunk(o, &p)).collect();
        let found = shown.and_then(|s| {
            let all = s
                .iter()
                .map(|o| ratios(o, p.thickness_mm))
                .collect::<Result<Vec<_>, _>>()?;
            weak_with(&s, p.thickness_mm, &limits).map(|w| (w, all))
        });
        match found {
            Ok((list, all)) => {
                self.weak = list.into_iter().map(|(i, _)| i).collect();
                self.ratios = all;
            }
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        }
        match paths(&self.openings, &p) {
            Ok(cuts) => self.cuts = cuts,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        }
        self.status = self.blocker().unwrap_or(Status::Computed {
            openings: self.openings.len(),
            weak: self.weak.len(),
        });
        true
    }

    /// The smallest aspect ratio and the smallest area ratio of the openings as the paste sees them; `None` when there are none.
    pub fn smallest_ratios(&self) -> Option<(f64, f64)> {
        let aspect = self.ratios.iter().map(|r| r.aspect).reduce(f64::min)?;
        let area = self.ratios.iter().map(|r| r.area).reduce(f64::min)?;
        Some((aspect, area))
    }

    /// The `count` openings with the lowest aspect ratio (then the lowest area ratio), by their places in `openings`, worst first.
    pub fn weakest(&self, count: usize) -> Vec<(usize, Ratios)> {
        let mut all: Vec<(usize, Ratios)> = self.ratios.iter().copied().enumerate().collect();
        all.sort_by(|a, b| {
            a.1.aspect
                .total_cmp(&b.1.aspect)
                .then(a.1.area.total_cmp(&b.1.area))
                .then(a.0.cmp(&b.0))
        });
        all.truncate(count);
        all
    }

    /// The sentences the window shows for the ratios, in the language chosen: the smallest ones, then the `count` weakest openings by their numbers (from 1). Empty when there are no openings.
    pub fn ratio_texts(&self, count: usize) -> Vec<String> {
        let Some((aspect, area)) = self.smallest_ratios() else {
            return Vec::new();
        };
        let mut out = vec![fill(
            Msg::RatiosSmallest.tr(self.lang),
            &[
                format!("{aspect:.2}").as_str(),
                format!("{area:.2}").as_str(),
            ],
        )];
        for (i, r) in self.weakest(count) {
            out.push(fill(
                Msg::RatiosLine.tr(self.lang),
                &[
                    (i + 1).to_string().as_str(),
                    format!("{:.2}", r.aspect).as_str(),
                    format!("{:.2}", r.area).as_str(),
                ],
            ));
        }
        out
    }

    /// The name suggested for the file of the stencil: the name of the layer with `.nc`, next to it.
    pub fn suggested_output(&self) -> Option<PathBuf> {
        self.layer.as_ref().map(|l| l.path.with_extension("nc"))
    }

    /// Writes the stencil: HP-GL for a name that ends in `.plt` (whatever its case), laser G-code for any other.
    pub fn export(&mut self, out: &Path) -> bool {
        let Some(layer_path) = self.layer.as_ref().map(|l| l.path.clone()) else {
            self.status = Status::NoLayer;
            return false;
        };
        if !self.update() {
            return false;
        }
        let same = match (fs::canonicalize(&layer_path), fs::canonicalize(out)) {
            (Ok(a), Ok(b)) => a == b,
            _ => layer_path == out,
        };
        if same {
            self.status = Status::SameFile;
            return false;
        }
        if let Some(block) = self.blocker() {
            self.status = block;
            return false;
        }
        let (_, feed, smax) = match self.params() {
            Ok(v) => v,
            Err(st) => {
                self.status = st;
                return false;
            }
        };
        let (name, hash) = self
            .layer
            .as_ref()
            .map(|l| (l.name.clone(), sha256_hex(&l.bytes)))
            .unwrap_or_default();
        let is_plot = out
            .extension()
            .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("plt"));
        let text = if is_plot {
            Ok(hpgl(&self.cuts))
        } else {
            let head = header(&Header {
                generator: "eh3d-stencilcam",
                version: env!("CARGO_PKG_VERSION"),
                source: Some((name.as_str(), hash)),
                tools: "laser",
            });
            gcode(&self.cuts, feed, smax).map(|g| format!("{head}{g}"))
        };
        let text = match text {
            Ok(t) => t,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        match write_atomic(out, &text) {
            Ok(()) => {
                self.status = Status::Exported {
                    file: out
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| out.display().to_string()),
                };
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }
}

/// The name of a file without its folders.
fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Reads a paste layer the way the command line does: with the size limit (refused in the language of the person), the notices of the reader counted, and the pads turned into openings.
pub fn read_paste_layer(path: &Path, lang: Lang) -> Result<Layer, String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let max_bytes = eh3d_parser_gerber::InputLimits::default().max_bytes;
    let bytes = read_layer_file(&path.to_string_lossy(), max_bytes, lang.code())
        .map_err(|e| format!("{name}: {e}"))?;
    paste_from_bytes(path, &name, bytes)
}

/// The paste layer drawn from the bytes of a file.
pub fn paste_from_bytes(path: &Path, name: &str, bytes: Vec<u8>) -> Result<Layer, String> {
    let name = name.to_string();
    let max_bytes = eh3d_parser_gerber::InputLimits::default().max_bytes;
    let reading = read_layer(&bytes, max_bytes, &[]).map_err(|e| format!("{name}: {e}"))?;
    let (openings, mut notes) = from_gerber_with(
        &reading.file,
        eh3d_gerberviewer::scene::DEFAULT_ARC_TOLERANCE_MM,
    );
    // what the reader lost is lost from the stencil too
    for _ in 0..reading.errors {
        notes.push(String::from("reader error"));
    }
    if openings.is_empty() {
        return Err(format!("{name}: has no pads to turn into openings"));
    }
    Ok(Layer {
        path: path.to_path_buf(),
        name,
        bytes,
        openings,
        notes,
        job: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_PADS: &str =
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\n%ADD11R,2.0X0.5*%\nD10*\nX20000Y20000D03*\nD11*\nX80000Y20000D03*\nM02*\n";
    const WITH_POLYGON: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\n%ADD11P,0.8X6*%\nD10*\nX20000Y20000D03*\nD11*\nX40000Y20000D03*\nM02*\n";
    const SMALL_PAD: &str =
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,0.3X0.3*%\nD10*\nX20000Y20000D03*\nM02*\n";

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "eh3d-stencilcam-window-{}-{tag}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn layer_in(dir: &Path, name: &str, text: &str) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn the_parameters_start_with_the_profile_of_mylar() {
        let w = Window::new(Lang::En);
        let (p, feed, smax) = w.params().unwrap();
        assert_eq!(p, Profile::of(Material::Mylar));
        assert_eq!((feed, smax), (600.0, 1000));
        assert_eq!(w.fields[0].key, "thickness_mm");
        assert_eq!(w.fields[0].text, "0.12");
    }

    #[test]
    fn choosing_a_material_changes_the_four_values_and_keeps_the_feed() {
        let mut w = Window::new(Lang::En);
        w.set_field(4, "450".into());
        w.set_material(Material::Kapton);
        assert_eq!(w.fields[0].text, "0.075");
        assert_eq!(w.fields[1].text, "8");
        assert_eq!(w.fields[4].text, "450");
        assert_eq!(w.params().unwrap().0, Profile::of(Material::Kapton));
    }

    #[test]
    fn a_value_that_is_not_a_number_names_its_parameter() {
        let mut w = Window::new(Lang::En);
        w.set_field(3, "abc".into());
        match w.params() {
            Err(Status::BadValue { label, .. }) => assert_eq!(label, Msg::Kerf),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_value_outside_the_limits_is_refused_with_its_reason() {
        let mut w = Window::new(Lang::En);
        w.set_field(0, "5".into());
        assert!(matches!(w.params(), Err(Status::Failed(_))));
        w.set_field(0, "0.12".into());
        w.set_field(5, "0".into());
        assert!(matches!(w.params(), Err(Status::BadValue { .. })));
    }

    #[test]
    fn opening_a_layer_makes_the_openings_and_the_paths() {
        let dir = scratch("open");
        let layer = layer_in(&dir, "paste.gtp", TWO_PADS);
        let mut w = Window::new(Lang::En);
        assert!(w.open(&layer));
        assert_eq!(w.openings.len(), 2);
        assert_eq!(w.cuts.len(), 2);
        assert!(w.weak.is_empty());
        assert_eq!(
            w.status,
            Status::Computed {
                openings: 2,
                weak: 0
            }
        );
        w.fit(800.0, 600.0);
        assert!(w.viewport.px_per_mm > 1.0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sample_of_the_repository_opens() {
        let mut w = Window::new(Lang::En);
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/paste.gtp");
        assert!(w.open(&sample));
        assert_eq!(w.openings.len(), 4);
        assert_eq!(w.cuts.len(), 4);
    }

    #[test]
    fn a_file_that_has_no_pads_is_refused_and_the_old_layer_stays() {
        let dir = scratch("nopads");
        let good = layer_in(&dir, "paste.gtp", TWO_PADS);
        let bad = layer_in(&dir, "notes.gtp", "this is not a Gerber file\n");
        let mut w = Window::new(Lang::En);
        assert!(w.open(&good));
        assert!(!w.open(&bad));
        assert!(matches!(w.status, Status::Failed(_)));
        assert!(w.status.is_problem());
        assert_eq!(w.layer.as_ref().unwrap().name, "paste.gtp");
        assert_eq!(w.openings.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_file_is_an_error_and_not_a_panic() {
        let mut w = Window::new(Lang::En);
        assert!(!w.open(Path::new("/this/file/does/not/exist.gtp")));
        assert!(matches!(w.status, Status::Failed(_)));
    }

    #[test]
    fn mirroring_flips_the_x_of_the_pads_and_of_the_cuts() {
        let dir = scratch("mirror");
        let layer = layer_in(&dir, "paste.gtp", TWO_PADS);
        let mut w = Window::new(Lang::En);
        assert!(w.open(&layer));
        let before: Vec<f64> = w.openings.iter().map(|o| o.x).collect();
        let cut_before = w.cuts[0][0].0;
        w.set_mirror(true);
        let after: Vec<f64> = w.openings.iter().map(|o| o.x).collect();
        assert!(after[0] > before[0] && after[1] < before[1]);
        assert!((w.cuts[0][0].0 - cut_before).abs() > 1.0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_opening_that_will_not_release_the_paste_blocks_the_export_unless_forced() {
        let dir = scratch("weak");
        let layer = layer_in(&dir, "paste.gtp", SMALL_PAD);
        let out = dir.join("out.nc");
        let mut w = Window::new(Lang::En);
        w.set_field(3, "0".into());
        w.set_field(0, "0.2".into());
        assert!(w.open(&layer));
        assert_eq!(w.weak, [0]);
        assert_eq!(w.status, Status::Weak(1));
        assert!(!w.export(&out));
        assert!(!out.exists());
        w.set_force(true);
        assert!(w.status.text(Lang::En).contains("1 openings"));
        assert!(w.export(&out));
        assert!(out.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_layer_with_parts_that_are_not_openings_is_incomplete_unless_forced() {
        let dir = scratch("incomplete");
        let layer = layer_in(&dir, "paste.gtp", WITH_POLYGON);
        let out = dir.join("out.nc");
        let mut w = Window::new(Lang::En);
        w.set_field(0, "0.1".into());
        assert!(w.open(&layer));
        assert!(matches!(w.status, Status::Incomplete(n) if n > 0));
        assert!(!w.export(&out));
        assert!(!out.exists());
        w.set_force(true);
        assert!(w.export(&out));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_extension_chooses_hpgl_or_laser_gcode_whatever_its_case() {
        let dir = scratch("formats");
        let layer = layer_in(&dir, "paste.gtp", TWO_PADS);
        let mut w = Window::new(Lang::En);
        w.set_field(0, "0.1".into());
        assert!(w.open(&layer));
        for (name, starts) in [
            ("a.PLT", "IN;SP1;"),
            ("a.plt", "IN;SP1;"),
            ("a.nc", "; generator: eh3d-stencilcam "),
            ("noext", "; generator: eh3d-stencilcam "),
        ] {
            let out = dir.join(name);
            assert!(w.export(&out), "{name}");
            let text = fs::read_to_string(&out).unwrap();
            assert!(text.starts_with(starts), "{name}");
            if starts.starts_with(';') {
                assert!(text.contains("; source: paste.gtp sha256:"), "{name}");
                assert!(text.contains("\nG21\nG90\n"), "{name}");
            }
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_export_that_cannot_be_written_says_why_and_leaves_no_temporary_file() {
        let dir = scratch("failexport");
        let layer = layer_in(&dir, "paste.gtp", TWO_PADS);
        let mut w = Window::new(Lang::En);
        w.set_field(0, "0.1".into());
        assert!(w.open(&layer));
        // an old program is replaced whole, with nothing left beside it
        let out = dir.join("out.nc");
        fs::write(
            &out,
            "an old program
",
        )
        .unwrap();
        assert!(w.export(&out));
        assert!(fs::read_to_string(&out)
            .unwrap()
            .starts_with("; generator: eh3d-stencilcam "));
        assert!(!dir.join("out.nc.tmp").exists());
        // a folder cannot be replaced by a file: the folder stays and the reason has its code
        let folder = dir.join("folder.nc");
        fs::create_dir_all(&folder).unwrap();
        assert!(!w.export(&folder));
        assert!(matches!(&w.status, Status::Failed(e) if e.starts_with("file_unwritable: ")));
        assert!(folder.is_dir());
        assert!(!dir.join("folder.nc.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_output_may_not_be_the_layer_itself() {
        let dir = scratch("same");
        let layer = layer_in(&dir, "paste.gtp", TWO_PADS);
        let mut w = Window::new(Lang::En);
        w.set_field(0, "0.1".into());
        assert!(w.open(&layer));
        assert!(!w.export(&layer));
        assert_eq!(w.status, Status::SameFile);
        assert_eq!(fs::read_to_string(&layer).unwrap(), TWO_PADS);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn exporting_without_a_layer_says_so() {
        let mut w = Window::new(Lang::En);
        assert!(!w.export(Path::new("nothing.nc")));
        assert_eq!(w.status, Status::NoLayer);
        assert!(!w.update());
    }

    #[test]
    fn the_window_shows_the_ratios_of_each_opening() {
        let dir = scratch("ratios");
        let layer = layer_in(&dir, "paste.gtp", TWO_PADS);
        let mut w = Window::new(Lang::En);
        assert!(
            w.ratio_texts(5).is_empty(),
            "nothing before a layer is open"
        );
        assert_eq!(w.smallest_ratios(), None);
        w.set_field(0, "0.1".into());
        assert!(w.open(&layer));
        assert_eq!(w.ratios.len(), 2);
        // the pads shrunk by 10 % on a sheet of 0.1 mm: both have aspect ratio 4.5, the first has the smaller area ratio
        let (aspect, area) = w.smallest_ratios().unwrap();
        assert!(
            (aspect - 4.5).abs() < 1e-9 && (area - 1.5).abs() < 1e-9,
            "{aspect} {area}"
        );
        let texts = w.ratio_texts(5);
        assert_eq!(texts.len(), 3, "{texts:?}");
        assert_eq!(
            texts[0],
            "Smallest aspect ratio 4.50, smallest area ratio 1.50"
        );
        assert_eq!(texts[1], "Opening 1: aspect 4.50, area 1.50");
        assert_eq!(texts[2], "Opening 2: aspect 4.50, area 1.80");
        assert_eq!(w.ratio_texts(1).len(), 2, "only the weakest one is listed");
        assert!(w.ratio_texts(0).len() == 1, "the summary alone");
        assert_eq!(w.weakest(9).len(), 2);
        // a thicker sheet moves the numbers and the language moves the words
        w.set_field(0, "0.3".into());
        let (aspect, _) = w.smallest_ratios().unwrap();
        assert!((aspect - 1.5).abs() < 1e-9, "{aspect}");
        w.lang = Lang::Pt;
        assert!(
            w.ratio_texts(1)[0].starts_with("Rácio de aspeto mínimo 1.50"),
            "{:?}",
            w.ratio_texts(1)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sentences_of_the_status_are_in_the_language_chosen() {
        let st = Status::Computed {
            openings: 12,
            weak: 3,
        };
        assert_eq!(
            st.text(Lang::En),
            "12 openings, 3 will not release the paste"
        );
        assert_eq!(st.text(Lang::Es), "12 aberturas, 3 no soltarán la pasta");
        assert_eq!(Status::Ready.text(Lang::De), "Bereit");
        assert!(!st.is_problem());
        assert!(Status::Weak(1).is_problem());
    }
}
