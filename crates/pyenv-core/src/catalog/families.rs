// ./crates/pyenv-core/src/catalog/families.rs
//! Runtime family classification and stable display metadata for catalog grouping.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VersionFamily {
    CPython,
    PyPy,
    Miniforge,
    Mambaforge,
    Miniconda,
    Anaconda,
    GraalPy,
    Stackless,
    Pyston,
    MicroPython,
    Jython,
    IronPython,
    ActivePython,
    Cinder,
    Nogil,
    Other(String),
}

impl VersionFamily {
    pub(crate) fn classify(name: &str) -> Self {
        let lowered = name.to_ascii_lowercase();
        if lowered.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            Self::CPython
        } else if lowered.starts_with("pypy") {
            Self::PyPy
        } else if lowered.starts_with("miniforge") {
            Self::Miniforge
        } else if lowered.starts_with("mambaforge") {
            Self::Mambaforge
        } else if lowered.starts_with("miniconda") {
            Self::Miniconda
        } else if lowered.starts_with("anaconda") {
            Self::Anaconda
        } else if lowered.starts_with("graalpy") || lowered.starts_with("graalpython") {
            Self::GraalPy
        } else if lowered.starts_with("stackless") {
            Self::Stackless
        } else if lowered.starts_with("pyston") {
            Self::Pyston
        } else if lowered.starts_with("micropython") {
            Self::MicroPython
        } else if lowered.starts_with("jython") {
            Self::Jython
        } else if lowered.starts_with("ironpython") {
            Self::IronPython
        } else if lowered.starts_with("activepython") {
            Self::ActivePython
        } else if lowered.starts_with("cinder") {
            Self::Cinder
        } else if lowered.starts_with("nogil") {
            Self::Nogil
        } else {
            let family = lowered
                .split(['-', '.'])
                .next()
                .filter(|segment| !segment.is_empty())
                .unwrap_or("other");
            Self::Other(title_case(family))
        }
    }

    pub(crate) fn label(&self) -> String {
        match self {
            Self::CPython => "CPython".to_string(),
            Self::PyPy => "PyPy".to_string(),
            Self::Miniforge => "Miniforge".to_string(),
            Self::Mambaforge => "Mambaforge".to_string(),
            Self::Miniconda => "Miniconda".to_string(),
            Self::Anaconda => "Anaconda".to_string(),
            Self::GraalPy => "GraalPy".to_string(),
            Self::Stackless => "Stackless".to_string(),
            Self::Pyston => "Pyston".to_string(),
            Self::MicroPython => "MicroPython".to_string(),
            Self::Jython => "Jython".to_string(),
            Self::IronPython => "IronPython".to_string(),
            Self::ActivePython => "ActivePython".to_string(),
            Self::Cinder => "Cinder".to_string(),
            Self::Nogil => "Nogil".to_string(),
            Self::Other(label) => label.clone(),
        }
    }

    pub(crate) fn slug(&self) -> String {
        match self {
            Self::CPython => "cpython".to_string(),
            Self::PyPy => "pypy".to_string(),
            Self::Miniforge => "miniforge".to_string(),
            Self::Mambaforge => "mambaforge".to_string(),
            Self::Miniconda => "miniconda".to_string(),
            Self::Anaconda => "anaconda".to_string(),
            Self::GraalPy => "graalpy".to_string(),
            Self::Stackless => "stackless".to_string(),
            Self::Pyston => "pyston".to_string(),
            Self::MicroPython => "micropython".to_string(),
            Self::Jython => "jython".to_string(),
            Self::IronPython => "ironpython".to_string(),
            Self::ActivePython => "activepython".to_string(),
            Self::Cinder => "cinder".to_string(),
            Self::Nogil => "nogil".to_string(),
            Self::Other(label) => label.to_ascii_lowercase().replace(' ', "-"),
        }
    }

    pub(crate) fn rank(&self) -> usize {
        match self {
            Self::CPython => 0,
            Self::PyPy => 1,
            Self::Miniforge => 2,
            Self::Mambaforge => 3,
            Self::Miniconda => 4,
            Self::Anaconda => 5,
            Self::GraalPy => 6,
            Self::Stackless => 7,
            Self::Pyston => 8,
            Self::MicroPython => 9,
            Self::Jython => 10,
            Self::IronPython => 11,
            Self::ActivePython => 12,
            Self::Cinder => 13,
            Self::Nogil => 14,
            Self::Other(_) => 15,
        }
    }
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}
