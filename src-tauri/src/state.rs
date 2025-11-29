use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::{
    db::{AppSettings, Database},
    error::{AppError, Result},
    logging::OpsLogger,
    paths::AppPaths,
};

#[derive(Default)]
pub struct SharedState {
    inner: RwLock<StateInner>,
}

#[derive(Default)]
struct StateInner {
    paths: Option<AppPaths>,
    db: Option<Arc<Database>>,
    logger: Option<Arc<OpsLogger>>,
}

impl SharedState {
    pub fn initialize(&self, root: PathBuf, locale: Option<String>) -> Result<AppSettings> {
        let paths = AppPaths::new(root);
        paths.ensure_layout()?;

        let db = Arc::new(Database::open(&paths)?);
        db.update_root_path(paths.root())?;
        if let Some(locale) = locale {
            db.update_locale(&locale)?;
        }
        let settings = db.get_settings()?;

        let logger = Arc::new(OpsLogger::new(paths.ops_log_path())?);
        logger.log_line("init_root", format!("root={}", paths.root().display()))?;

        {
            let mut inner = self.inner.write().expect("state lock poisoned");
            inner.paths = Some(paths);
            inner.db = Some(db.clone());
            inner.logger = Some(logger);
        }

        Ok(settings)
    }

    pub fn get_settings(&self) -> Result<Option<AppSettings>> {
        if let Some(db) = self.db_opt() {
            Ok(Some(db.get_settings()?))
        } else {
            Ok(None)
        }
    }

    pub fn paths(&self) -> Result<AppPaths> {
        self.inner
            .read()
            .expect("state lock poisoned")
            .paths
            .clone()
            .ok_or(AppError::RootNotInitialized)
    }

    pub fn db(&self) -> Result<Arc<Database>> {
        self.db_opt().ok_or(AppError::RootNotInitialized)
    }

    fn db_opt(&self) -> Option<Arc<Database>> {
        self.inner.read().expect("state lock poisoned").db.clone()
    }

    pub fn logger(&self) -> Option<Arc<OpsLogger>> {
        self.inner
            .read()
            .expect("state lock poisoned")
            .logger
            .clone()
    }
}
