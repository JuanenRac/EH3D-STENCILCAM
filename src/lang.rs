//! The words of the window in the eight languages of the repository (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal).

/// The languages of the window, in the order the lists of words below follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    De,
    Fr,
    It,
    Ja,
    Zh,
    Pt,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The two letters of the language (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Zh => "zh",
            Lang::Pt => "pt",
        }
    }

    /// The name of the language in the language itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::Zh => "中文",
            Lang::Pt => "Português",
        }
    }

    /// The language of a code such as `es`, `es_ES.UTF-8` or `pt-PT`.
    pub fn from_code(code: &str) -> Option<Lang> {
        let two: String = code
            .chars()
            .take(2)
            .collect::<String>()
            .to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == two)
    }

    /// The language of the person from the variables of the system; English when it is none of the eight.
    pub fn from_env() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if !v.is_empty() {
                    return Lang::from_code(&v).unwrap_or(Lang::En);
                }
            }
        }
        Lang::En
    }
}

/// Puts the values in the places `{0}`, `{1}`... of a text.
pub fn fill(template: &str, values: &[&str]) -> String {
    let mut out = template.to_string();
    for (k, v) in values.iter().enumerate() {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Every word and sentence of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    Open,
    Export,
    Language,
    Fit,
    DropHint,
    Parameters,
    Material,
    MaterialMylar,
    MaterialKapton,
    MaterialSteel,
    Thickness,
    Shrink,
    CornerRadius,
    Kerf,
    Feed,
    LaserPower,
    Mirror,
    Force,
    ZoomHint,
    LegendPads,
    LegendCut,
    LegendWeak,
    OpenTitle,
    ExportTitle,
    Folder,
    Up,
    AllFiles,
    Cancel,
    EmptyFolder,
    FileName,
    Ready,
    Computed,
    Exported,
    NoLayer,
    Failed,
    Incomplete,
    Weak,
    BadValue,
    SameFile,
    RatiosTitle,
    RatiosSmallest,
    RatiosLine,
    SheetLine,
    SaveProject,
    JobLayer,
    SheetTable,
    Sheet,
    NoSheet,
    ProjectOpened,
    ProjectSaved,
    ProjectStale,
    NoProject,
    SheetsRead,
    LeadsByShape,
    LeadIn,
    LeadOut,
    LeadInRound,
    LeadOutRound,
    LeadInRect,
    LeadOutRect,
    LaserWait,
    Passes,
    OriginX,
    OriginY,
    BedWidth,
    BedHeight,
    OutsideBed,
    Duplicates,
}

impl Msg {
    pub const ALL: [Msg; 68] = [
        Msg::Open,
        Msg::Export,
        Msg::Language,
        Msg::Fit,
        Msg::DropHint,
        Msg::Parameters,
        Msg::Material,
        Msg::MaterialMylar,
        Msg::MaterialKapton,
        Msg::MaterialSteel,
        Msg::Thickness,
        Msg::Shrink,
        Msg::CornerRadius,
        Msg::Kerf,
        Msg::Feed,
        Msg::LaserPower,
        Msg::Mirror,
        Msg::Force,
        Msg::ZoomHint,
        Msg::LegendPads,
        Msg::LegendCut,
        Msg::LegendWeak,
        Msg::OpenTitle,
        Msg::ExportTitle,
        Msg::Folder,
        Msg::Up,
        Msg::AllFiles,
        Msg::Cancel,
        Msg::EmptyFolder,
        Msg::FileName,
        Msg::Ready,
        Msg::Computed,
        Msg::Exported,
        Msg::NoLayer,
        Msg::Failed,
        Msg::Incomplete,
        Msg::Weak,
        Msg::BadValue,
        Msg::SameFile,
        Msg::RatiosTitle,
        Msg::RatiosSmallest,
        Msg::RatiosLine,
        Msg::SheetLine,
        Msg::SaveProject,
        Msg::JobLayer,
        Msg::SheetTable,
        Msg::Sheet,
        Msg::NoSheet,
        Msg::ProjectOpened,
        Msg::ProjectSaved,
        Msg::ProjectStale,
        Msg::NoProject,
        Msg::SheetsRead,
        Msg::LeadsByShape,
        Msg::LeadIn,
        Msg::LeadOut,
        Msg::LeadInRound,
        Msg::LeadOutRound,
        Msg::LeadInRect,
        Msg::LeadOutRect,
        Msg::LaserWait,
        Msg::Passes,
        Msg::OriginX,
        Msg::OriginY,
        Msg::BedWidth,
        Msg::BedHeight,
        Msg::OutsideBed,
        Msg::Duplicates,
    ];

    /// The words in the order English, Spanish, German, French, Italian, Japanese, Chinese, Portuguese.
    fn words(self) -> [&'static str; 8] {
        match self {
            Msg::Open => [
                "Open paste layer...",
                "Abrir capa de pasta...",
                "Pastenebene öffnen...",
                "Ouvrir la couche de pâte...",
                "Apri livello pasta...",
                "ペーストレイヤーを開く...",
                "打开锡膏层...",
                "Abrir camada de pasta...",
            ],
            Msg::Export => [
                "Export...",
                "Exportar...",
                "Exportieren...",
                "Exporter...",
                "Esporta...",
                "書き出し...",
                "导出...",
                "Exportar...",
            ],
            Msg::Language => [
                "Language",
                "Idioma",
                "Sprache",
                "Langue",
                "Lingua",
                "言語",
                "语言",
                "Idioma",
            ],
            Msg::Fit => [
                "Fit",
                "Ajustar",
                "Einpassen",
                "Ajuster",
                "Adatta",
                "全体表示",
                "适应窗口",
                "Ajustar",
            ],
            Msg::DropHint => [
                "Drop a paste layer (Gerber) here or press Open",
                "Suelte aquí una capa de pasta (Gerber) o pulse Abrir",
                "Eine Pastenebene (Gerber) hierher ziehen oder auf Öffnen klicken",
                "Déposez une couche de pâte (Gerber) ici ou appuyez sur Ouvrir",
                "Trascina qui un livello pasta (Gerber) o premi Apri",
                "ペーストレイヤー（Gerber）をここにドロップするか「開く」を押してください",
                "将锡膏层 (Gerber) 拖到此处，或点击“打开”",
                "Largue aqui uma camada de pasta (Gerber) ou prima Abrir",
            ],
            Msg::Parameters => [
                "Parameters",
                "Parámetros",
                "Parameter",
                "Paramètres",
                "Parametri",
                "パラメーター",
                "参数",
                "Parâmetros",
            ],
            Msg::Material => [
                "Material",
                "Material",
                "Material",
                "Matériau",
                "Materiale",
                "材料",
                "材料",
                "Material",
            ],
            Msg::MaterialMylar => [
                "Mylar",
                "Mylar",
                "Mylar",
                "Mylar",
                "Mylar",
                "マイラー",
                "聚酯膜 (Mylar)",
                "Mylar",
            ],
            Msg::MaterialKapton => [
                "Kapton",
                "Kapton",
                "Kapton",
                "Kapton",
                "Kapton",
                "カプトン",
                "聚酰亚胺 (Kapton)",
                "Kapton",
            ],
            Msg::MaterialSteel => [
                "Steel",
                "Acero",
                "Stahl",
                "Acier",
                "Acciaio",
                "ステンレス鋼",
                "不锈钢",
                "Aço",
            ],
            Msg::Thickness => [
                "Sheet thickness (mm)",
                "Espesor de la lámina (mm)",
                "Foliendicke (mm)",
                "Épaisseur de la feuille (mm)",
                "Spessore del foglio (mm)",
                "シート厚 (mm)",
                "薄片厚度 (mm)",
                "Espessura da folha (mm)",
            ],
            Msg::Shrink => [
                "Shrink of each side (%)",
                "Reducción de cada lado (%)",
                "Verkleinerung je Seite (%)",
                "Réduction de chaque côté (%)",
                "Riduzione di ogni lato (%)",
                "各辺の縮小率 (%)",
                "每边缩小 (%)",
                "Redução de cada lado (%)",
            ],
            Msg::CornerRadius => [
                "Corner radius (mm)",
                "Radio de las esquinas (mm)",
                "Eckenradius (mm)",
                "Rayon des coins (mm)",
                "Raggio degli angoli (mm)",
                "角の半径 (mm)",
                "圆角半径 (mm)",
                "Raio dos cantos (mm)",
            ],
            Msg::Kerf => [
                "Laser kerf (mm)",
                "Anchura de corte del láser (mm)",
                "Schnittbreite des Lasers (mm)",
                "Largeur de coupe du laser (mm)",
                "Larghezza di taglio del laser (mm)",
                "レーザーの切り幅 (mm)",
                "激光切缝宽度 (mm)",
                "Largura de corte do laser (mm)",
            ],
            Msg::Feed => [
                "Feed (mm/min)",
                "Avance (mm/min)",
                "Vorschub (mm/min)",
                "Avance (mm/min)",
                "Avanzamento (mm/min)",
                "送り速度 (mm/min)",
                "进给 (mm/min)",
                "Avanço (mm/min)",
            ],
            Msg::LaserPower => [
                "Laser power (S value)",
                "Potencia del láser (valor S)",
                "Laserleistung (S-Wert)",
                "Puissance du laser (valeur S)",
                "Potenza del laser (valore S)",
                "レーザー出力 (S 値)",
                "激光功率 (S 值)",
                "Potência do laser (valor S)",
            ],
            Msg::Mirror => [
                "Mirror (stencil of the bottom side)",
                "Espejo (plantilla de la cara inferior)",
                "Spiegeln (Schablone der Unterseite)",
                "Miroir (pochoir de la face inférieure)",
                "Specchio (stencil del lato inferiore)",
                "ミラー（下面のステンシル）",
                "镜像（底面钢网）",
                "Espelho (stencil da face inferior)",
            ],
            Msg::Force => [
                "Write even if there are problems",
                "Escribir aunque haya problemas",
                "Auch bei Problemen schreiben",
                "Écrire même en cas de problème",
                "Scrivi anche in caso di problemi",
                "問題があっても書き出す",
                "即使有问题也输出",
                "Escrever mesmo que haja problemas",
            ],
            Msg::ZoomHint => [
                "Wheel: zoom - drag: move",
                "Rueda: zoom - arrastrar: mover",
                "Rad: Zoom - Ziehen: verschieben",
                "Molette : zoom - glisser : déplacer",
                "Rotella: zoom - trascina: sposta",
                "ホイール: 拡大縮小 - ドラッグ: 移動",
                "滚轮：缩放 - 拖动：移动",
                "Roda: zoom - arrastar: mover",
            ],
            Msg::LegendPads => [
                "Pads of the paste layer",
                "Pads de la capa de pasta",
                "Pads der Pastenebene",
                "Pastilles de la couche de pâte",
                "Pad del livello pasta",
                "ペースト層のパッド",
                "锡膏层焊盘",
                "Pads da camada de pasta",
            ],
            Msg::LegendCut => [
                "Cutting path",
                "Trayectoria de corte",
                "Schneidbahn",
                "Trajet de coupe",
                "Percorso di taglio",
                "切断パス",
                "切割路径",
                "Trajetória de corte",
            ],
            Msg::LegendWeak => [
                "Will not release the paste",
                "No soltará la pasta",
                "Gibt die Paste nicht frei",
                "Ne libérera pas la pâte",
                "Non rilascerà la pasta",
                "ペーストが抜けません",
                "锡膏无法脱出",
                "Não libertará a pasta",
            ],
            Msg::OpenTitle => [
                "Open a paste layer (Gerber)",
                "Abrir una capa de pasta (Gerber)",
                "Pastenebene öffnen (Gerber)",
                "Ouvrir une couche de pâte (Gerber)",
                "Apri un livello pasta (Gerber)",
                "ペーストレイヤー（Gerber）を開く",
                "打开锡膏层 (Gerber)",
                "Abrir uma camada de pasta (Gerber)",
            ],
            Msg::ExportTitle => [
                "Export the stencil (.nc laser G-code, .plt HP-GL)",
                "Exportar la plantilla (.nc G-code de láser, .plt HP-GL)",
                "Schablone exportieren (.nc Laser-G-Code, .plt HP-GL)",
                "Exporter le pochoir (.nc G-code laser, .plt HP-GL)",
                "Esporta lo stencil (.nc G-code laser, .plt HP-GL)",
                "ステンシルを書き出す (.nc レーザー G コード, .plt HP-GL)",
                "导出钢网 (.nc 激光 G 代码, .plt HP-GL)",
                "Exportar o stencil (.nc G-code de laser, .plt HP-GL)",
            ],
            Msg::Folder => [
                "Folder",
                "Carpeta",
                "Ordner",
                "Dossier",
                "Cartella",
                "フォルダー",
                "文件夹",
                "Pasta",
            ],
            Msg::Up => [
                "Up",
                "Subir",
                "Nach oben",
                "Dossier parent",
                "Su",
                "上へ",
                "上一级",
                "Subir",
            ],
            Msg::AllFiles => [
                "All files",
                "Todos los ficheros",
                "Alle Dateien",
                "Tous les fichiers",
                "Tutti i file",
                "すべてのファイル",
                "所有文件",
                "Todos os ficheiros",
            ],
            Msg::Cancel => [
                "Cancel",
                "Cancelar",
                "Abbrechen",
                "Annuler",
                "Annulla",
                "キャンセル",
                "取消",
                "Cancelar",
            ],
            Msg::EmptyFolder => [
                "(nothing to open here)",
                "(nada que abrir aquí)",
                "(hier gibt es nichts zu öffnen)",
                "(rien à ouvrir ici)",
                "(niente da aprire qui)",
                "(ここに開けるものはありません)",
                "(这里没有可打开的内容)",
                "(nada para abrir aqui)",
            ],
            Msg::FileName => [
                "File name",
                "Nombre del fichero",
                "Dateiname",
                "Nom du fichier",
                "Nome del file",
                "ファイル名",
                "文件名",
                "Nome do ficheiro",
            ],
            Msg::Ready => [
                "Ready",
                "Listo",
                "Bereit",
                "Prêt",
                "Pronto",
                "準備完了",
                "就绪",
                "Pronto",
            ],
            Msg::Computed => [
                "{0} openings, {1} will not release the paste",
                "{0} aberturas, {1} no soltarán la pasta",
                "{0} Öffnungen, bei {1} bleibt die Paste hängen",
                "{0} ouvertures, {1} ne libéreront pas la pâte",
                "{0} aperture, {1} non rilasceranno la pasta",
                "開口 {0} 個、そのうち {1} 個はペーストが抜けません",
                "{0} 个开口，其中 {1} 个无法让锡膏脱出",
                "{0} aberturas, {1} não libertarão a pasta",
            ],
            Msg::Exported => [
                "Written to {0}",
                "Escrito en {0}",
                "Geschrieben nach {0}",
                "Écrit dans {0}",
                "Scritto in {0}",
                "{0} に書き出しました",
                "已写入 {0}",
                "Escrito em {0}",
            ],
            Msg::NoLayer => [
                "Open a paste layer first",
                "Abra primero una capa de pasta",
                "Zuerst eine Pastenebene öffnen",
                "Ouvrez d'abord une couche de pâte",
                "Apri prima un livello pasta",
                "先にペーストレイヤーを開いてください",
                "请先打开锡膏层",
                "Abra primeiro uma camada de pasta",
            ],
            Msg::Failed => [
                "Error: {0}",
                "Error: {0}",
                "Fehler: {0}",
                "Erreur : {0}",
                "Errore: {0}",
                "エラー: {0}",
                "错误：{0}",
                "Erro: {0}",
            ],
            Msg::Incomplete => [
                "{0} parts of the layer cannot become openings, so the stencil would be incomplete; tick the box to write anyway",
                "{0} partes de la capa no pueden ser aberturas, así que la plantilla quedaría incompleta; marque la casilla para escribir de todos modos",
                "{0} Teile der Ebene können keine Öffnungen werden, die Schablone wäre unvollständig; zum Schreiben trotzdem das Kästchen anhaken",
                "{0} éléments de la couche ne peuvent pas devenir des ouvertures, le pochoir serait incomplet ; cochez la case pour écrire quand même",
                "{0} parti del livello non possono diventare aperture, quindi lo stencil sarebbe incompleto; spunta la casella per scrivere comunque",
                "レイヤーの {0} か所は開口にできず、ステンシルが不完全になります。それでも書き出すにはチェックを入れてください",
                "该层有 {0} 处无法变成开口，钢网会不完整；勾选此框可强制输出",
                "{0} partes da camada não podem ser aberturas, por isso o stencil ficaria incompleto; assinale a caixa para escrever na mesma",
            ],
            Msg::Weak => [
                "{0} openings will not release the paste; tick the box to write anyway",
                "{0} aberturas no soltarán la pasta; marque la casilla para escribir de todos modos",
                "Bei {0} Öffnungen bleibt die Paste hängen; zum Schreiben trotzdem das Kästchen anhaken",
                "{0} ouvertures ne libéreront pas la pâte ; cochez la case pour écrire quand même",
                "{0} aperture non rilasceranno la pasta; spunta la casella per scrivere comunque",
                "{0} 個の開口ではペーストが抜けません。それでも書き出すにはチェックを入れてください",
                "有 {0} 个开口无法让锡膏脱出；勾选此框可强制输出",
                "{0} aberturas não libertarão a pasta; assinale a caixa para escrever na mesma",
            ],
            Msg::BadValue => [
                "{0}: {1}",
                "{0}: {1}",
                "{0}: {1}",
                "{0} : {1}",
                "{0}: {1}",
                "{0}: {1}",
                "{0}：{1}",
                "{0}: {1}",
            ],
            Msg::SameFile => [
                "The output file is the layer itself; choose another name",
                "El fichero de salida es la propia capa; elija otro nombre",
                "Die Ausgabedatei ist die Ebene selbst; einen anderen Namen wählen",
                "Le fichier de sortie est la couche elle-même ; choisissez un autre nom",
                "Il file di uscita è il livello stesso; scegli un altro nome",
                "出力ファイルがレイヤーそのものです。別の名前を選んでください",
                "输出文件就是该层本身；请选择其他名称",
                "O ficheiro de saída é a própria camada; escolha outro nome",
            ],
            Msg::RatiosTitle => [
                "Ratios",
                "Relaciones",
                "Verhältnisse",
                "Rapports",
                "Rapporti",
                "比率",
                "比值",
                "Rácios",
            ],
            Msg::RatiosSmallest => [
                "Smallest aspect ratio {0}, smallest area ratio {1}",
                "Relación de aspecto mínima {0}, relación de área mínima {1}",
                "Kleinstes Aspektverhältnis {0}, kleinstes Flächenverhältnis {1}",
                "Rapport d'aspect minimal {0}, rapport de surface minimal {1}",
                "Rapporto d'aspetto minimo {0}, rapporto d'area minimo {1}",
                "最小アスペクト比 {0}、最小面積比 {1}",
                "最小纵横比 {0}，最小面积比 {1}",
                "Rácio de aspeto mínimo {0}, rácio de área mínimo {1}",
            ],
            Msg::RatiosLine => [
                "Opening {0}: aspect {1}, area {2}",
                "Abertura {0}: aspecto {1}, área {2}",
                "Öffnung {0}: Aspekt {1}, Fläche {2}",
                "Ouverture {0} : aspect {1}, surface {2}",
                "Apertura {0}: aspetto {1}, area {2}",
                "開口 {0}：アスペクト {1}、面積 {2}",
                "开口 {0}：纵横 {1}，面积 {2}",
                "Abertura {0}: aspeto {1}, área {2}",
            ],
            Msg::SheetLine => [
                "Sheet {0}: thickness {1} mm, reduction {2} mm per side, kerf {3} mm",
                "Lámina {0}: espesor {1} mm, reducción {2} mm por lado, anchura de corte {3} mm",
                "Folie {0}: Dicke {1} mm, Verkleinerung {2} mm je Seite, Schnittbreite {3} mm",
                "Feuille {0} : épaisseur {1} mm, réduction {2} mm par côté, largeur de coupe {3} mm",
                "Foglio {0}: spessore {1} mm, riduzione {2} mm per lato, larghezza di taglio {3} mm",
                "シート {0}：厚さ {1} mm、片側の縮小 {2} mm、切り幅 {3} mm",
                "薄片 {0}：厚度 {1} mm，每边缩小 {2} mm，切缝宽度 {3} mm",
                "Folha {0}: espessura {1} mm, redução {2} mm por lado, largura de corte {3} mm",
            ],
            Msg::SaveProject => [
                "Save to the project",
                "Guardar en el proyecto",
                "Im Projekt speichern",
                "Enregistrer dans le projet",
                "Salva nel progetto",
                "プロジェクトに保存",
                "保存到项目",
                "Guardar no projeto",
            ],
            Msg::JobLayer => [
                "Paste layer of the job",
                "Capa de pasta del trabajo",
                "Pastenebene des Auftrags",
                "Couche de pâte du travail",
                "Livello pasta del lavoro",
                "ジョブのペーストレイヤー",
                "作业的锡膏层",
                "Camada de pasta do trabalho",
            ],
            Msg::SheetTable => [
                "Sheet table...",
                "Tabla de láminas...",
                "Folientabelle...",
                "Table des feuilles...",
                "Tabella dei fogli...",
                "シート表...",
                "薄片表...",
                "Tabela de folhas...",
            ],
            Msg::Sheet => [
                "Sheet of the table",
                "Lámina de la tabla",
                "Folie der Tabelle",
                "Feuille de la table",
                "Foglio della tabella",
                "表のシート",
                "表中的薄片",
                "Folha da tabela",
            ],
            Msg::NoSheet => [
                "Material only",
                "Solo el material",
                "Nur das Material",
                "Matériau seul",
                "Solo il materiale",
                "材料のみ",
                "仅材料",
                "Só o material",
            ],
            Msg::ProjectOpened => [
                "{0}: {1} paste layers of the job",
                "{0}: {1} capas de pasta del trabajo",
                "{0}: {1} Pastenebenen des Auftrags",
                "{0} : {1} couches de pâte du travail",
                "{0}: {1} livelli pasta del lavoro",
                "{0}：ジョブのペーストレイヤー {1} 個",
                "{0}：作业中的 {1} 个锡膏层",
                "{0}: {1} camadas de pasta do trabalho",
            ],
            Msg::ProjectSaved => [
                "{0} saved in the project",
                "{0} guardado en el proyecto",
                "{0} im Projekt gespeichert",
                "{0} enregistré dans le projet",
                "{0} salvato nel progetto",
                "{0} をプロジェクトに保存しました",
                "{0} 已保存到项目",
                "{0} guardado no projeto",
            ],
            Msg::ProjectStale => [
                "{0}: the saved stencils are out of date, the job changed",
                "{0}: las plantillas guardadas están desactualizadas, el trabajo cambió",
                "{0}: die gespeicherten Schablonen sind veraltet, der Auftrag hat sich geändert",
                "{0} : les pochoirs enregistrés sont périmés, le travail a changé",
                "{0}: gli stencil salvati non sono aggiornati, il lavoro è cambiato",
                "{0}：保存したステンシルは古くなっています。ジョブが変更されました",
                "{0}：已保存的钢网已过期，作业已更改",
                "{0}: os estênceis guardados estão desatualizados, o trabalho mudou",
            ],
            Msg::NoProject => [
                "Open a .eh3d project first",
                "Abra primero un proyecto .eh3d",
                "Öffnen Sie zuerst ein .eh3d-Projekt",
                "Ouvrez d'abord un projet .eh3d",
                "Apri prima un progetto .eh3d",
                "先に .eh3d プロジェクトを開いてください",
                "请先打开 .eh3d 项目",
                "Abra primeiro um projeto .eh3d",
            ],
            Msg::SheetsRead => [
                "{0} sheets read from {1}",
                "{0} láminas leídas de {1}",
                "{0} Folien aus {1} gelesen",
                "{0} feuilles lues dans {1}",
                "{0} fogli letti da {1}",
                "{1} から {0} 枚のシートを読み込みました",
                "从 {1} 读取了 {0} 种薄片",
                "{0} folhas lidas de {1}",
            ],
            Msg::LeadsByShape => [
                "Leads by shape (empty: as above)",
                "Entradas y salidas por forma (vacío: como arriba)",
                "An- und Ausläufe je Form (leer: wie oben)",
                "Amorces par forme (vide : comme ci-dessus)",
                "Attacchi per forma (vuoto: come sopra)",
                "形状ごとの導入線・導出線（空欄は上と同じ）",
                "按形状设置引入线和引出线（留空则同上）",
                "Entradas e saídas por forma (vazio: como acima)",
            ],
            Msg::LeadIn => [
                "Lead-in (mm)",
                "Entrada del corte (mm)",
                "Anlauf (mm)",
                "Amorce d'entrée (mm)",
                "Attacco in entrata (mm)",
                "導入線 (mm)",
                "引入线 (mm)",
                "Entrada do corte (mm)",
            ],
            Msg::LeadOut => [
                "Lead-out (mm)",
                "Salida del corte (mm)",
                "Auslauf (mm)",
                "Amorce de sortie (mm)",
                "Attacco in uscita (mm)",
                "導出線 (mm)",
                "引出线 (mm)",
                "Saída do corte (mm)",
            ],
            Msg::LeadInRound => [
                "Lead-in, round openings (mm)",
                "Entrada, aberturas redondas (mm)",
                "Anlauf, runde Öffnungen (mm)",
                "Entrée, ouvertures rondes (mm)",
                "Entrata, aperture tonde (mm)",
                "導入線（丸い開口）(mm)",
                "引入线（圆形开口）(mm)",
                "Entrada, aberturas redondas (mm)",
            ],
            Msg::LeadOutRound => [
                "Lead-out, round openings (mm)",
                "Salida, aberturas redondas (mm)",
                "Auslauf, runde Öffnungen (mm)",
                "Sortie, ouvertures rondes (mm)",
                "Uscita, aperture tonde (mm)",
                "導出線（丸い開口）(mm)",
                "引出线（圆形开口）(mm)",
                "Saída, aberturas redondas (mm)",
            ],
            Msg::LeadInRect => [
                "Lead-in, rectangular openings (mm)",
                "Entrada, aberturas rectangulares (mm)",
                "Anlauf, rechteckige Öffnungen (mm)",
                "Entrée, ouvertures rectangulaires (mm)",
                "Entrata, aperture rettangolari (mm)",
                "導入線（四角い開口）(mm)",
                "引入线（矩形开口）(mm)",
                "Entrada, aberturas retangulares (mm)",
            ],
            Msg::LeadOutRect => [
                "Lead-out, rectangular openings (mm)",
                "Salida, aberturas rectangulares (mm)",
                "Auslauf, rechteckige Öffnungen (mm)",
                "Sortie, ouvertures rectangulaires (mm)",
                "Uscita, aperture rettangolari (mm)",
                "導出線（四角い開口）(mm)",
                "引出线（矩形开口）(mm)",
                "Saída, aberturas retangulares (mm)",
            ],
            Msg::LaserWait => [
                "Wait for the laser (s)",
                "Espera del láser (s)",
                "Wartezeit für den Laser (s)",
                "Attente du laser (s)",
                "Attesa del laser (s)",
                "レーザーの待ち時間 (秒)",
                "激光等待时间 (秒)",
                "Espera do laser (s)",
            ],
            Msg::Passes => [
                "Passes",
                "Pasadas",
                "Durchgänge",
                "Passes",
                "Passate",
                "パス回数",
                "切割次数",
                "Passagens",
            ],
            Msg::OriginX => [
                "Origin X (mm, empty: as drawn)",
                "Origen X (mm, vacío: como está)",
                "Ursprung X (mm, leer: wie gezeichnet)",
                "Origine X (mm, vide : tel que dessiné)",
                "Origine X (mm, vuoto: come disegnato)",
                "原点 X (mm、空欄は図面のまま)",
                "原点 X (mm，留空则按原图)",
                "Origem X (mm, vazio: como está)",
            ],
            Msg::OriginY => [
                "Origin Y (mm)",
                "Origen Y (mm)",
                "Ursprung Y (mm)",
                "Origine Y (mm)",
                "Origine Y (mm)",
                "原点 Y (mm)",
                "原点 Y (mm)",
                "Origem Y (mm)",
            ],
            Msg::BedWidth => [
                "Bed width (mm, empty: no limit)",
                "Ancho de la cama (mm, vacío: sin límite)",
                "Bettbreite (mm, leer: keine Grenze)",
                "Largeur du plateau (mm, vide : sans limite)",
                "Larghezza del piano (mm, vuoto: nessun limite)",
                "ベッドの幅 (mm、空欄は制限なし)",
                "工作台宽度 (mm，留空则不限制)",
                "Largura da mesa (mm, vazio: sem limite)",
            ],
            Msg::BedHeight => [
                "Bed height (mm)",
                "Alto de la cama (mm)",
                "Betttiefe (mm)",
                "Profondeur du plateau (mm)",
                "Profondità del piano (mm)",
                "ベッドの奥行き (mm)",
                "工作台深度 (mm)",
                "Profundidade da mesa (mm)",
            ],
            Msg::OutsideBed => [
                "The point {0},{1} falls outside the bed of {2} x {3} mm (the lead-in and the lead-out count): move the stencil with the origin",
                "El punto {0},{1} queda fuera de la cama de {2} x {3} mm (la entrada y la salida cuentan): mueva la plantilla con el origen",
                "Der Punkt {0},{1} liegt außerhalb des Betts von {2} x {3} mm (An- und Auslauf zählen mit): verschieben Sie die Schablone mit dem Ursprung",
                "Le point {0},{1} sort du plateau de {2} x {3} mm (les amorces comptent) : déplacez le pochoir avec l'origine",
                "Il punto {0},{1} esce dal piano di {2} x {3} mm (gli attacchi contano): sposti la maschera con l'origine",
                "点 {0},{1} が {2} x {3} mm のベッドの外に出ます（導入線と導出線も含みます）。原点でステンシルを移動してください",
                "点 {0},{1} 超出 {2} x {3} mm 的工作台（引入线和引出线计算在内）：请用原点移动钢网",
                "O ponto {0},{1} fica fora da mesa de {2} x {3} mm (a entrada e a saída contam): desloque o estêncil com a origem",
            ],
            Msg::Duplicates => [
                "{0} opening(s) repeat another one and are left out",
                "{0} abertura(s) repiten otra y se dejan fuera",
                "{0} Öffnung(en) wiederholen eine andere und werden weggelassen",
                "{0} ouverture(s) en répètent une autre et sont laissées de côté",
                "{0} apertura/e ripetono un'altra e vengono omesse",
                "{0} 個の開口は別の開口と重複するため除外します",
                "{0} 个开口与其他开口重复，已略去",
                "{0} abertura(s) repetem outra e ficam de fora",
            ],
        }
    }

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.words()[lang as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sentence_has_a_text_in_each_of_the_eight_languages() {
        for m in Msg::ALL {
            for l in Lang::ALL {
                assert!(!m.tr(l).trim().is_empty(), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_list_of_sentences_has_no_repeats() {
        for (i, a) in Msg::ALL.iter().enumerate() {
            assert!(!Msg::ALL[i + 1..].contains(a), "{a:?} is listed twice");
        }
    }

    #[test]
    fn a_template_has_the_same_places_in_every_language() {
        for m in Msg::ALL {
            let places = |l: Lang| {
                (0..4)
                    .filter(|k| m.tr(l).contains(&format!("{{{k}}}")))
                    .count()
            };
            for l in Lang::ALL {
                assert_eq!(places(l), places(Lang::En), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_order_of_the_languages_matches_the_lists() {
        for (k, l) in Lang::ALL.into_iter().enumerate() {
            assert_eq!(l as usize, k);
        }
        assert_eq!(Msg::Cancel.tr(Lang::Es), "Cancelar");
        assert_eq!(Msg::Cancel.tr(Lang::De), "Abbrechen");
        assert_eq!(Msg::Cancel.tr(Lang::Pt), "Cancelar");
    }

    #[test]
    fn a_language_is_found_from_the_code_of_the_system() {
        assert_eq!(Lang::from_code("es_ES.UTF-8"), Some(Lang::Es));
        assert_eq!(Lang::from_code("PT-pt"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_code("xx"), None);
        assert_eq!(Lang::from_code(""), None);
    }

    #[test]
    fn the_places_of_a_template_are_filled() {
        assert_eq!(fill("{1} and {0}", &["a", "b"]), "b and a");
        assert_eq!(fill("none", &["a"]), "none");
    }
}
